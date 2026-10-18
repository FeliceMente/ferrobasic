//! Default values and the conversions of `docs/spec/level0.md`.

use std::cell::RefCell;
use std::rc::Rc;

use ferro_mir::{Program, Ty};

use crate::number::{number_to_string, parse_java_double};
use crate::strings::to_rust;
use crate::value::*;

/// `Rvalue::Default`: 0, `False`, `""`, `Chr(0)`, `Null`, a fresh
/// uninitialised wrapper, an empty array, or a record whose fields hold
/// Java defaults (see [`java_default`]) with `IsInitialized` false.
pub fn default_value(ty: &Ty, program: &Program) -> Value {
    match ty {
        Ty::String => Value::Str(Rc::from(Vec::new())),
        Ty::List | Ty::Map | Ty::StringBuilder | Ty::Exception => fresh_wrapper(ty),
        // The contract says "an empty array of rank 1"; an array type of a
        // higher rank gets an empty array with as many (zero) dimensions,
        // so that indexing it reports an index error, not a rank mismatch.
        Ty::Array(elem, rank) => Value::Array(Rc::new(ArrayObj {
            elem: (**elem).clone(),
            dims: vec![0; (*rank).max(1) as usize],
            items: Rc::new(RefCell::new(Vec::new())),
        })),
        // B4J records are Java objects: `Dim p As T` creates one whose
        // fields hold Java defaults; `Initialize` (the record's `init`
        // function) then gives them their B4X defaults.
        Ty::Record(id) => Value::Record(Rc::new(RecordObj {
            id: *id,
            initialized: std::cell::Cell::new(false),
            fields: RefCell::new(
                program
                    .records
                    .get(id.0 as usize)
                    .map(|r| r.fields.iter().map(|f| java_default(&f.ty)).collect())
                    .unwrap_or_default(),
            ),
        })),
        _ => java_default(ty),
    }
}

/// A Java field's default: 0, `false`, `'\0'` for primitives, `null` for
/// every reference type (String, Object, wrappers, arrays, records).
pub fn java_default(ty: &Ty) -> Value {
    match ty {
        Ty::Boolean => Value::Bool(false),
        Ty::Byte => Value::Byte(0),
        Ty::Short => Value::Short(0),
        Ty::Int => Value::Int(0),
        Ty::Long => Value::Long(0),
        Ty::Float => Value::Float(0.0),
        Ty::Double => Value::Double(0.0),
        Ty::Char => Value::Char(0),
        _ => Value::Null,
    }
}

/// A new uninitialised `List`/`Map`/`StringBuilder`/`Exception` wrapper.
pub fn fresh_wrapper(ty: &Ty) -> Value {
    match ty {
        Ty::List => Value::List(Rc::new(RefCell::new(None))),
        Ty::Map => Value::Map(Rc::new(RefCell::new(None))),
        Ty::StringBuilder => Value::Sb(Rc::new(RefCell::new(None))),
        Ty::Exception => Value::Exc(None),
        _ => Value::Null,
    }
}

/// Normalises a value stored into a slot of type `ty`: `Null` in a
/// `List`/`Map`/`StringBuilder`/`Exception` slot is an uninitialised
/// wrapper (B4J wrappers hold `null`), and a number of another type is
/// converted (constants are only Int, Long and Double).
pub fn coerce(ty: &Ty, v: Value) -> Value {
    match (ty, &v) {
        (Ty::List | Ty::Map | Ty::StringBuilder | Ty::Exception, Value::Null) => fresh_wrapper(ty),
        (Ty::Byte, Value::Byte(_))
        | (Ty::Short, Value::Short(_))
        | (Ty::Int, Value::Int(_))
        | (Ty::Long, Value::Long(_))
        | (Ty::Float, Value::Float(_))
        | (Ty::Double, Value::Double(_)) => v,
        (t, _) if t.is_numeric() && v.is_number() => numeric_cast(&v, t).unwrap_or(v),
        _ => v,
    }
}

