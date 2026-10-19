//! Runtime values.
//!
//! Primitives are stored unboxed and keep their type tag, so a value held
//! in an `Object` slot is its own box (`Int` 1 and `Double` 1 stay
//! different). Strings are immutable UTF-16. Everything else is a shared
//! reference.
//!
//! `List`, `Map`, `StringBuilder` follow the B4J wrapper model: a variable
//! holds a *wrapper* (`Rc<RefCell<Option<inner>>>`) that is uninitialised
//! while it holds `None`; `Initialize` stores a fresh inner object in it.
//! Converting a wrapper to `Object` unwraps it (an uninitialised one becomes
//! `Null`), and converting back wraps the inner object in a new wrapper,
//! as B4J does with `getObject` / `ConvertToWrapper`. That is also why
//! printing depends on the static type: a `List` prints as
//! `(ArrayList) [1, 2]`, the same list held in an `Object` as `[1, 2]`.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use ferro_mir::{Program, RecordField, RecordId, Ty};

use crate::number::{java_double_to_string, java_float_to_string};
#[cfg(test)]
use crate::strings::to_rust;
use crate::strings::to_utf16;

/// A Java String. The text is built first, then shared without copying.
pub type Str = Rc<Vec<u16>>;
pub type Shared<T> = Rc<RefCell<T>>;
pub type Wrapper<T> = Rc<RefCell<Option<T>>>;

/// How a list may be changed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListKind {
    /// `java.util.ArrayList`.
    Growable,
    /// `Arrays.asList`: elements can be set, the size cannot change.
    FixedSize,
    /// `SubList`: `Collections.unmodifiableList(parent.subList(a, b))`, a
    /// read-only view of the parent.
    ReadOnly,
}

impl ListKind {
    /// `getClass().getName()`.
    pub fn class_name(self) -> &'static str {
        match self {
            ListKind::Growable => "java.util.ArrayList",
            ListKind::FixedSize => "java.util.Arrays$ArrayList",
            ListKind::ReadOnly => "java.util.Collections$UnmodifiableRandomAccessList",
        }
    }

    /// `getClass().getSimpleName()`, the prefix a `List` prints with.
    pub fn simple_name(self) -> &'static str {
        match self {
            ListKind::Growable | ListKind::FixedSize => "ArrayList",
            ListKind::ReadOnly => "UnmodifiableRandomAccessList",
        }
    }
}

/// A `SubList` view: `len` elements of `parent` from `from`, valid while
/// the parent's modification count is still `expected` (Java `modCount`).
#[derive(Debug)]
struct SubView {
    parent: ListRef,
    from: usize,
    len: usize,
    expected: u64,
}

#[derive(Debug)]
pub struct ListObj {
    pub kind: ListKind,
    /// The elements, shared with the array for a fixed-size list made from
    /// a reference array. Empty and unused for a view.
    items: Shared<Vec<Value>>,
    /// The array's runtime component type when `items` is an array's
    /// storage: stores are checked against it (`ArrayStoreException`).
    pub elem: Option<Ty>,
    /// Structural modifications so far (Java `modCount`).
    mod_count: std::cell::Cell<u64>,
    view: Option<SubView>,
}

pub const CONCURRENT_MODIFICATION: &str = "java.util.ConcurrentModificationException";

impl ListObj {
    pub fn new(items: Vec<Value>, kind: ListKind) -> ListObj {
        ListObj::shared(Rc::new(RefCell::new(items)), kind, None)
    }

    pub fn shared(items: Shared<Vec<Value>>, kind: ListKind, elem: Option<Ty>) -> ListObj {
        ListObj {
            kind,
            items,
            elem,
            mod_count: std::cell::Cell::new(0),
            view: None,
        }
    }

    /// A read-only view of `parent[from..to]` (bounds already checked, so
    /// a parent view is still valid). A view of a view points straight at
    /// the underlying list, as Java's `SubList` does (it keeps the root
    /// list): views never chain, however often a program takes a `SubList`
    /// of a `SubList`.
    pub fn view(parent: &ListRef, from: usize, to: usize) -> ListObj {
        let (root, offset) = match &parent.view {
            Some(v) => (v.parent.clone(), v.from),
            None => (parent.clone(), 0),
        };
        let expected = root.mod_count.get();
        ListObj {
            kind: ListKind::ReadOnly,
            items: Rc::new(RefCell::new(Vec::new())),
            elem: None,
            mod_count: std::cell::Cell::new(0),
            view: Some(SubView {
                parent: root,
                from: offset + from,
                len: to - from,
                expected,
            }),
        }
    }

    /// The storage and the range this list covers. A view whose parent was
    /// structurally modified raises `ConcurrentModificationException`.
    fn resolve(&self) -> Result<(Shared<Vec<Value>>, usize, usize), Exc> {
        match &self.view {
            None => {
                let len = self.items.borrow().len();
                Ok((self.items.clone(), 0, len))
            }
            Some(v) => {
                // The parent is never a view itself (see `view`).
                if v.parent.mod_count.get() != v.expected {
                    return Err(Exc::bare(CONCURRENT_MODIFICATION));
                }
                Ok((v.parent.items.clone(), v.from, v.len))
            }
        }
    }

