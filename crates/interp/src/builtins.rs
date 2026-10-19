//! The Level 0 built-ins (`docs/spec/level0-builtins.md`), with the
//! semantics of the B4J runtime and the Java classes it wraps.

use std::cell::RefCell;
use std::cmp::Ordering;
use std::rc::Rc;

use ferro_mir::{Builtin, Ty};

use crate::convert::{array_to_list, checked_store};
use crate::machine::{Machine, Signal, as_array, int_of};
use crate::number::{JavaRandom, number_format, parse_java_double};
use crate::strings::{self, compare_ignore_case, compare_to, to_rust, to_utf16};
use crate::value::*;

type R = Result<Value, Signal>;

fn str_arg(v: &Value) -> Result<&[u16], Exc> {
    match v {
        Value::Str(s) => Ok(s),
        Value::Null => Err(Exc::bare(NULL_POINTER)),
        _ => Err(Exc::internal("expected a String")),
    }
}

fn f64_arg(v: &Value) -> Result<f64, Exc> {
    v.number_f64()
        .or_else(|| v.integer_i64().map(|i| i as f64))
        .ok_or_else(|| Exc::internal("expected a number"))
}

fn bool_arg(v: &Value) -> Result<bool, Exc> {
    match v {
        Value::Bool(b) => Ok(*b),
        _ => Err(Exc::internal("expected a Boolean")),
    }
}

fn arg(args: &[Value], i: usize) -> Result<&Value, Exc> {
    args.get(i).ok_or_else(|| Exc::internal("missing argument"))
}

fn text(s: Vec<u16>) -> Value {
    Value::from_utf16(s)
}

/// `Math.round(double)`: halves round up, NaN → 0, saturating.
pub fn java_round(x: f64) -> i64 {
    if x.is_nan() {
        return 0;
    }
    let f = x.floor();
    let r = if x - f >= 0.5 { f + 1.0 } else { f };
    r as i64
}

/// `Math.max` / `Math.min` on doubles: NaN wins, -0.0 < 0.0.
fn java_max(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else if a == b {
        if a.is_sign_negative() { b } else { a }
    } else {
        a.max(b)
    }
}

fn java_min(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else if a == b {
        if a.is_sign_negative() { a } else { b }
    } else {
        a.min(b)
    }
}

/// `Math.atan2` (fdlibm, as `StrictMath`). `libm` is a port of the same
/// code, except that when |y/x| > 2^60 and x < 0 fdlibm adds the low part
/// of π/2 back, and so returns a value one ulp off π/2.
fn java_atan2(y: f64, x: f64) -> f64 {
    let high = |v: f64| ((v.to_bits() >> 32) as i32) & 0x7fff_ffff;
    if x < 0.0 && x.is_finite() && y.is_finite() && y != 0.0 && (high(y) - high(x)) >> 20 > 60 {
        // π − (atan(|y/x|) − π_lo), with atan(|y/x|) = π/2 + π_lo/2.
        use std::f64::consts::{FRAC_PI_2, PI};
        const PI_LO: f64 = 1.224_646_799_147_353_2e-16;
        let v = PI - ((FRAC_PI_2 + 0.5 * PI_LO) - PI_LO);
        return if y < 0.0 { -v } else { v };
    }
    libm::atan2(y, x)
}

/// `Math.pow`: unlike C, a NaN exponent always gives NaN, and ±1 to an
/// infinite power is NaN.
fn java_pow(x: f64, y: f64) -> f64 {
    if y.is_nan() || (x.abs() == 1.0 && y.is_infinite()) {
        f64::NAN
    } else {
        x.powf(y)
    }
}

/// `Double.compare`.
fn java_double_cmp(a: f64, b: f64) -> Ordering {
    if a < b {
        Ordering::Less
    } else if a > b {
        Ordering::Greater
    } else {
        let bits = |x: f64| {
            if x.is_nan() {
                0x7ff8_0000_0000_0000i64
            } else {
                x.to_bits() as i64
            }
        };
        bits(a).cmp(&bits(b))
    }
}

