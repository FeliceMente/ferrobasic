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

use ferro_mir::{Program, RecordId, Ty};

use crate::number::{java_double_to_string, java_float_to_string};
use crate::strings::{to_rust, to_utf16};

pub type Str = Rc<[u16]>;
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

    /// A read-only view of `parent[from..to]` (bounds already checked).
    pub fn view(parent: &ListRef, from: usize, to: usize) -> ListObj {
        ListObj {
            kind: ListKind::ReadOnly,
            items: Rc::new(RefCell::new(Vec::new())),
            elem: None,
            mod_count: std::cell::Cell::new(0),
            view: Some(SubView {
                parent: parent.clone(),
                from,
                len: to - from,
                expected: parent.mod_count.get(),
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
                if v.parent.mod_count.get() != v.expected {
                    return Err(Exc::bare(CONCURRENT_MODIFICATION));
                }
                let (items, offset, _) = v.parent.resolve()?;
                Ok((items, offset + v.from, v.len))
            }
        }
    }

    pub fn len(&self) -> Result<usize, Exc> {
        Ok(self.resolve()?.2)
    }

    pub fn to_vec(&self) -> Result<Vec<Value>, Exc> {
        let (items, from, len) = self.resolve()?;
        let items = items.borrow();
        Ok(items[from..from + len].to_vec())
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
        Value::Str(to_utf16(s).into())
    }

    pub fn from_utf16(s: Vec<u16>) -> Value {
        Value::Str(s.into())
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
    /// Removed entries leave a hole until the next positional access.
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
    pub fn put(&mut self, key: Value, value: Value) -> Option<Value> {
        let h = HKey::of(&key);
        if let Some(&i) = self.index.get(&h)
            && let Some((_, v)) = self.entries[i].as_mut()
        {
            return Some(std::mem::replace(v, value));
        }
        self.index.insert(h, self.entries.len());
        self.entries.push(Some((key, value)));
        self.live += 1;
        None
    }

    pub fn remove(&mut self, key: &Value) -> Option<Value> {
        let i = self.index.remove(&HKey::of(key))?;
        let (_, v) = self.entries[i].take()?;
        self.live -= 1;
        if self.live == 0 {
            self.entries.clear();
        }
        Some(v)
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.index.clear();
        self.live = 0;
    }

    fn compact(&mut self) {
        if self.entries.len() != self.live {
            self.entries.retain(Option::is_some);
            self.index = self
                .entries
                .iter()
                .enumerate()
                .filter_map(|(i, e)| e.as_ref().map(|(k, _)| (HKey::of(k), i)))
                .collect();
        }
    }

    /// The entry at a position, in insertion order.
    pub fn entry_at(&mut self, i: usize) -> Option<(Value, Value)> {
        self.compact();
        self.entries.get(i).cloned().flatten()
    }

    pub fn iter(&self) -> impl Iterator<Item = &(Value, Value)> {
        self.entries.iter().flatten()
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
                        let xs: Vec<(Value, Value)> = x.borrow().iter().cloned().collect();
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

/// Appends text to a UTF-16 buffer.
fn push(out: &mut Vec<u16>, s: &str) {
    out.extend(s.encode_utf16());
}

/// Java `String.valueOf(object)` of a value held in an `Object` (so a
/// collection prints without its wrapper's prefix and a Double keeps its
/// `.0`). Self-containing collections print `(this Collection)` /
/// `(this Map)`. Fails like Java's `toString` does, e.g. on a sub-list
/// whose parent changed.
pub fn raw_to_string(v: &Value, program: &Program, out: &mut Vec<u16>) -> Result<(), Exc> {
    raw(v, program, out, &mut Vec::new())
}

type Seen = Vec<usize>;

fn raw(v: &Value, program: &Program, out: &mut Vec<u16>, seen: &mut Seen) -> Result<(), Exc> {
    match v {
        Value::Null => push(out, "null"),
        Value::Bool(b) => push(out, if *b { "true" } else { "false" }),
        Value::Byte(x) => push(out, &x.to_string()),
        Value::Short(x) => push(out, &x.to_string()),
        Value::Int(x) => push(out, &x.to_string()),
        Value::Long(x) => push(out, &x.to_string()),
        Value::Float(x) => push(out, &java_float_to_string(*x)),
        Value::Double(x) => push(out, &java_double_to_string(*x)),
        Value::Char(c) => out.push(*c),
        Value::Str(s) => out.extend_from_slice(s),
        Value::List(w) => match w.borrow().clone() {
            None => push(out, "null"),
            Some(l) => {
                let items = l.to_vec()?;
                sequence(&items, ptr(&l), "(this Collection)", program, out, seen)?;
            }
        },
        Value::Map(w) => match w.borrow().clone() {
            None => push(out, "null"),
            Some(m) => {
                let id = ptr(&m);
                let entries: Vec<(Value, Value)> = m.borrow().iter().cloned().collect();
                seen.push(id);
                out.push(b'{' as u16);
                for (i, (k, val)) in entries.iter().enumerate() {
                    if i > 0 {
                        push(out, ", ");
                    }
                    element(k, "(this Map)", program, out, seen)?;
                    out.push(b'=' as u16);
                    element(val, "(this Map)", program, out, seen)?;
                }
                out.push(b'}' as u16);
                seen.pop();
            }
        },
        Value::Sb(w) => match w.borrow().as_ref() {
            None => push(out, "null"),
            Some(sb) => out.extend_from_slice(&sb.borrow()),
        },
        Value::Exc(e) => match e {
            None => push(out, "null"),
            Some(e) => push(out, &e.to_java_string()),
        },
        Value::Array(a) => {
            // Java prints the identity hash code, which no test can compare.
            let mut name = "[".repeat(a.dims.len().max(1));
            name.push_str(&array_descriptor(&a.elem));
            push(out, &format!("{name}@{:x}", (ptr(a) >> 4) & 0x7fff_ffff));
        }
        Value::Record(r) => record_to_string(r, program, out, seen)?,
        Value::View(m, keys) => {
            let items: Vec<Value> = m
                .borrow()
                .iter()
                .map(|(k, v)| if *keys { k.clone() } else { v.clone() })
                .collect();
            sequence(&items, ptr(m), "(this Collection)", program, out, seen)?;
        }
    }
    Ok(())
}

fn identity(v: &Value) -> Option<usize> {
    match v {
        Value::List(w) => w.borrow().as_ref().map(ptr),
        Value::Map(w) => w.borrow().as_ref().map(ptr),
        Value::Record(r) => Some(ptr(r)),
        _ => None,
    }
}

fn element(
    v: &Value,
    this: &str,
    program: &Program,
    out: &mut Vec<u16>,
    seen: &mut Seen,
) -> Result<(), Exc> {
    if identity(v).is_some_and(|id| seen.contains(&id)) {
        push(out, this);
        Ok(())
    } else {
        raw(v, program, out, seen)
    }
}

fn sequence(
    items: &[Value],
    id: usize,
    this: &str,
    program: &Program,
    out: &mut Vec<u16>,
    seen: &mut Seen,
) -> Result<(), Exc> {
    seen.push(id);
    out.push(b'[' as u16);
    for (i, item) in items.iter().enumerate() {
        if i > 0 {
            push(out, ", ");
        }
        element(item, this, program, out, seen)?;
    }
    out.push(b']' as u16);
    seen.pop();
    Ok(())
}

/// The `toString` B4X generates for a `Type`:
/// `[IsInitialized=true, Name=abc, Price=1.5]`.
fn record_to_string(
    r: &Rc<RecordObj>,
    program: &Program,
    out: &mut Vec<u16>,
    seen: &mut Seen,
) -> Result<(), Exc> {
    let Some(rec) = program.records.get(r.id.0 as usize) else {
        push(out, "[]");
        return Ok(());
    };
    let fields = r.fields.borrow().clone();
    seen.push(ptr(r));
    push(out, "[IsInitialized=");
    push(out, if r.initialized.get() { "true" } else { "false" });
    for (f, v) in rec.fields.iter().zip(&fields) {
        push(out, ", ");
        push(out, &f.name);
        out.push(b'=' as u16);
        if is_wrapper_type(&f.ty) {
            wrapper_to_string(v, program, out)?;
        } else {
            element(v, "(this Collection)", program, out, seen)?;
        }
    }
    out.push(b']' as u16);
    seen.pop();
    Ok(())
}

pub fn is_wrapper_type(ty: &Ty) -> bool {
    matches!(ty, Ty::List | Ty::Map | Ty::StringBuilder | Ty::Exception)
}

/// `AbsObjectWrapper.toString`: what a `List`, `Map`, `StringBuilder` or
/// `Exception` *variable* prints: `(ArrayList) [1, 2]`, `(MyMap) {a=1}`,
/// `(List) Not initialized`. The prefix is the inner object's simple class
/// name (`(UnmodifiableRandomAccessList)` for a `SubList`). A Java `null`
/// wrapper reference (a field of a record that was never initialised)
/// prints `null`.
pub fn wrapper_to_string(v: &Value, program: &Program, out: &mut Vec<u16>) -> Result<(), Exc> {
    let (short, simple) = match v {
        Value::List(w) => ("List", w.borrow().as_ref().map(|l| l.kind.simple_name())),
        Value::Map(w) => ("Map", w.borrow().is_some().then_some("MyMap")),
        Value::Sb(w) => (
            "StringBuilder",
            w.borrow().is_some().then_some("StringBuilder"),
        ),
        Value::Exc(e) => ("Exception", e.as_ref().map(|e| e.simple_name())),
        other => return raw_to_string(other, program, out),
    };
    match simple {
        None => push(out, &format!("({short}) Not initialized")),
        Some(simple) => {
            push(out, &format!("({simple}) "));
            raw_to_string(v, program, out)?;
        }
    }
    Ok(())
}

/// The Rust text of a `String` value (`null` for Null).
pub fn text_of(v: &Value) -> String {
    match v {
        Value::Str(s) => to_rust(s),
        _ => "null".into(),
    }
}