    pub fn len(&self) -> Result<usize, Exc> {
        Ok(self.resolve()?.2)
    }

    pub fn to_vec(&self) -> Result<Vec<Value>, Exc> {
        let (items, from, len) = self.resolve()?;
        let mut copy = Vec::new();
        try_grow(&mut copy, len)?;
        copy.extend_from_slice(&items.borrow()[from..from + len]);
        Ok(copy)
    }

    pub fn get(&self, i: usize) -> Result<Option<Value>, Exc> {
        let (items, from, len) = self.resolve()?;
        let items = items.borrow();
        Ok((i < len).then(|| items[from + i].clone()))
    }

    /// The own storage, for changes; only lists that are not views (callers
    /// reject `ReadOnly` lists first).
    pub fn storage(&self) -> &Shared<Vec<Value>> {
        &self.items
    }

    /// Records a structural modification (it invalidates sub-list views).
    pub fn modified(&self) {
        self.mod_count.set(self.mod_count.get() + 1);
    }
}

pub type ListRef = Rc<ListObj>;
pub type MapRef = Shared<OrderedMap>;
pub type SbRef = Shared<Vec<u16>>;

#[derive(Debug)]
pub struct ArrayObj {
    pub elem: Ty,
    /// One length per dimension; elements are stored row-major.
    pub dims: Vec<usize>,
    pub items: Shared<Vec<Value>>,
}

/// A `Type` record: a Java object whose fields exist from the start (with
/// Java defaults) and whose `IsInitialized` becomes true at `Initialize`.
#[derive(Debug)]
pub struct RecordObj {
    pub id: RecordId,
    pub initialized: std::cell::Cell<bool>,
    pub fields: RefCell<Vec<Value>>,
}

/// A runtime error or the value of `LastException`, named after the Java
/// exception B4J raises.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Exc {
    pub kind: &'static str,
    /// `None` for an exception without a message (`toString` is then the
    /// kind alone).
    pub message: Option<String>,
}

pub const RUNTIME: &str = "java.lang.RuntimeException";
pub const NUMBER_FORMAT: &str = "java.lang.NumberFormatException";
pub const INDEX_OUT_OF_BOUNDS: &str = "java.lang.IndexOutOfBoundsException";
pub const ARRAY_INDEX_OUT_OF_BOUNDS: &str = "java.lang.ArrayIndexOutOfBoundsException";
pub const STRING_INDEX_OUT_OF_BOUNDS: &str = "java.lang.StringIndexOutOfBoundsException";
pub const ARITHMETIC: &str = "java.lang.ArithmeticException";
pub const CLASS_CAST: &str = "java.lang.ClassCastException";
pub const UNSUPPORTED: &str = "java.lang.UnsupportedOperationException";
pub const STACK_OVERFLOW: &str = "java.lang.StackOverflowError";
pub const NULL_POINTER: &str = "java.lang.NullPointerException";
pub const ILLEGAL_ARGUMENT: &str = "java.lang.IllegalArgumentException";
pub const NEGATIVE_ARRAY_SIZE: &str = "java.lang.NegativeArraySizeException";
pub const OUT_OF_MEMORY: &str = "java.lang.OutOfMemoryError";
pub const NO_SUCH_FIELD: &str = "java.lang.NoSuchFieldException";
pub const UNSUPPORTED_ENCODING: &str = "java.io.UnsupportedEncodingException";
/// Not a Java exception: the program broke the MIR contract.
pub const INTERNAL: &str = "ferro.InternalError";
pub const ARRAY_STORE: &str = "java.lang.ArrayStoreException";

impl Exc {
    pub fn new(kind: &'static str, message: impl Into<String>) -> Exc {
        Exc {
            kind,
            message: Some(message.into()),
        }
    }

    pub fn bare(kind: &'static str) -> Exc {
        Exc {
            kind,
            message: None,
        }
    }

    /// `Object should first be initialized (List).`
    pub fn not_initialized(what: &str) -> Exc {
        Exc::new(
            RUNTIME,
            format!("Object should first be initialized ({what})."),
        )
    }

    /// The JDK message of `Objects.checkIndex`.
    pub fn index(kind: &'static str, index: i64, length: usize) -> Exc {
        Exc::new(
            kind,
            format!("Index {index} out of bounds for length {length}"),
        )
    }

    pub fn internal(message: impl Into<String>) -> Exc {
        Exc::new(INTERNAL, message)
    }

    /// Java `Throwable.toString`: what `LastException.Message` returns.
    pub fn to_java_string(&self) -> String {
        match &self.message {
            Some(m) => format!("{}: {m}", self.kind),
            None => self.kind.to_string(),
        }
    }

    /// `getClass().getSimpleName()`.
    pub fn simple_name(&self) -> &'static str {
        self.kind.rsplit('.').next().unwrap_or(self.kind)
    }
}

/// Java's longest String or StringBuilder, in UTF-16 code units.
pub const MAX_STRING: usize = i32::MAX as usize;

/// `java.lang.OutOfMemoryError: Java heap space`.
pub fn out_of_memory() -> Exc {
    Exc::new(OUT_OF_MEMORY, "Java heap space")
}