/// `Integer.parseInt(s, radix)`.
fn parse_int(s: &[u16], radix: i32) -> Result<i32, Exc> {
    if !(2..=36).contains(&radix) {
        return Err(Exc::new(
            NUMBER_FORMAT,
            format!("radix {radix} out of range"),
        ));
    }
    let text = to_rust(s);
    let fail = || {
        let suffix = if radix == 10 {
            String::new()
        } else {
            format!(" under radix {radix}")
        };
        Exc::new(
            NUMBER_FORMAT,
            format!("For input string: \"{text}\"{suffix}"),
        )
    };
    let (neg, digits) = match text.as_bytes().first() {
        Some(b'-') => (true, &text[1..]),
        Some(b'+') => (false, &text[1..]),
        _ => (false, text.as_str()),
    };
    if digits.is_empty() {
        return Err(fail());
    }
    let mut v: i64 = 0;
    for c in digits.chars() {
        let d = c.to_digit(radix as u32).ok_or_else(fail)? as i64;
        v = v * radix as i64 + d;
        if v > i32::MAX as i64 + 1 {
            return Err(fail());
        }
    }
    let v = if neg { -v } else { v };
    i32::try_from(v).map_err(|_| fail())
}

/// The class a natural-order sort compares by (`Comparable`).
#[derive(PartialEq, Eq, Clone, Copy)]
enum Class {
    Bool,
    Byte,
    Short,
    Int,
    Long,
    Float,
    Double,
    Char,
    Str,
}

fn comparable_class(v: &Value) -> Option<Class> {
    Some(match v {
        Value::Bool(_) => Class::Bool,
        Value::Byte(_) => Class::Byte,
        Value::Short(_) => Class::Short,
        Value::Int(_) => Class::Int,
        Value::Long(_) => Class::Long,
        Value::Float(_) => Class::Float,
        Value::Double(_) => Class::Double,
        Value::Char(_) => Class::Char,
        Value::Str(_) => Class::Str,
        _ => return None,
    })
}

/// Natural order of two values of the same comparable class.
fn natural_cmp(a: &Value, b: &Value) -> Ordering {
    match (a, b) {
        (Value::Str(x), Value::Str(y)) => x.as_ref().cmp(y.as_ref()),
        (Value::Bool(x), Value::Bool(y)) => x.cmp(y),
        (Value::Char(x), Value::Char(y)) => x.cmp(y),
        (Value::Float(x), Value::Float(y)) => java_double_cmp(*x as f64, *y as f64),
        (Value::Double(x), Value::Double(y)) => java_double_cmp(*x, *y),
        _ => match (a.integer_i64(), b.integer_i64()) {
            (Some(x), Some(y)) => x.cmp(&y),
            _ => Ordering::Equal,
        },
    }
}

impl Machine<'_, '_> {
    fn class_name(&self, v: &Value) -> String {
        java_class_name(v, self.program).unwrap_or_else(|| "null".into())
    }

    /// Checks that `keys` can be sorted in natural order: no Null, all of
    /// one `Comparable` class (Java throws as soon as two classes meet,
    /// which any sort of a mixed list does).
    fn check_comparable(&self, keys: &[Value]) -> Result<(), Exc> {
        if keys.len() < 2 {
            return Ok(());
        }
        let first = &keys[0];
        for k in keys {
            if matches!(k, Value::Null) {
                return Err(Exc::bare(NULL_POINTER));
            }
            let Some(c) = comparable_class(k) else {
                return Err(Exc::new(
                    CLASS_CAST,
                    format!(
                        "class {} cannot be cast to class java.lang.Comparable",
                        self.class_name(k)
                    ),
                ));
            };
            if Some(c) != comparable_class(first) {
                return Err(Exc::new(
                    CLASS_CAST,
                    format!(
                        "class {} cannot be cast to class {}",
                        self.class_name(k),
                        self.class_name(first)
                    ),
                ));
            }
        }
        Ok(())
    }

    /// The text a case-insensitive sort compares. `SortCaseInsensitive`
    /// calls `toString()` on the element, so Null raises; the `SortType`
    /// comparator uses `String.valueOf`, so a Null field is `"null"`.
    fn text_key(&self, v: &Value, null_is_text: bool) -> Result<Vec<u16>, Exc> {
        if matches!(v, Value::Null) && !null_is_text {
            return Err(Exc::bare(NULL_POINTER));
        }
        let mut out = Vec::new();
        raw_to_string(v, self.program, &mut out)?;
        Ok(out)
    }

    /// The value of field `name` of every record in `items`.
    fn field_keys(&self, items: &[Value], name: &[u16]) -> Result<Vec<Value>, Exc> {
        let name = to_rust(name);
        items
            .iter()
            .map(|item| {
                let r = match item {
                    Value::Record(r) => r,
                    Value::Null => return Err(Exc::bare(NULL_POINTER)),
                    _ => return Err(Exc::new(NO_SUCH_FIELD, name.clone())),
                };
                let def = self
                    .program
                    .records
                    .get(r.id.0 as usize)
                    .ok_or_else(|| Exc::internal("missing record"))?;
                // The spec says the field name is case-sensitive.
                let i = def
                    .fields
                    .iter()
                    .position(|f| f.name == name)
                    .ok_or_else(|| Exc::new(NO_SUCH_FIELD, name.clone()))?;
                Ok(r.fields.borrow()[i].clone())
            })
            .collect()
    }

