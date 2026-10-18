//! Level 0 types and the implicit conversions between them
//! (`docs/spec/level0.md`, "Types" and "Conversions").

use std::fmt;

/// Index of a `Type` record declaration in [`crate::Module::records`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RecordId(pub u32);

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub enum Ty {
    Boolean,
    Byte,
    Short,
    Int,
    Long,
    Float,
    Double,
    Char,
    String,
    Object,
    List,
    Map,
    StringBuilder,
    /// The type of `LastException`.
    Exception,
    /// What `Map.Keys` and `Map.Values` return: only usable in `For Each`.
    Iterable,
    /// An array with element type and rank (`Dim m(3, 4) As Double` has rank 2).
    Array(Box<Ty>, u8),
    Record(RecordId),
    /// The result of a Sub with no return type.
    #[default]
    Void,
    /// The type of `Null` before it is converted.
    Null,
    /// An expression that already produced a diagnostic; converts to anything.
    Error,
}

impl Ty {
    pub fn is_integer(&self) -> bool {
        matches!(self, Ty::Byte | Ty::Short | Ty::Int | Ty::Long)
    }

    pub fn is_floating(&self) -> bool {
        matches!(self, Ty::Float | Ty::Double)
    }

    pub fn is_numeric(&self) -> bool {
        self.is_integer() || self.is_floating()
    }

    /// Copied on assignment (numbers, Boolean, Char).
    pub fn is_primitive(&self) -> bool {
        self.is_numeric() || matches!(self, Ty::Boolean | Ty::Char)
    }

    /// Can hold `Null`.
    pub fn is_reference(&self) -> bool {
        !self.is_primitive() && !matches!(self, Ty::Void | Ty::Error)
    }

    /// Result type of `+ - *` (and `Mod`): Byte and Short widen to Int, two
    /// integers give the wider of Int and Long, anything else gives Double.
    pub fn arithmetic(left: &Ty, right: &Ty) -> Ty {
        if *left == Ty::Error || *right == Ty::Error {
            Ty::Error
        } else if left.is_integer() && right.is_integer() {
            if *left == Ty::Long || *right == Ty::Long {
                Ty::Long
            } else {
                Ty::Int
            }
        } else {
            Ty::Double
        }
    }

    /// The type a unary minus or arithmetic operand is promoted to on its own.
    pub fn promoted(&self) -> Ty {
        match self {
            Ty::Byte | Ty::Short | Ty::Int => Ty::Int,
            Ty::Long => Ty::Long,
            Ty::Error => Ty::Error,
            _ => Ty::Double,
        }
    }
}

/// Whether a value of type `from` may be implicitly converted to `to`.
/// Conversions that can fail at run time (String → Int, Object → List) are
/// allowed here, as in B4X; only conversions no value can satisfy are refused.
pub fn can_convert(from: &Ty, to: &Ty) -> bool {
    use Ty::*;
    if from == to || matches!(from, Error) || matches!(to, Error) {
        return true;
    }
    match (from, to) {
        (Void, _) | (_, Void) | (Iterable, _) | (_, Iterable) | (_, Null) => false,
        (_, Object) | (Object, _) => true,
        (Null, to) => to.is_reference(),
        (f, t) if f.is_numeric() && t.is_numeric() => true,
        // Any value converts to String (`BA.ObjectToString` for objects:
        // B4X compiles `Log(LastException)` and `"e: " & LastException`).
        (_, String) => true,
        (String, t) => t.is_numeric() || matches!(t, Boolean | Char),
        // An array passed where a List is expected is wrapped (`List.AddAll`,
        // `Initialize2`).
        (Array(..), List) => true,
        _ => false,
    }
}

impl fmt::Display for Ty {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Ty::Boolean => f.write_str("Boolean"),
            Ty::Byte => f.write_str("Byte"),
            Ty::Short => f.write_str("Short"),
            Ty::Int => f.write_str("Int"),
            Ty::Long => f.write_str("Long"),
            Ty::Float => f.write_str("Float"),
            Ty::Double => f.write_str("Double"),
            Ty::Char => f.write_str("Char"),
            Ty::String => f.write_str("String"),
            Ty::Object => f.write_str("Object"),
            Ty::List => f.write_str("List"),
            Ty::Map => f.write_str("Map"),
            Ty::StringBuilder => f.write_str("StringBuilder"),
            Ty::Exception => f.write_str("Exception"),
            Ty::Iterable => f.write_str("Map keys/values"),
            Ty::Array(elem, rank) => {
                write!(f, "{elem}(")?;
                for _ in 1..*rank {
                    f.write_str(",")?;
                }
                f.write_str(")")
            }
            Ty::Record(id) => write!(f, "Type #{}", id.0),
            Ty::Void => f.write_str("nothing"),
            Ty::Null => f.write_str("Null"),
            Ty::Error => f.write_str("{unknown}"),
        }
    }
}

/// The built-in type spelled `name` (case-insensitive).
pub fn builtin_type(name: &str) -> Option<Ty> {
    const TYPES: &[(&str, Ty)] = &[
        ("boolean", Ty::Boolean),
        ("byte", Ty::Byte),
        ("short", Ty::Short),
        ("int", Ty::Int),
        ("long", Ty::Long),
        ("float", Ty::Float),
        ("double", Ty::Double),
        ("char", Ty::Char),
        ("string", Ty::String),
        ("object", Ty::Object),
        ("list", Ty::List),
        ("map", Ty::Map),
        ("stringbuilder", Ty::StringBuilder),
        ("exception", Ty::Exception),
    ];
    TYPES
        .iter()
        .find(|(n, _)| n.eq_ignore_ascii_case(name))
        .map(|(_, t)| t.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arithmetic_result_types() {
        assert_eq!(Ty::arithmetic(&Ty::Byte, &Ty::Short), Ty::Int);
        assert_eq!(Ty::arithmetic(&Ty::Int, &Ty::Long), Ty::Long);
        assert_eq!(Ty::arithmetic(&Ty::Int, &Ty::Float), Ty::Double);
        assert_eq!(Ty::arithmetic(&Ty::String, &Ty::Int), Ty::Double);
    }

    #[test]
    fn conversions() {
        assert!(can_convert(&Ty::Int, &Ty::Double));
        assert!(can_convert(&Ty::Double, &Ty::Int));
        assert!(can_convert(&Ty::Int, &Ty::String));
        assert!(can_convert(&Ty::String, &Ty::Int));
        assert!(can_convert(&Ty::Object, &Ty::Map));
        assert!(can_convert(&Ty::Null, &Ty::List));
        assert!(can_convert(&Ty::Array(Box::new(Ty::Int), 1), &Ty::List));
        assert!(!can_convert(&Ty::Null, &Ty::Int));
        assert!(!can_convert(&Ty::Boolean, &Ty::Int));
        assert!(!can_convert(&Ty::List, &Ty::Map));
        assert!(can_convert(&Ty::Exception, &Ty::String));
        assert!(can_convert(&Ty::Map, &Ty::String));
        assert!(!can_convert(&Ty::Void, &Ty::Object));
        assert!(!can_convert(&Ty::Iterable, &Ty::Object));
    }

    #[test]
    fn type_names_are_case_insensitive() {
        assert_eq!(builtin_type("INT"), Some(Ty::Int));
        assert_eq!(builtin_type("stringBuilder"), Some(Ty::StringBuilder));
        assert_eq!(builtin_type("Label"), None);
    }
}