/// Makes room for `extra` more elements. A failed allocation raises
/// `OutOfMemoryError` instead of aborting the process.
pub fn try_grow<T>(v: &mut Vec<T>, extra: usize) -> Result<(), Exc> {
    v.try_reserve(extra).map_err(|_| out_of_memory())
}

/// [`try_grow`] for text, which also stops at Java's length limit.
pub fn try_grow_text(v: &mut Vec<u16>, extra: usize) -> Result<(), Exc> {
    if v.len().saturating_add(extra) > MAX_STRING {
        return Err(out_of_memory());
    }
    try_grow(v, extra)
}

/// A copy of `s`, or `OutOfMemoryError`.
pub fn try_copy_text(s: &[u16]) -> Result<Vec<u16>, Exc> {
    let mut copy = Vec::new();
    try_grow_text(&mut copy, s.len())?;
    copy.extend_from_slice(s);
    Ok(copy)
}

#[derive(Debug, Clone)]
pub enum Value {
    Null,
    Bool(bool),
    Byte(i8),
    Short(i16),
    Int(i32),
    Long(i64),
    Float(f32),
    Double(f64),
    Char(u16),
    Str(Str),
    List(Wrapper<ListRef>),
    Map(Wrapper<MapRef>),
    Sb(Wrapper<SbRef>),
    /// `LastException`; `None` is an uninitialised exception.
    Exc(Option<Rc<Exc>>),
    Array(Rc<ArrayObj>),
    Record(Rc<RecordObj>),
    /// `Map.Keys` (`true`) or `Map.Values`.
    View(MapRef, bool),
}

impl Value {
    pub fn str(s: &str) -> Value {
        Value::Str(Rc::new(to_utf16(s)))
    }

    pub fn from_utf16(s: Vec<u16>) -> Value {
        Value::Str(Rc::new(s))
    }

    #[cfg(test)]
    pub fn new_list(items: Vec<Value>, kind: ListKind) -> Value {
        Value::List(Rc::new(RefCell::new(Some(Rc::new(ListObj::new(
            items, kind,
        ))))))
    }

    pub fn new_map(map: OrderedMap) -> Value {
        Value::Map(Rc::new(RefCell::new(Some(Rc::new(RefCell::new(map))))))
    }

    pub fn is_number(&self) -> bool {
        matches!(
            self,
            Value::Byte(_)
                | Value::Short(_)
                | Value::Int(_)
                | Value::Long(_)
                | Value::Float(_)
                | Value::Double(_)
        )
    }

    /// The value of a number box as a Java `doubleValue()`.
    pub fn number_f64(&self) -> Option<f64> {
        Some(match *self {
            Value::Byte(v) => v as f64,
            Value::Short(v) => v as f64,
            Value::Int(v) => v as f64,
            Value::Long(v) => v as f64,
            Value::Float(v) => v as f64,
            Value::Double(v) => v,
            _ => return None,
        })
    }

    /// The value of an integer box (`longValue()`).
    pub fn integer_i64(&self) -> Option<i64> {
        Some(match *self {
            Value::Byte(v) => v as i64,
            Value::Short(v) => v as i64,
            Value::Int(v) => v as i64,
            Value::Long(v) => v,
            Value::Char(v) => v as i64,
            _ => return None,
        })
    }
}

/// A Java `LinkedHashMap`: insertion order, replacing a value keeps the
/// key's position, keys compare with Java `equals` (see [`HKey`]).
#[derive(Debug, Default)]
pub struct OrderedMap {
    /// Removed entries leave a hole until the next positional access, or
    /// until there are more holes than entries.
    entries: Vec<Option<(Value, Value)>>,
    index: HashMap<HKey, usize>,
    live: usize,
}

impl OrderedMap {
    pub fn len(&self) -> usize {
        self.live
    }

    pub fn get(&self, key: &Value) -> Option<&Value> {
        let i = *self.index.get(&HKey::of(key))?;
        self.entries[i].as_ref().map(|(_, v)| v)
    }

    pub fn contains(&self, key: &Value) -> bool {
        self.index.contains_key(&HKey::of(key))
    }

    /// Returns the previous value.
    pub fn put(&mut self, key: Value, value: Value) -> Result<Option<Value>, Exc> {
        let h = HKey::of(&key);
        if let Some(&i) = self.index.get(&h)
            && let Some((_, v)) = self.entries[i].as_mut()
        {
            return Ok(Some(std::mem::replace(v, value)));
        }
        try_grow(&mut self.entries, 1)?;
        self.index.try_reserve(1).map_err(|_| out_of_memory())?;
        self.index.insert(h, self.entries.len());
        self.entries.push(Some((key, value)));
        self.live += 1;
        Ok(None)
    }

    pub fn remove(&mut self, key: &Value) -> Option<Value> {
        let i = self.index.remove(&HKey::of(key))?;
        let (_, v) = self.entries[i].take()?;
        self.live -= 1;
        if self.live == 0 {
            self.entries.clear();
        } else if self.entries.len() - self.live > self.live.max(8) {
            // Each compaction follows at least as many removals as it
            // moves entries, so removing stays O(1) amortised.
            self.compact();
        }
        Some(v)
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.index.clear();
        self.live = 0;
    }