    pub fn builtin(&mut self, b: Builtin, receiver: Option<Value>, args: Vec<Value>) -> R {
        use Builtin::*;
        let recv = || {
            receiver
                .clone()
                .ok_or_else(|| Exc::internal("missing receiver"))
        };
        let a = |i| arg(&args, i);
        let d = |i| arg(&args, i).and_then(f64_arg);
        let n = |i| arg(&args, i).and_then(int_of);
        let num = |x: f64| Ok(Value::Double(x));
        match b {
            Log => {
                self.out.log(&utf16_of(a(0)?));
                Ok(Value::Null)
            }
            LogError => {
                self.out.log_error(&utf16_of(a(0)?));
                Ok(Value::Null)
            }
            // Release mode is the reference: LogDebug prints nothing.
            LogDebug => Ok(Value::Null),
            Crlf => Ok(Value::str("\n")),
            Tab => Ok(Value::str("\t")),
            Quote => Ok(Value::str("\"")),
            Pi => num(std::f64::consts::PI),
            E => num(std::f64::consts::E),
            LastException => Ok(Value::Exc(self.last_exception.clone())),
            Not => Ok(Value::Bool(!bool_arg(a(0)?)?)),
            Abs => num(d(0)?.abs()),
            Sqrt => num(d(0)?.sqrt()),
            Power => num(java_pow(d(0)?, d(1)?)),
            Floor => num(d(0)?.floor()),
            Ceil => num(d(0)?.ceil()),
            Sin => num(d(0)?.sin()),
            Cos => num(d(0)?.cos()),
            Tan => num(d(0)?.tan()),
            // Java's `Math.asin`, `acos`, `atan` and `atan2` are fdlibm's,
            // which `libm` ports bit for bit; Rust's are the platform's.
            ASin => num(libm::asin(d(0)?)),
            ACos => num(libm::acos(d(0)?)),
            ATan => num(libm::atan(d(0)?)),
            ATan2 => num(java_atan2(d(0)?, d(1)?)),
            SinD => num((d(0)? / 180.0 * std::f64::consts::PI).sin()),
            CosD => num((d(0)? / 180.0 * std::f64::consts::PI).cos()),
            TanD => num((d(0)? / 180.0 * std::f64::consts::PI).tan()),
            ASinD => num(libm::asin(d(0)?) / std::f64::consts::PI * 180.0),
            ACosD => num(libm::acos(d(0)?) / std::f64::consts::PI * 180.0),
            ATanD => num(libm::atan(d(0)?) / std::f64::consts::PI * 180.0),
            ATan2D => num(java_atan2(d(0)?, d(1)?) / std::f64::consts::PI * 180.0),
            Logarithm => num(d(0)?.ln() / d(1)?.ln()),
            Max => num(java_max(d(0)?, d(1)?)),
            Min => num(java_min(d(0)?, d(1)?)),
            Round => Ok(Value::Long(java_round(d(0)?))),
            Round2 => {
                let shift = java_pow(10.0, n(1)? as f64);
                num(java_round(d(0)? * shift) as f64 / shift)
            }
            NumberFormat => Ok(Value::str(&number_format(d(0)?, n(1)?, n(2)?, 0, true))),
            NumberFormat2 => Ok(Value::str(&number_format(
                d(0)?,
                n(1)?,
                n(2)?,
                n(3)?,
                bool_arg(a(4)?)?,
            ))),
            IsNumber => Ok(Value::Bool(match a(0)? {
                Value::Str(s) => parse_java_double(&to_rust(s)).is_some(),
                _ => false,
            })),
            Asc => Ok(Value::Int(n(0)?)),
            Chr => Ok(Value::Char(n(0)? as u16)),
            Rnd => {
                let (min, max) = (n(0)?, n(1)?);
                let random = self.random.get_or_insert_with(|| {
                    let nanos = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map_or(0, |t| t.as_nanos() as i64);
                    JavaRandom::new(nanos)
                });
                let r = random
                    .next_int(max.wrapping_sub(min))
                    .ok_or_else(|| Exc::new(ILLEGAL_ARGUMENT, "bound must be positive"))?;
                Ok(Value::Int(min.wrapping_add(r)))
            }
            RndSeed => {
                let seed = a(0)?.integer_i64().ok_or_else(|| Exc::internal("seed"))?;
                self.random = Some(JavaRandom::new(seed));
                Ok(Value::Null)
            }
            ExitApplication => Err(Signal::Exit(0)),
            ExitApplication2 => Err(Signal::Exit(n(0)?)),
            GetType => match java_class_name(a(0)?, self.program) {
                Some(name) => Ok(Value::str(&name)),
                None => Err(Exc::bare(NULL_POINTER).into()),
            },

            BitAnd => Ok(Value::Int(n(0)? & n(1)?)),
            BitOr => Ok(Value::Int(n(0)? | n(1)?)),
            BitXor => Ok(Value::Int(n(0)? ^ n(1)?)),
            BitNot => Ok(Value::Int(!n(0)?)),
            BitShiftLeft => Ok(Value::Int(n(0)?.wrapping_shl(n(1)? as u32))),
            BitShiftRight => Ok(Value::Int(n(0)?.wrapping_shr(n(1)? as u32))),
            BitUnsignedShiftRight => {
                Ok(Value::Int((n(0)? as u32).wrapping_shr(n(1)? as u32) as i32))
            }
            BitToHexString => Ok(Value::str(&format!("{:x}", n(0)? as u32))),
            BitToBinaryString => Ok(Value::str(&format!("{:b}", n(0)? as u32))),
            BitToOctalString => Ok(Value::str(&format!("{:o}", n(0)? as u32))),
            BitParseInt => Ok(Value::Int(parse_int(str_arg(a(0)?)?, n(1)?)?)),

            StrCharAt | StrCompareTo | StrContains | StrStartsWith | StrEndsWith
            | StrEqualsIgnoreCase | StrIndexOf | StrIndexOf2 | StrLastIndexOf | StrLastIndexOf2
            | StrLength | StrReplace | StrSubString | StrSubString2 | StrToLowerCase
            | StrToUpperCase | StrTrim | StrGetBytes => {
                Ok(self.string_member(b, &recv()?, &args)?)
            }

            SbInitialize | SbIsInitialized | SbAppend | SbInsert | SbRemove | SbLength
            | SbToString => Ok(self.sb_member(b, &recv()?, &args)?),

            ListInitialize
            | ListInitialize2
            | ListIsInitialized
            | ListAdd
            | ListAddAll
            | ListAddAllAt
            | ListInsertAt
            | ListGet
            | ListSet
            | ListRemoveAt
            | ListClear
            | ListSize
            | ListIndexOf
            | ListSubList
            | ListSort
            | ListSortCaseInsensitive
            | ListSortType
            | ListSortTypeCaseInsensitive => Ok(self.list_member(b, &recv()?, &args)?),

            MapInitialize | MapIsInitialized | MapPut | MapGet | MapGetDefault | MapRemove
            | MapContainsKey | MapClear | MapSize | MapKeys | MapValues | MapGetKeyAt
            | MapGetValueAt => Ok(self.map_member(b, &recv()?, &args)?),

            ExcMessage => match recv()? {
                Value::Exc(Some(e)) => Ok(Value::str(&e.to_java_string())),
                _ => Err(Exc::not_initialized("Exception").into()),
            },
            ExcIsInitialized => Ok(Value::Bool(matches!(recv()?, Value::Exc(Some(_))))),
            ArrayLength => {
                let r = recv()?;
                let arr = as_array(&r)?;
                Ok(Value::Int(arr.items.borrow().len() as i32))
            }
            // Normally a statement-level call (see `Machine::call_step`).
            RecordInitialize => {
                Err(Exc::internal("Initialize of a record outside a statement").into())
            }
            RecordIsInitialized => match recv()? {
                Value::Record(r) => Ok(Value::Bool(r.initialized.get())),
                _ => Err(Exc::bare(NULL_POINTER).into()),
            },
        }
    }