/// Java primitive widening and narrowing between numeric types. Narrowing
/// from a floating type goes through `int` (`(byte) d` is `(byte)(int) d`),
/// saturating, with NaN → 0; integer narrowing keeps the low bits.
pub fn numeric_cast(v: &Value, to: &Ty) -> Option<Value> {
    enum N {
        I(i64),
        F(f32),
        D(f64),
    }
    let n = match *v {
        Value::Byte(x) => N::I(x as i64),
        Value::Short(x) => N::I(x as i64),
        Value::Int(x) => N::I(x as i64),
        Value::Long(x) => N::I(x),
        Value::Char(x) => N::I(x as i64),
        Value::Float(x) => N::F(x),
        Value::Double(x) => N::D(x),
        _ => return None,
    };
    let as_int = |n: &N| -> i64 {
        match *n {
            N::I(i) => i,
            N::F(f) => (f as i32) as i64,
            N::D(d) => (d as i32) as i64,
        }
    };
    Some(match to {
        Ty::Byte => Value::Byte(as_int(&n) as i8),
        Ty::Short => Value::Short(as_int(&n) as i16),
        Ty::Int => Value::Int(as_int(&n) as i32),
        Ty::Char => Value::Char(as_int(&n) as u16),
        Ty::Long => Value::Long(match n {
            N::I(i) => i,
            N::F(f) => f as i64,
            N::D(d) => d as i64,
        }),
        Ty::Float => Value::Float(match n {
            N::I(i) => i as f32,
            N::F(f) => f,
            N::D(d) => d as f32,
        }),
        Ty::Double => Value::Double(match n {
            N::I(i) => i as f64,
            N::F(f) => f as f64,
            N::D(d) => d,
        }),
        _ => return None,
    })
}

fn class_cast(v: &Value, to: &str, program: &Program) -> Exc {
    let from = java_class_name(v, program).unwrap_or_else(|| "null".into());
    Exc::new(
        CLASS_CAST,
        format!("class {from} cannot be cast to class {to}"),
    )
}

/// `Double.parseDouble` of a String value; `NumberFormatException` with the
/// JDK message when it fails.
pub fn parse_number(v: &Value) -> Result<f64, Exc> {
    match v {
        Value::Str(s) => {
            let text = to_rust(s);
            parse_java_double(&text).ok_or_else(|| {
                if text.trim_matches(|c: char| c <= ' ').is_empty() {
                    Exc::new(NUMBER_FORMAT, "empty String")
                } else {
                    Exc::new(NUMBER_FORMAT, format!("For input string: \"{text}\""))
                }
            })
        }
        _ => Err(Exc::bare(NULL_POINTER)),
    }
}

/// `Value::Str` text of a number (`BA.NumberToString`).
pub fn number_text(v: &Value) -> Option<String> {
    Some(match *v {
        Value::Byte(x) => x.to_string(),
        Value::Short(x) => x.to_string(),
        Value::Int(x) => x.to_string(),
        Value::Long(x) => x.to_string(),
        Value::Float(x) => number_to_string(x as f64),
        Value::Double(x) => number_to_string(x),
        _ => return None,
    })
}

/// Any value converted to String, given its static type: numbers through
/// `NumberToString`, wrappers through their `toString`, everything else
/// through `String.valueOf`.
pub fn to_string_from(v: &Value, from: &Ty, program: &Program) -> Result<Value, Exc> {
    if let Value::Str(_) = v {
        return Ok(v.clone());
    }
    if from.is_numeric()
        && let Some(s) = number_text(v)
    {
        return Ok(Value::str(&s));
    }
    let mut out = Vec::new();
    match (from, v) {
        // A Null String or the Null literal stays a null String.
        (Ty::String | Ty::Null, Value::Null) => return Ok(Value::Null),
        (t, _) if is_wrapper_type(t) => wrapper_to_string(v, program, &mut out)?,
        _ => raw_to_string(v, program, &mut out)?,
    }
    Ok(Value::from_utf16(out))
}

/// Whether a Java array of `src_rank` dimensions over `src` can be viewed as
/// one of `dst_rank` over `dst`: the same type, or reference-array
/// covariance (`String()` is an `Object()`, `Int()` is not, and `Int(,)`
/// is not an `Int()` but is an `Object()`, its elements being arrays).
pub fn array_assignable(src: &Ty, src_rank: usize, dst: &Ty, dst_rank: usize) -> bool {
    let is_ref = |t: &Ty, rank: usize| rank > 0 || t.is_reference();
    if src_rank == dst_rank && src == dst {
        return true;
    }
    if dst_rank == 0 {
        return matches!(dst, Ty::Object) && is_ref(src, src_rank);
    }
    if src_rank == 0 {
        return false;
    }
    is_ref(src, src_rank - 1)
        && is_ref(dst, dst_rank - 1)
        && array_assignable(src, src_rank - 1, dst, dst_rank - 1)
}