    /// Removes the holes. Allocates nothing: the index keeps its capacity,
    /// which already holds every live key.
    fn compact(&mut self) {
        if self.entries.len() != self.live {
            self.entries.retain(Option::is_some);
            self.index.clear();
            for (i, (k, _)) in self.entries.iter().flatten().enumerate() {
                self.index.insert(HKey::of(k), i);
            }
        }
    }

    /// A copy of the entries, keys or values (`part`), in order.
    pub fn snapshot<T>(&self, part: impl Fn(&(Value, Value)) -> T) -> Result<Vec<T>, Exc> {
        let mut copy = Vec::new();
        try_grow(&mut copy, self.live)?;
        copy.extend(self.iter().map(part));
        Ok(copy)
    }

    /// The entry at a position, in insertion order.
    pub fn entry_at(&mut self, i: usize) -> Option<(Value, Value)> {
        self.compact();
        self.entries.get(i).cloned().flatten()
    }

    pub fn iter(&self) -> impl Iterator<Item = &(Value, Value)> {
        self.entries.iter().flatten()
    }

    #[cfg(test)]
    pub fn slots(&self) -> usize {
        self.entries.len()
    }
}

// Dropping without recursion. By default, dropping a Value drops what it
// holds, recursively: a long linked list of records, or Lists nested
// 100,000 deep, overflowed the stack. Instead, a container that is about
// to be freed moves the values only it holds onto a work list, and they
// are freed from there, one at a time. The work list reuses the storage of
// the containers being freed, so freeing needs no memory of its own when
// memory has run out.

impl Drop for ListObj {
    fn drop(&mut self) {
        let mut work = Vec::new();
        self.take_values(&mut work);
        dismantle(work);
    }
}

impl Drop for OrderedMap {
    fn drop(&mut self) {
        let mut work = Vec::new();
        self.take_values(&mut work);
        dismantle(work);
    }
}

impl Drop for ArrayObj {
    fn drop(&mut self) {
        let mut work = Vec::new();
        take_items(&mut self.items, &mut work);
        dismantle(work);
    }
}

impl Drop for RecordObj {
    fn drop(&mut self) {
        let mut work = Vec::new();
        absorb(&mut work, std::mem::take(self.fields.get_mut()));
        dismantle(work);
    }
}

impl ListObj {
    fn take_values(&mut self, work: &mut Vec<Value>) {
        take_items(&mut self.items, work);
        if let Some(mut view) = self.view.take()
            && let Some(parent) = Rc::get_mut(&mut view.parent)
        {
            // A view's parent is never a view (see `ListObj::view`).
            take_items(&mut parent.items, work);
        }
    }
}

impl OrderedMap {
    fn take_values(&mut self, work: &mut Vec<Value>) {
        for (k, v) in std::mem::take(&mut self.entries).into_iter().flatten() {
            for value in [k, v] {
                if holds_values(&value) {
                    if work.try_reserve(1).is_ok() {
                        work.push(value);
                    } else {
                        // Out of memory even to free: leak it rather than abort.
                        std::mem::forget(value);
                    }
                }
            }
        }
    }
}

/// Frees the values on `work`: each one first gives up the values only it
/// holds, so freeing it does not recurse.
fn dismantle(mut work: Vec<Value>) {
    while let Some(mut v) = work.pop() {
        take_values(&mut v, &mut work);
    }
}

/// Moves the values that only `v` holds onto `work`.
fn take_values(v: &mut Value, work: &mut Vec<Value>) {
    match v {
        Value::List(w) => {
            if let Some(cell) = Rc::get_mut(w)
                && let Some(list) = cell.get_mut()
                && let Some(list) = Rc::get_mut(list)
            {
                list.take_values(work);
            }
        }
        Value::Map(w) => {
            if let Some(cell) = Rc::get_mut(w)
                && let Some(map) = cell.get_mut()
                && let Some(map) = Rc::get_mut(map)
            {
                map.get_mut().take_values(work);
            }
        }
        Value::View(map, _) => {
            if let Some(map) = Rc::get_mut(map) {
                map.get_mut().take_values(work);
            }
        }
        Value::Array(a) => {
            if let Some(a) = Rc::get_mut(a) {
                take_items(&mut a.items, work);
            }
        }
        Value::Record(r) => {
            if let Some(r) = Rc::get_mut(r) {
                absorb(work, std::mem::take(r.fields.get_mut()));
            }
        }
        _ => {}
    }
}

fn take_items(items: &mut Shared<Vec<Value>>, work: &mut Vec<Value>) {
    if let Some(items) = Rc::get_mut(items) {
        absorb(work, std::mem::take(items.get_mut()));
    }
}

/// Whether a value can hold other values.
fn holds_values(v: &Value) -> bool {
    matches!(
        v,
        Value::List(_) | Value::Map(_) | Value::View(..) | Value::Array(_) | Value::Record(_)
    )
}

/// Moves the values in `values` that can hold others onto `work`, and frees
/// the rest (numbers, Strings, StringBuilders, exceptions) at once. The
/// values go into `work`, or `work` goes into `values` if only that has room
/// for both, so this usually allocates nothing (and `work` grows by
/// doubling otherwise); if memory has run out even for that, the values are
/// leaked rather than aborting.
fn absorb(work: &mut Vec<Value>, mut values: Vec<Value>) {
    values.retain(holds_values);
    if values.is_empty() {
        return;
    }
    if work.capacity() - work.len() < values.len() && values.capacity() - values.len() >= work.len()
    {
        std::mem::swap(work, &mut values);
    }
    if work.try_reserve(values.len()).is_ok() {
        work.append(&mut values);
    } else {
        std::mem::forget(values);
    }
}