    fn string_member(&self, b: Builtin, recv: &Value, args: &[Value]) -> Result<Value, Exc> {
        use Builtin::*;
        let s = str_arg(recv)?;
        let len = s.len();
        let sa = |i| arg(args, i).and_then(str_arg);
        let n = |i| arg(args, i).and_then(int_of);
        let bounds = |begin: i32, end: i32| -> Result<Value, Exc> {
            if begin < 0 || end < begin || end as usize > len {
                Err(Exc::new(
                    STRING_INDEX_OUT_OF_BOUNDS,
                    format!("begin {begin}, end {end}, length {len}"),
                ))
            } else {
                Ok(text(try_copy_text(&s[begin as usize..end as usize])?))
            }
        };
        Ok(match b {
            StrCharAt => {
                let i = n(0)?;
                match usize::try_from(i).ok().and_then(|i| s.get(i)) {
                    Some(&c) => Value::Char(c),
                    None => return Err(Exc::index(STRING_INDEX_OUT_OF_BOUNDS, i as i64, len)),
                }
            }
            StrCompareTo => Value::Int(compare_to(s, sa(0)?)),
            StrContains => Value::Bool(strings::index_of(s, sa(0)?, 0) >= 0),
            StrStartsWith => Value::Bool(s.starts_with(sa(0)?)),
            StrEndsWith => Value::Bool(s.ends_with(sa(0)?)),
            StrEqualsIgnoreCase => Value::Bool(match arg(args, 0)? {
                Value::Str(o) => strings::equals_ignore_case(s, o),
                _ => false,
            }),
            StrIndexOf => Value::Int(strings::index_of(s, sa(0)?, 0)),
            StrIndexOf2 => Value::Int(strings::index_of(s, sa(0)?, n(1)?)),
            StrLastIndexOf => Value::Int(strings::last_index_of(s, sa(0)?, len as i32)),
            StrLastIndexOf2 => Value::Int(strings::last_index_of(s, sa(0)?, n(1)?)),
            StrLength => Value::Int(len as i32),
            StrReplace => {
                let (target, with) = (sa(0)?, sa(1)?);
                let mut out = Vec::new();
                try_grow_text(&mut out, strings::replaced_len(s, target, with))?;
                strings::replace_into(&mut out, s, target, with);
                text(out)
            }
            StrSubString => bounds(n(0)?, len as i32)?,
            StrSubString2 => bounds(n(0)?, n(1)?)?,
            StrToLowerCase | StrToUpperCase => text(
                strings::change_case(s, b == StrToUpperCase, MAX_STRING)
                    .ok_or_else(out_of_memory)?,
            ),
            StrTrim => text(try_copy_text(strings::trim(s))?),
            StrGetBytes => {
                let charset = to_rust(sa(0)?);
                if !matches!(charset.to_ascii_uppercase().as_str(), "UTF8" | "UTF-8") {
                    return Err(Exc::new(UNSUPPORTED_ENCODING, charset));
                }
                // Encoded twice, to reserve the array before filling it.
                let mut items = Vec::new();
                try_grow(&mut items, strings::utf8_bytes(s).count())?;
                items.extend(strings::utf8_bytes(s).map(|b| Value::Byte(b as i8)));
                Value::new_array(Ty::Byte, 1, items)
            }
            _ => return Err(Exc::internal("not a String member")),
        })
    }