/// A value stored into an array whose runtime component type is `elem`:
/// numbers are converted, and a value of another type raises
/// `java.lang.ArrayStoreException` (the array may be a `String()` seen as an
/// `Object()`).
pub fn checked_store(elem: &Ty, v: Value, program: &Program) -> Result<Value, Exc> {
    let ok = match (elem, &v) {
        (Ty::Object | Ty::Iterable | Ty::Error | Ty::Void | Ty::Null, _) => true,
        (t, Value::Null) => t.is_reference(),
        (t, n) if t.is_numeric() => n.is_number(),
        (Ty::Boolean, Value::Bool(_))
        | (Ty::Char, Value::Char(_))
        | (Ty::String, Value::Str(_))
        | (Ty::List, Value::List(_))
        | (Ty::Map, Value::Map(_))
        | (Ty::StringBuilder, Value::Sb(_))
        | (Ty::Exception, Value::Exc(_)) => true,
        (Ty::Record(id), Value::Record(r)) => r.id == *id,
        (Ty::Array(e, rank), Value::Array(a)) => {
            array_assignable(&a.elem, a.dims.len(), e, *rank as usize)
        }
        _ => false,
    };
    if ok {
        Ok(coerce(elem, v))
    } else {
        let class = java_class_name(&v, program).unwrap_or_else(|| "null".into());
        Err(Exc::new(ARRAY_STORE, class))
    }
}

/// `Rvalue::Convert`.
pub fn convert(v: Value, from: &Ty, to: &Ty, program: &Program) -> Result<Value, Exc> {
    if from == to {
        return Ok(v);
    }
    match to {
        Ty::Byte | Ty::Short | Ty::Int | Ty::Long | Ty::Float | Ty::Double => {
            to_number(v, from, to, program)
        }
        Ty::Boolean => match v {
            Value::Bool(_) => Ok(v),
            Value::Str(ref s) => match to_rust(s).as_str() {
                "true" => Ok(Value::Bool(true)),
                "false" => Ok(Value::Bool(false)),
                other => Err(Exc::new(
                    RUNTIME,
                    format!("Cannot parse: {other} as boolean"),
                )),
            },
            Value::Null => Err(Exc::bare(NULL_POINTER)),
            _ => Err(class_cast(&v, "java.lang.Boolean", program)),
        },
        Ty::Char => match v {
            Value::Char(_) => Ok(v),
            Value::Str(ref s) => Ok(Value::Char(s.first().copied().unwrap_or(0))),
            Value::Null => Err(Exc::bare(NULL_POINTER)),
            ref n if n.is_number() && from.is_numeric() => {
                numeric_cast(n, &Ty::Char).ok_or_else(|| Exc::internal("char"))
            }
            _ => Err(class_cast(&v, "java.lang.Character", program)),
        },
        Ty::String => to_string_from(&v, from, program),
        Ty::Object => Ok(unwrap(v)),
        Ty::List | Ty::Map | Ty::StringBuilder | Ty::Exception => rewrap(v, to, program),
        Ty::Array(elem, rank) => match &v {
            Value::Null => Ok(v),
            Value::Array(a) if array_assignable(&a.elem, a.dims.len(), elem, *rank as usize) => {
                Ok(v)
            }
            _ => Err(class_cast(&v, &array_class_name(elem, *rank), program)),
        },
        Ty::Record(id) => match &v {
            Value::Null => Ok(v),
            Value::Record(r) if r.id == *id => Ok(v),
            _ => {
                let name = program
                    .records
                    .get(id.0 as usize)
                    .map_or("record", |r| r.name.as_str());
                Err(class_cast(&v, name, program))
            }
        },
        Ty::Iterable | Ty::Void | Ty::Null | Ty::Error => Ok(v),
    }
}

fn to_number(v: Value, from: &Ty, to: &Ty, program: &Program) -> Result<Value, Exc> {
    let cast = |v: &Value| numeric_cast(v, to).ok_or_else(|| Exc::internal("numeric cast"));
    match &v {
        // Object → Long keeps an integer box exact (`ObjectToLongNumber`);
        // other unboxings go through `doubleValue()` (`ObjectToNumber`).
        n if n.is_number() => {
            if from.is_numeric() || matches!(to, Ty::Double) {
                cast(n)
            } else if let (Ty::Long, Some(i)) = (to, n.integer_i64()) {
                Ok(Value::Long(i))
            } else {
                cast(&Value::Double(n.number_f64().unwrap_or(0.0)))
            }
        }
        // Java widening of a char (not reachable from Level 0 type checking).
        Value::Char(_) if !matches!(from, Ty::Object) => cast(&v),
        Value::Str(_) => cast(&Value::Double(parse_number(&v)?)),
        Value::Null => Err(Exc::bare(NULL_POINTER)),
        _ => Err(class_cast(&v, "java.lang.Number", program)),
    }
}