/// A hashable key with Java `equals` semantics for boxes and Strings:
/// different box types are different keys, Doubles compare by
/// `doubleToLongBits` (NaN equals NaN, 0.0 differs from -0.0).
/// Other objects compare by identity.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum HKey {
    Null,
    Bool(bool),
    Byte(i8),
    Short(i16),
    Int(i32),
    Long(i64),
    Float(u32),
    Double(u64),
    Char(u16),
    Str(Str),
    Ref(usize),
}

fn ptr<T: ?Sized>(rc: &Rc<T>) -> usize {
    Rc::as_ptr(rc) as *const u8 as usize
}

impl HKey {
    pub fn of(v: &Value) -> HKey {
        match v {
            Value::Null => HKey::Null,
            Value::Bool(b) => HKey::Bool(*b),
            Value::Byte(x) => HKey::Byte(*x),
            Value::Short(x) => HKey::Short(*x),
            Value::Int(x) => HKey::Int(*x),
            Value::Long(x) => HKey::Long(*x),
            Value::Float(x) => HKey::Float(if x.is_nan() { 0x7fc0_0000 } else { x.to_bits() }),
            Value::Double(x) => HKey::Double(if x.is_nan() {
                0x7ff8_0000_0000_0000
            } else {
                x.to_bits()
            }),
            Value::Char(c) => HKey::Char(*c),
            Value::Str(s) => HKey::Str(s.clone()),
            Value::List(w) => w
                .borrow()
                .as_ref()
                .map_or(HKey::Null, |l| HKey::Ref(ptr(l))),
            Value::Map(w) => w
                .borrow()
                .as_ref()
                .map_or(HKey::Null, |m| HKey::Ref(ptr(m))),
            Value::Sb(w) => w
                .borrow()
                .as_ref()
                .map_or(HKey::Null, |s| HKey::Ref(ptr(s))),
            Value::Exc(e) => e.as_ref().map_or(HKey::Null, |e| HKey::Ref(ptr(e))),
            Value::Array(a) => HKey::Ref(ptr(a)),
            Value::Record(r) => HKey::Ref(ptr(r)),
            Value::View(m, keys) => HKey::Ref(ptr(m) + *keys as usize),
        }
    }
}

/// How deep `equals` may go into nested collections before raising
/// `StackOverflowError`, as Java's recursive `ArrayList.equals` does on
/// self-containing lists (`a.Add(a)`, `b.Add(b)`, `a = b`).
const MAX_EQUALS_DEPTH: usize = 1000;

/// Java `equals` on boxed values. Lists and maps compare by content, as
/// `ArrayList.equals` and `AbstractMap.equals` do; other objects by identity.
///
/// Runs on an explicit work list (no Rust recursion) in Java's order:
/// depth first, elements left to right, so the first difference or the
/// overflow is found where Java finds it.
pub fn java_equals(a: &Value, b: &Value) -> Result<bool, Exc> {
    enum Work {
        Pair(Value, Value, usize),
        /// A map key missing from the other map.
        Unequal,
    }
    let mut work = vec![Work::Pair(a.clone(), b.clone(), 0)];
    while let Some(item) = work.pop() {
        let (a, b, depth) = match item {
            Work::Pair(a, b, depth) => (a, b, depth),
            Work::Unequal => return Ok(false),
        };
        if depth > MAX_EQUALS_DEPTH {
            return Err(Exc::bare(STACK_OVERFLOW));
        }
        match (&a, &b) {
            (Value::List(x), Value::List(y)) => {
                let (x, y) = (x.borrow().clone(), y.borrow().clone());
                match (x, y) {
                    (Some(x), Some(y)) => {
                        if Rc::ptr_eq(&x, &y) {
                            continue;
                        }
                        let (x, y) = (x.to_vec()?, y.to_vec()?);
                        if x.len() != y.len() {
                            return Ok(false);
                        }
                        for (p, q) in x.into_iter().zip(y).rev() {
                            work.push(Work::Pair(p, q, depth + 1));
                        }
                    }
                    (None, None) => {}
                    _ => return Ok(false),
                }
            }
            (Value::Map(x), Value::Map(y)) => {
                let (x, y) = (x.borrow().clone(), y.borrow().clone());
                match (x, y) {
                    (Some(x), Some(y)) => {
                        if Rc::ptr_eq(&x, &y) {
                            continue;
                        }
                        let xs = x.borrow().snapshot(Clone::clone)?;
                        if xs.len() != y.borrow().len() {
                            return Ok(false);
                        }
                        let mut pending = Vec::with_capacity(xs.len());
                        for (k, v) in xs {
                            pending.push(match y.borrow().get(&k) {
                                Some(w) => Work::Pair(v, w.clone(), depth + 1),
                                None => Work::Unequal,
                            });
                        }
                        work.extend(pending.into_iter().rev());
                    }
                    (None, None) => {}
                    _ => return Ok(false),
                }
            }
            _ => {
                if HKey::of(&a) != HKey::of(&b) {
                    return Ok(false);
                }
            }
        }
    }
    Ok(true)
}