    fn sb_member(&self, b: Builtin, recv: &Value, args: &[Value]) -> Result<Value, Exc> {
        use Builtin::*;
        let w = match recv {
            Value::Sb(w) => w,
            Value::Null => return Err(Exc::bare(NULL_POINTER)),
            _ => return Err(Exc::internal("not a StringBuilder")),
        };
        match b {
            SbInitialize => {
                *w.borrow_mut() = Some(Rc::new(RefCell::new(Vec::new())));
                return Ok(Value::Null);
            }
            SbIsInitialized => return Ok(Value::Bool(w.borrow().is_some())),
            _ => {}
        }
        let sb = w
            .borrow()
            .clone()
            .ok_or_else(|| Exc::not_initialized("StringBuilder"))?;
        let n = |i| arg(args, i).and_then(int_of);
        let piece = |i| -> Result<Str, Exc> {
            Ok(match arg(args, i)? {
                Value::Str(s) => s.clone(),
                _ => Rc::new(to_utf16("null")),
            })
        };
        Ok(match b {
            SbAppend => {
                let p = piece(0)?;
                try_grow_text(&mut sb.borrow_mut(), p.len())?;
                sb.borrow_mut().extend_from_slice(&p);
                recv.clone()
            }
            SbInsert => {
                let (offset, p) = (n(0)?, piece(1)?);
                let len = sb.borrow().len();
                if offset < 0 || offset as usize > len {
                    return Err(Exc::new(
                        STRING_INDEX_OUT_OF_BOUNDS,
                        format!("offset {offset}, length {len}"),
                    ));
                }
                let at = offset as usize;
                try_grow_text(&mut sb.borrow_mut(), p.len())?;
                sb.borrow_mut().splice(at..at, p.iter().copied());
                recv.clone()
            }
            SbRemove => {
                let (start, end) = (n(0)?, n(1)?);
                let len = sb.borrow().len();
                let end = end.min(len as i32);
                if start < 0 || start > end {
                    return Err(Exc::new(
                        STRING_INDEX_OUT_OF_BOUNDS,
                        format!("start {start}, end {end}, length {len}"),
                    ));
                }
                sb.borrow_mut().drain(start as usize..end as usize);
                recv.clone()
            }
            SbLength => Value::Int(sb.borrow().len() as i32),
            SbToString => text(try_copy_text(&sb.borrow())?),
            _ => return Err(Exc::internal("not a StringBuilder member")),
        })
    }