/// A wrapper converted to `Object` is unwrapped: an uninitialised one
/// becomes `Null`, an initialised one a new wrapper sharing the inner
/// object (later `Initialize` calls on the variable do not affect it).
pub fn unwrap(v: Value) -> Value {
    match v {
        Value::List(w) => match w.borrow().clone() {
            Some(l) => Value::List(Rc::new(RefCell::new(Some(l)))),
            None => Value::Null,
        },
        Value::Map(w) => match w.borrow().clone() {
            Some(m) => Value::Map(Rc::new(RefCell::new(Some(m)))),
            None => Value::Null,
        },
        Value::Sb(w) => match w.borrow().clone() {
            Some(s) => Value::Sb(Rc::new(RefCell::new(Some(s)))),
            None => Value::Null,
        },
        Value::Exc(None) => Value::Null,
        other => other,
    }
}

/// `Object` (or another value) to a wrapper type.
fn rewrap(v: Value, to: &Ty, program: &Program) -> Result<Value, Exc> {
    match (to, &v) {
        (_, Value::Null) => Ok(fresh_wrapper(to)),
        (Ty::List, Value::List(_))
        | (Ty::Map, Value::Map(_))
        | (Ty::StringBuilder, Value::Sb(_))
        | (Ty::Exception, Value::Exc(_)) => Ok(match unwrap(v) {
            Value::Null => fresh_wrapper(to),
            w => w,
        }),
        (Ty::List, Value::Array(a)) => Ok(array_to_list(a)),
        (Ty::List, _) => Err(class_cast(&v, "java.util.List", program)),
        (Ty::Map, _) => Err(class_cast(&v, "java.util.Map", program)),
        (Ty::StringBuilder, _) => Err(class_cast(&v, "java.lang.StringBuilder", program)),
        _ => Err(class_cast(&v, "java.lang.Throwable", program)),
    }
}