/// Java `getClass().getName()` of a value held in an `Object`.
pub fn java_class_name(v: &Value, program: &Program) -> Option<String> {
    Some(match v {
        Value::Null => return None,
        Value::Bool(_) => "java.lang.Boolean".into(),
        Value::Byte(_) => "java.lang.Byte".into(),
        Value::Short(_) => "java.lang.Short".into(),
        Value::Int(_) => "java.lang.Integer".into(),
        Value::Long(_) => "java.lang.Long".into(),
        Value::Float(_) => "java.lang.Float".into(),
        Value::Double(_) => "java.lang.Double".into(),
        Value::Char(_) => "java.lang.Character".into(),
        Value::Str(_) => "java.lang.String".into(),
        Value::List(w) => w.borrow().as_ref()?.kind.class_name().into(),
        Value::Map(w) => {
            w.borrow().as_ref()?;
            "anywheresoftware.b4a.objects.collections.Map$MyMap".into()
        }
        Value::Sb(w) => {
            w.borrow().as_ref()?;
            "java.lang.StringBuilder".into()
        }
        Value::Exc(e) => e.as_ref()?.kind.into(),
        Value::Array(a) => {
            let mut s = "[".repeat(a.dims.len().max(1));
            s.push_str(&array_descriptor(&a.elem));
            s
        }
        // Deviation: B4J names the generated class after the package.
        Value::Record(r) => format!(
            "_{}",
            program
                .records
                .get(r.id.0 as usize)
                .map_or(String::new(), |rec| rec.name.to_lowercase())
        ),
        Value::View(_, true) => "java.util.LinkedHashMap$LinkedKeySet".into(),
        Value::View(_, false) => "java.util.LinkedHashMap$LinkedValues".into(),
    })
}

/// Java's name of an array class: `[I`, `[[Ljava.lang.String;`.
pub fn array_class_name(elem: &Ty, rank: u8) -> String {
    let mut s = "[".repeat(rank.max(1) as usize);
    s.push_str(&array_descriptor(elem));
    s
}

fn array_descriptor(elem: &Ty) -> String {
    match elem {
        Ty::Boolean => "Z".into(),
        Ty::Byte => "B".into(),
        Ty::Short => "S".into(),
        Ty::Int => "I".into(),
        Ty::Long => "J".into(),
        Ty::Float => "F".into(),
        Ty::Double => "D".into(),
        Ty::Char => "C".into(),
        Ty::String => "Ljava.lang.String;".into(),
        Ty::List => "Ljava.util.List;".into(),
        Ty::Map => "Ljava.util.Map;".into(),
        _ => "Ljava.lang.Object;".into(),
    }
}

/// How many collections and records may be nested in one another while
/// printing, before `StackOverflowError`: Java's `toString` recurses, like
/// `equals` (see [`MAX_EQUALS_DEPTH`]).
const MAX_PRINT_DEPTH: usize = MAX_EQUALS_DEPTH;

/// Java `String.valueOf(object)` of a value held in an `Object` (so a
/// collection prints without its wrapper's prefix and a Double keeps its
/// `.0`). Self-containing collections print `(this Collection)` /
/// `(this Map)`. Fails like Java's `toString` does, e.g. on a sub-list
/// whose parent changed.
pub fn raw_to_string(v: &Value, program: &Program, out: &mut Vec<u16>) -> Result<(), Exc> {
    Printer::new(program, out).run(Print::Raw(v.clone()))
}

/// `AbsObjectWrapper.toString`: what a `List`, `Map`, `StringBuilder` or
/// `Exception` *variable* prints: `(ArrayList) [1, 2]`, `(MyMap) {a=1}`,
/// `(List) Not initialized`. The prefix is the inner object's simple class
/// name (`(UnmodifiableRandomAccessList)` for a `SubList`). A Java `null`
/// wrapper reference (a field of a record that was never initialised)
/// prints `null`.
pub fn wrapper_to_string(v: &Value, program: &Program, out: &mut Vec<u16>) -> Result<(), Exc> {
    Printer::new(program, out).run(Print::Wrapper(v.clone()))
}

/// The ids of the collections and records being printed, outermost first.
type Seen = Vec<usize>;

/// What is left to print, on a work list rather than the Rust stack. A
/// collection is one job that prints an element at a time, so the list is
/// only as long as the nesting is deep.
enum Print<'p> {
    /// `String.valueOf` of a value.
    Raw(Value),
    /// `AbsObjectWrapper.toString` of a value, which starts over: its
    /// object does not know which collections enclose the wrapper.
    Wrapper(Value),
    /// The end of a `Wrapper`: back to the enclosing collections.
    Restore(Seen),
    /// A collection's elements from `next` on, then `]`.
    Items { items: Vec<Value>, next: usize },
    /// A Map's keys and values from `next` on (two per entry), then `}`.
    Entries {
        entries: Vec<(Value, Value)>,
        next: usize,
    },
    /// A record's fields from `next` on, then `]`.
    Fields {
        fields: &'p [RecordField],
        values: Vec<Value>,
        next: usize,
    },
}