    fn list_member(&self, b: Builtin, recv: &Value, args: &[Value]) -> Result<Value, Exc> {
        use Builtin::*;
        let w = match recv {
            Value::List(w) => w,
            // A Java `null` wrapper (a record field never initialised).
            Value::Null => return Err(Exc::bare(NULL_POINTER)),
            _ => return Err(Exc::internal("not a List")),
        };
        let n = |i| arg(args, i).and_then(int_of);
        let a = |i| arg(args, i);
        // The inner list of a `List` argument (an array is wrapped); an
        // uninitialised one raises, as `source.getObject()` does.
        let list_arg = |i| -> Result<ListRef, Exc> {
            let inner = match a(i)? {
                Value::List(w) => w.borrow().clone(),
                Value::Array(arr) => match array_to_list(arr) {
                    Value::List(w) => w.borrow().clone(),
                    _ => None,
                },
                Value::Null => None,
                _ => return Err(Exc::internal("expected a List")),
            };
            inner.ok_or_else(|| Exc::not_initialized("List"))
        };
        match b {
            ListInitialize => {
                *w.borrow_mut() = Some(Rc::new(ListObj::new(Vec::new(), ListKind::Growable)));
                return Ok(Value::Null);
            }
            ListInitialize2 => {
                let inner = list_arg(0)?;
                *w.borrow_mut() = Some(inner);
                return Ok(Value::Null);
            }
            ListIsInitialized => return Ok(Value::Bool(w.borrow().is_some())),
            _ => {}
        }
        let list = w
            .borrow()
            .clone()
            .ok_or_else(|| Exc::not_initialized("List"))?;
        // Any access through a sub-list whose parent changed raises here.
        let size = list.len()?;
        let unsupported = || Exc::bare(UNSUPPORTED);
        let growable = || {
            if list.kind == ListKind::Growable {
                Ok(())
            } else {
                Err(unsupported())
            }
        };
        // `ArrayList` reports `IndexOutOfBoundsException`; the fixed-size
        // `Arrays$ArrayList` indexes its array directly.
        let checked = |i: i32| -> Result<usize, Exc> {
            if i < 0 || i as usize >= size {
                let kind = if list.kind == ListKind::FixedSize {
                    ARRAY_INDEX_OUT_OF_BOUNDS
                } else {
                    INDEX_OUT_OF_BOUNDS
                };
                Err(Exc::index(kind, i as i64, size))
            } else {
                Ok(i as usize)
            }
        };
        let for_add = |i: i32| -> Result<usize, Exc> {
            if i < 0 || i as usize > size {
                Err(Exc::new(
                    INDEX_OUT_OF_BOUNDS,
                    format!("Index: {i}, Size: {size}"),
                ))
            } else {
                Ok(i as usize)
            }
        };
        let items = list.storage();
        Ok(match b {
            ListAdd => {
                growable()?;
                try_grow(&mut items.borrow_mut(), 1)?;
                items.borrow_mut().push(a(0)?.clone());
                list.modified();
                Value::Null
            }
            ListAddAll => {
                let src = list_arg(0)?.to_vec()?;
                growable()?;
                try_grow(&mut items.borrow_mut(), src.len())?;
                items.borrow_mut().extend(src);
                list.modified();
                Value::Null
            }
            ListAddAllAt => {
                let src = list_arg(1)?.to_vec()?;
                growable()?;
                let at = for_add(n(0)?)?;
                try_grow(&mut items.borrow_mut(), src.len())?;
                items.borrow_mut().splice(at..at, src);
                list.modified();
                Value::Null
            }
            ListInsertAt => {
                growable()?;
                let at = for_add(n(0)?)?;
                try_grow(&mut items.borrow_mut(), 1)?;
                items.borrow_mut().insert(at, a(1)?.clone());
                list.modified();
                Value::Null
            }
            ListGet => list
                .get(checked(n(0)?)?)?
                .ok_or_else(|| Exc::internal("list index"))?,
            ListSet => {
                if list.kind == ListKind::ReadOnly {
                    return Err(unsupported());
                }
                let i = checked(n(0)?)?;
                // A list sharing an array's storage checks the array's type.
                let value = match &list.elem {
                    Some(elem) => checked_store(elem, a(1)?.clone(), self.program)?,
                    None => a(1)?.clone(),
                };
                items.borrow_mut()[i] = value;
                Value::Null
            }
            ListRemoveAt => {
                growable()?;
                let i = checked(n(0)?)?;
                items.borrow_mut().remove(i);
                list.modified();
                Value::Null
            }
            ListClear => {
                match list.kind {
                    // `Collections.unmodifiableList.clear` always throws.
                    ListKind::ReadOnly => return Err(unsupported()),
                    // `Arrays.asList(...).clear()` only fails on a removal.
                    ListKind::FixedSize if size > 0 => return Err(unsupported()),
                    ListKind::FixedSize => {}
                    ListKind::Growable => {
                        items.borrow_mut().clear();
                        list.modified();
                    }
                }
                Value::Null
            }
            ListSize => Value::Int(size as i32),
            ListIndexOf => {
                let item = a(0)?;
                let mut found = -1;
                for (i, x) in list.to_vec()?.iter().enumerate() {
                    if java_equals(x, item)? {
                        found = i as i32;
                        break;
                    }
                }
                Value::Int(found)
            }
            ListSubList => {
                let (from, to) = (n(0)?, n(1)?);
                if from < 0 {
                    return Err(Exc::new(INDEX_OUT_OF_BOUNDS, format!("fromIndex = {from}")));
                }
                if to as i64 > size as i64 {
                    return Err(Exc::new(INDEX_OUT_OF_BOUNDS, format!("toIndex = {to}")));
                }
                if from > to {
                    return Err(Exc::new(
                        ILLEGAL_ARGUMENT,
                        format!("fromIndex({from}) > toIndex({to})"),
                    ));
                }
                // `Collections.unmodifiableList(subList(from, to))`: a live,
                // read-only view of this list.
                let view = ListObj::view(&list, from as usize, to as usize);
                Value::List(Rc::new(RefCell::new(Some(Rc::new(view)))))
            }
            ListSort | ListSortCaseInsensitive | ListSortType | ListSortTypeCaseInsensitive => {
                let by_field = matches!(b, ListSortType | ListSortTypeCaseInsensitive);
                // `SortType` returns early on an empty list; `Sort` calls
                // `Collections.sort`, which an unmodifiable list refuses.
                if by_field && size == 0 {
                    return Ok(Value::Null);
                }
                if list.kind == ListKind::ReadOnly {
                    return Err(unsupported());
                }
                let values = list.to_vec()?;
                let (keys, asc) = if by_field {
                    (self.field_keys(&values, str_arg(a(0)?)?)?, bool_arg(a(1)?)?)
                } else {
                    (values.clone(), bool_arg(a(0)?)?)
                };
                let mut order: Vec<usize> = (0..values.len()).collect();
                if matches!(b, ListSort | ListSortType) {
                    self.check_comparable(&keys)?;
                    order.sort_by(|&x, &y| {
                        let c = natural_cmp(&keys[x], &keys[y]);
                        if asc { c } else { c.reverse() }
                    });
                } else {
                    let texts: Vec<Vec<u16>> = if keys.len() < 2 {
                        Vec::new()
                    } else {
                        keys.iter()
                            .map(|k| self.text_key(k, by_field))
                            .collect::<Result<_, _>>()?
                    };
                    order.sort_by(|&x, &y| {
                        let c = strings::ordering(compare_ignore_case(&texts[x], &texts[y]));
                        if asc { c } else { c.reverse() }
                    });
                }
                let sorted: Vec<Value> = order.into_iter().map(|i| values[i].clone()).collect();
                *items.borrow_mut() = sorted;
                // `ArrayList.sort` counts as a structural modification.
                if list.kind == ListKind::Growable {
                    list.modified();
                }
                Value::Null
            }
            _ => return Err(Exc::internal("not a List member")),
        })
    }