/// An array passed as a `List` (`ArrayToList`): a fixed-size list. For an
/// array of references it shares the array's storage (`Arrays.asList`); for
/// an array of primitives the elements are copied into boxes.
pub fn array_to_list(a: &Rc<ArrayObj>) -> Value {
    let list = if a.elem.is_primitive() || a.dims.len() > 1 {
        ListObj::new(a.items.borrow().clone(), ListKind::FixedSize)
    } else {
        // Stores through the list are checked against the array's type.
        ListObj::shared(a.items.clone(), ListKind::FixedSize, Some(a.elem.clone()))
    };
    Value::List(Rc::new(RefCell::new(Some(Rc::new(list)))))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn program() -> Program {
        Program {
            records: vec![],
            globals: vec![],
            functions: vec![],
            init: ferro_mir::FuncId(0),
            entry: ferro_mir::FuncId(0),
        }
    }

    fn conv(v: Value, from: Ty, to: Ty) -> Result<Value, Exc> {
        convert(v, &from, &to, &program())
    }

    fn text(v: Result<Value, Exc>) -> String {
        text_of(&v.unwrap())
    }

    #[test]
    fn numeric_conversions_follow_java() {
        let d = |x: f64, to: Ty| conv(Value::Double(x), Ty::Double, to).unwrap();
        assert!(matches!(d(5.9, Ty::Int), Value::Int(5)));
        assert!(matches!(d(-5.9, Ty::Int), Value::Int(-5)));
        assert!(matches!(d(f64::NAN, Ty::Int), Value::Int(0)));
        assert!(matches!(d(1e20, Ty::Int), Value::Int(i32::MAX)));
        assert!(matches!(d(-1e20, Ty::Long), Value::Long(i64::MIN)));
        // (short) 1e10 is (short)(int) 1e10 = (short) 0x7fffffff = -1.
        assert!(matches!(d(1e10, Ty::Short), Value::Short(-1)));
        assert!(matches!(d(300.0, Ty::Byte), Value::Byte(44)));
        let l = conv(Value::Long(0x1_0000_0005), Ty::Long, Ty::Int).unwrap();
        assert!(matches!(l, Value::Int(5)));
        let f = conv(Value::Int(16_777_217), Ty::Int, Ty::Float).unwrap();
        assert!(matches!(f, Value::Float(x) if x == 16_777_216.0));
        let w = conv(Value::Long(9_007_199_254_740_993), Ty::Long, Ty::Double).unwrap();
        assert!(matches!(w, Value::Double(x) if x == 9_007_199_254_740_992.0));
    }

    #[test]
    fn string_conversions() {
        assert!(matches!(
            conv(Value::str(" 12.7 "), Ty::String, Ty::Int).unwrap(),
            Value::Int(12)
        ));
        assert!(matches!(
            conv(Value::str("1e3"), Ty::String, Ty::Double).unwrap(),
            Value::Double(x) if x == 1000.0
        ));
        let e = conv(Value::str("abc"), Ty::String, Ty::Int).unwrap_err();
        assert_eq!(e.kind, NUMBER_FORMAT);
        assert_eq!(
            e.to_java_string(),
            "java.lang.NumberFormatException: For input string: \"abc\""
        );
        let e = conv(Value::str(""), Ty::String, Ty::Double).unwrap_err();
        assert_eq!(e.message.as_deref(), Some("empty String"));
        assert!(matches!(
            conv(Value::str("true"), Ty::String, Ty::Boolean).unwrap(),
            Value::Bool(true)
        ));
        assert_eq!(
            conv(Value::str("True"), Ty::String, Ty::Boolean)
                .unwrap_err()
                .kind,
            RUNTIME
        );
        assert!(matches!(
            conv(Value::str("xy"), Ty::String, Ty::Char).unwrap(),
            Value::Char(0x78)
        ));
        assert!(matches!(
            conv(Value::str(""), Ty::String, Ty::Char).unwrap(),
            Value::Char(0)
        ));
        assert_eq!(text(conv(Value::Double(3.0), Ty::Double, Ty::String)), "3");
        assert_eq!(
            text(conv(Value::Float(1.1), Ty::Float, Ty::String)),
            "1.100000023841858"
        );
        assert_eq!(text(conv(Value::Int(-42), Ty::Int, Ty::String)), "-42");
        assert_eq!(
            text(conv(Value::Bool(true), Ty::Boolean, Ty::String)),
            "true"
        );
        assert_eq!(text(conv(Value::Char(65), Ty::Char, Ty::String)), "A");
        assert_eq!(text(conv(Value::Null, Ty::Object, Ty::String)), "null");
        // An Object holding a Double prints with Java's `Double.toString`.
        assert_eq!(
            text(conv(Value::Double(1.0), Ty::Object, Ty::String)),
            "1.0"
        );
    }

    #[test]
    fn object_unboxing() {
        let o = |v: Value, to: Ty| conv(v, Ty::Object, to);
        assert!(matches!(o(Value::Int(7), Ty::Double).unwrap(), Value::Double(x) if x == 7.0));
        assert!(matches!(
            o(Value::Double(7.9), Ty::Int).unwrap(),
            Value::Int(7)
        ));
        assert!(matches!(
            o(Value::Long(i64::MAX), Ty::Long).unwrap(),
            Value::Long(i64::MAX)
        ));
        assert!(matches!(
            o(Value::str("5"), Ty::Int).unwrap(),
            Value::Int(5)
        ));
        assert_eq!(o(Value::Bool(true), Ty::Int).unwrap_err().kind, CLASS_CAST);
        assert_eq!(o(Value::Int(1), Ty::Boolean).unwrap_err().kind, CLASS_CAST);
        assert_eq!(o(Value::Int(1), Ty::List).unwrap_err().kind, CLASS_CAST);
        assert_eq!(o(Value::str("x"), Ty::Map).unwrap_err().kind, CLASS_CAST);
        assert!(
            matches!(o(Value::Null, Ty::List).unwrap(), Value::List(w) if w.borrow().is_none())
        );
    }

    #[test]
    fn collections_to_string() {
        let p = program();
        let list = Value::new_list(vec![Value::Int(1), Value::Double(2.0)], ListKind::Growable);
        assert_eq!(
            text_of(&to_string_from(&list, &Ty::List, &p).unwrap()),
            "(ArrayList) [1, 2.0]"
        );
        assert_eq!(
            text_of(&to_string_from(&list, &Ty::Object, &p).unwrap()),
            "[1, 2.0]"
        );
        let mut m = OrderedMap::default();
        m.put(Value::str("a"), Value::Int(1));
        m.put(Value::str("b"), Value::Int(2));
        let map = Value::new_map(m);
        assert_eq!(
            text_of(&to_string_from(&map, &Ty::Map, &p).unwrap()),
            "(MyMap) {a=1, b=2}"
        );
        let uninit = fresh_wrapper(&Ty::List);
        assert_eq!(
            text_of(&to_string_from(&uninit, &Ty::List, &p).unwrap()),
            "(List) Not initialized"
        );
        let uninit = fresh_wrapper(&Ty::Map);
        assert_eq!(
            text_of(&to_string_from(&uninit, &Ty::Map, &p).unwrap()),
            "(Map) Not initialized"
        );
        // Converting an uninitialised list to Object gives Null.
        assert!(matches!(unwrap(fresh_wrapper(&Ty::List)), Value::Null));
    }
}