struct Printer<'p, 'o> {
    program: &'p Program,
    out: &'o mut Vec<u16>,
    jobs: Vec<Print<'p>>,
    seen: Seen,
    /// Collections and records entered and not yet left, across wrappers.
    depth: usize,
}

impl<'p, 'o> Printer<'p, 'o> {
    fn new(program: &'p Program, out: &'o mut Vec<u16>) -> Printer<'p, 'o> {
        Printer {
            program,
            out,
            jobs: Vec::new(),
            seen: Vec::new(),
            depth: 0,
        }
    }

    fn run(mut self, first: Print<'p>) -> Result<(), Exc> {
        self.jobs.push(first);
        while let Some(job) = self.jobs.pop() {
            match job {
                Print::Raw(v) => self.raw(&v)?,
                Print::Wrapper(v) => self.wrapper(v)?,
                Print::Restore(seen) => self.seen = seen,
                Print::Items { mut items, next } => {
                    if next == items.len() {
                        self.put("]")?;
                        self.leave();
                        continue;
                    }
                    if next > 0 {
                        self.put(", ")?;
                    }
                    let item = std::mem::replace(&mut items[next], Value::Null);
                    // The rest waits below the element's own jobs.
                    self.jobs.push(Print::Items {
                        items,
                        next: next + 1,
                    });
                    self.element(&item, "(this Collection)")?;
                }
                Print::Entries { mut entries, next } => {
                    if next == 2 * entries.len() {
                        self.put("}")?;
                        self.leave();
                        continue;
                    }
                    let (k, v) = &mut entries[next / 2];
                    let item = if next % 2 == 0 {
                        if next > 0 {
                            self.put(", ")?;
                        }
                        std::mem::replace(k, Value::Null)
                    } else {
                        self.put("=")?;
                        std::mem::replace(v, Value::Null)
                    };
                    self.jobs.push(Print::Entries {
                        entries,
                        next: next + 1,
                    });
                    self.element(&item, "(this Map)")?;
                }
                Print::Fields {
                    fields,
                    mut values,
                    next,
                } => {
                    let Some(field) = fields.get(next).filter(|_| next < values.len()) else {
                        self.put("]")?;
                        self.leave();
                        continue;
                    };
                    self.put(", ")?;
                    self.put(&field.name)?;
                    self.put("=")?;
                    let value = std::mem::replace(&mut values[next], Value::Null);
                    self.jobs.push(Print::Fields {
                        fields,
                        values,
                        next: next + 1,
                    });
                    if is_wrapper_type(&field.ty) {
                        self.wrapper(value)?;
                    } else {
                        self.element(&value, "(this Collection)")?;
                    }
                }
            }
        }
        Ok(())
    }

    /// Appends text; very long text raises `OutOfMemoryError`.
    fn put(&mut self, s: &str) -> Result<(), Exc> {
        try_grow_text(self.out, s.encode_utf16().count())?;
        self.out.extend(s.encode_utf16());
        Ok(())
    }

    fn put_units(&mut self, s: &[u16]) -> Result<(), Exc> {
        try_grow_text(self.out, s.len())?;
        self.out.extend_from_slice(s);
        Ok(())
    }

    /// Starts printing a collection or record, as `id`.
    fn enter(&mut self, id: usize) -> Result<(), Exc> {
        if self.depth >= MAX_PRINT_DEPTH {
            return Err(Exc::bare(STACK_OVERFLOW));
        }
        self.seen.push(id);
        self.depth += 1;
        Ok(())
    }

    fn leave(&mut self) {
        self.seen.pop();
        self.depth -= 1;
    }

    /// An element: `this` if it is a collection being printed.
    fn element(&mut self, v: &Value, this: &str) -> Result<(), Exc> {
        if identity(v).is_some_and(|id| self.seen.contains(&id)) {
            self.put(this)
        } else {
            self.raw(v)
        }
    }

    fn raw(&mut self, v: &Value) -> Result<(), Exc> {
        match v {
            Value::Null => self.put("null")?,
            Value::Bool(b) => self.put(if *b { "true" } else { "false" })?,
            Value::Byte(x) => self.put(&x.to_string())?,
            Value::Short(x) => self.put(&x.to_string())?,
            Value::Int(x) => self.put(&x.to_string())?,
            Value::Long(x) => self.put(&x.to_string())?,
            Value::Float(x) => self.put(&java_float_to_string(*x))?,
            Value::Double(x) => self.put(&java_double_to_string(*x))?,
            Value::Char(c) => self.put_units(&[*c])?,
            Value::Str(s) => self.put_units(s)?,
            Value::List(w) => match w.borrow().clone() {
                None => self.put("null")?,
                Some(l) => self.sequence(l.to_vec()?, ptr(&l))?,
            },
            Value::Map(w) => match w.borrow().clone() {
                None => self.put("null")?,
                Some(m) => {
                    let entries = m.borrow().snapshot(Clone::clone)?;
                    self.enter(ptr(&m))?;
                    self.put("{")?;
                    self.jobs.push(Print::Entries { entries, next: 0 });
                }
            },
            Value::Sb(w) => match w.borrow().as_ref() {
                None => self.put("null")?,
                Some(sb) => self.put_units(&sb.borrow())?,
            },
            Value::Exc(e) => match e {
                None => self.put("null")?,
                // `Throwable.toString`, without building it first.
                Some(e) => {
                    self.put(e.kind)?;
                    if let Some(message) = &e.message {
                        self.put(": ")?;
                        self.put(message)?;
                    }
                }
            },
            Value::Array(a) => {
                // Java prints the identity hash code, which no test can compare.
                let mut name = "[".repeat(a.dims.len().max(1));
                name.push_str(&array_descriptor(&a.elem));
                self.put(&format!("{name}@{:x}", (ptr(a) >> 4) & 0x7fff_ffff))?;
            }
            Value::Record(r) => self.record(r)?,
            Value::View(m, keys) => {
                let items = m
                    .borrow()
                    .snapshot(|(k, v)| if *keys { k.clone() } else { v.clone() })?;
                self.sequence(items, ptr(m))?;
            }
        }
        Ok(())
    }