    fn map_member(&self, b: Builtin, recv: &Value, args: &[Value]) -> Result<Value, Exc> {
        use Builtin::*;
        let w = match recv {
            Value::Map(w) => w,
            Value::Null => return Err(Exc::bare(NULL_POINTER)),
            _ => return Err(Exc::internal("not a Map")),
        };
        match b {
            MapInitialize => {
                *w.borrow_mut() = Some(Rc::new(RefCell::new(OrderedMap::default())));
                return Ok(Value::Null);
            }
            MapIsInitialized => return Ok(Value::Bool(w.borrow().is_some())),
            _ => {}
        }
        let map = w
            .borrow()
            .clone()
            .ok_or_else(|| Exc::not_initialized("Map"))?;
        let a = |i| arg(args, i);
        let positional = |keys: bool| -> Result<Value, Exc> {
            let i = int_of(a(0)?)?;
            let size = map.borrow().len();
            let entry = usize::try_from(i)
                .ok()
                .and_then(|i| map.borrow_mut().entry_at(i));
            let (k, v) = entry.ok_or_else(|| {
                Exc::new(INDEX_OUT_OF_BOUNDS, format!("Index: {i}, Size: {size}"))
            })?;
            Ok(if keys { k } else { v })
        };
        Ok(match b {
            MapPut => {
                let (k, v) = (a(0)?.clone(), a(1)?.clone());
                map.borrow_mut().put(k, v)?.unwrap_or(Value::Null)
            }
            MapGet => map.borrow().get(a(0)?).cloned().unwrap_or(Value::Null),
            MapGetDefault => match map.borrow().get(a(0)?) {
                Some(v) if !matches!(v, Value::Null) => v.clone(),
                _ => a(1)?.clone(),
            },
            MapRemove => map.borrow_mut().remove(a(0)?).unwrap_or(Value::Null),
            MapContainsKey => Value::Bool(map.borrow().contains(a(0)?)),
            MapClear => {
                map.borrow_mut().clear();
                Value::Null
            }
            MapSize => Value::Int(map.borrow().len() as i32),
            MapKeys => Value::View(map.clone(), true),
            MapValues => Value::View(map.clone(), false),
            MapGetKeyAt => positional(true)?,
            MapGetValueAt => positional(false)?,
            _ => return Err(Exc::internal("not a Map member")),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn java_math() {
        assert_eq!(java_round(2.5), 3);
        assert_eq!(java_round(-2.5), -2);
        assert_eq!(java_round(0.49999999999999994), 0);
        assert_eq!(java_round(f64::NAN), 0);
        assert_eq!(java_round(1e300), i64::MAX);
        assert!(java_max(f64::NAN, 1.0).is_nan());
        assert!(java_max(-0.0, 0.0).is_sign_positive());
        assert!(java_min(-0.0, 0.0).is_sign_negative());
        assert!(java_pow(1.0, f64::NAN).is_nan());
        assert_eq!(java_pow(2.0, 10.0), 1024.0);
        assert_eq!(java_double_cmp(f64::NAN, f64::INFINITY), Ordering::Greater);
        assert_eq!(java_double_cmp(-0.0, 0.0), Ordering::Less);
    }

    #[test]
    fn integer_parse_int() {
        let p = |s: &str, r| parse_int(&to_utf16(s), r);
        assert_eq!(p("ff", 16), Ok(255));
        assert_eq!(p("-101", 2), Ok(-5));
        assert_eq!(p("-2147483648", 10), Ok(i32::MIN));
        assert_eq!(p("2147483648", 10).unwrap_err().kind, NUMBER_FORMAT);
        assert_eq!(
            p("xyz", 16).unwrap_err().message.as_deref(),
            Some("For input string: \"xyz\" under radix 16")
        );
        assert!(p("", 10).is_err());
        assert!(p("+", 10).is_err());
    }

    /// `tests/data/java_trig.tsv`: Java's own results, bit for bit.
    #[test]
    fn inverse_trigonometry_matches_jdk_21() {
        let data = include_str!("../tests/data/java_trig.tsv");
        let mut rows = 0;
        for line in data.lines().filter(|l| !l.starts_with('#')) {
            let v: Vec<f64> = line
                .split(' ')
                .map(|h| f64::from_bits(u64::from_str_radix(h, 16).unwrap()))
                .collect();
            let (x, y) = (v[0], v[1]);
            let got = [
                libm::atan(x),
                libm::asin(x),
                libm::acos(x),
                java_atan2(x, y),
            ];
            for (got, want) in got.into_iter().zip(&v[2..]) {
                assert_eq!(got.to_bits(), want.to_bits(), "{line}");
            }
            rows += 1;
        }
        assert!(rows > 900);
    }
}