    /// `[a, b, c]`, as `AbstractCollection.toString` prints it.
    fn sequence(&mut self, items: Vec<Value>, id: usize) -> Result<(), Exc> {
        self.enter(id)?;
        self.put("[")?;
        self.jobs.push(Print::Items { items, next: 0 });
        Ok(())
    }

    /// The `toString` B4X generates for a `Type`:
    /// `[IsInitialized=true, Name=abc, Price=1.5]`.
    fn record(&mut self, r: &Rc<RecordObj>) -> Result<(), Exc> {
        let Some(rec) = self.program.records.get(r.id.0 as usize) else {
            return self.put("[]");
        };
        let mut values = Vec::new();
        try_grow(&mut values, r.fields.borrow().len())?;
        values.extend_from_slice(&r.fields.borrow());
        self.enter(ptr(r))?;
        self.put("[IsInitialized=")?;
        self.put(if r.initialized.get() { "true" } else { "false" })?;
        self.jobs.push(Print::Fields {
            fields: &rec.fields,
            values,
            next: 0,
        });
        Ok(())
    }

    fn wrapper(&mut self, v: Value) -> Result<(), Exc> {
        // The wrapper's type, and its object's simple class name if any.
        let wrapper = match &v {
            Value::List(w) => Some(("List", w.borrow().as_ref().map(|l| l.kind.simple_name()))),
            Value::Map(w) => Some(("Map", w.borrow().is_some().then_some("MyMap"))),
            Value::Sb(w) => Some((
                "StringBuilder",
                w.borrow().is_some().then_some("StringBuilder"),
            )),
            Value::Exc(e) => Some(("Exception", e.as_ref().map(|e| e.simple_name()))),
            _ => None,
        };
        match wrapper {
            Some((short, None)) => {
                return self.put(&format!("({short}) Not initialized"));
            }
            Some((_, Some(simple))) => self.put(&format!("({simple}) "))?,
            None => {}
        }
        let enclosing = std::mem::take(&mut self.seen);
        self.jobs.push(Print::Restore(enclosing));
        self.jobs.push(Print::Raw(v));
        Ok(())
    }
}

fn identity(v: &Value) -> Option<usize> {
    match v {
        Value::List(w) => w.borrow().as_ref().map(ptr),
        Value::Map(w) => w.borrow().as_ref().map(ptr),
        Value::Record(r) => Some(ptr(r)),
        _ => None,
    }
}

pub fn is_wrapper_type(ty: &Ty) -> bool {
    matches!(ty, Ty::List | Ty::Map | Ty::StringBuilder | Ty::Exception)
}

/// The text of a `String` value (`null` for Null), shared.
pub fn utf16_of(v: &Value) -> Str {
    match v {
        Value::Str(s) => s.clone(),
        _ => Rc::new(to_utf16("null")),
    }
}

/// The Rust text of a `String` value (`null` for Null).
#[cfg(test)]
pub fn text_of(v: &Value) -> String {
    match v {
        Value::Str(s) => to_rust(s),
        _ => "null".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn removed_map_entries_are_compacted() {
        let mut m = OrderedMap::default();
        m.put(Value::str("first"), Value::Int(0)).unwrap();
        for i in 1..10_000 {
            m.put(Value::Int(i), Value::Int(i)).unwrap();
            m.remove(&Value::Int(i - 1));
            assert!(m.slots() <= 2 * m.len() + 9, "{} slots", m.slots());
        }
        let keys: Vec<HKey> = m.iter().map(|(k, _)| HKey::of(k)).collect();
        assert_eq!(keys, [HKey::of(&Value::str("first")), HKey::Int(9_999)]);
        assert!(matches!(m.get(&Value::Int(9_999)), Some(Value::Int(9_999))));
        assert!(matches!(m.entry_at(1), Some((Value::Int(9_999), _))));
    }

    #[test]
    fn text_stops_at_java_length_limit() {
        let mut text = vec![0u16; 4];
        assert!(try_grow_text(&mut text, MAX_STRING - 3).is_err_and(|e| e.kind == OUT_OF_MEMORY));
        assert!(try_grow_text(&mut text, usize::MAX).is_err());
        assert!(try_grow_text(&mut text, 10).is_ok());
        let mut huge: Vec<Value> = Vec::new();
        assert!(try_grow(&mut huge, usize::MAX / 2).is_err_and(|e| e.kind == OUT_OF_MEMORY));
    }
}
