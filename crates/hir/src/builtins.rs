//! The Level 0 built-ins: keywords, constants and library members
//! (`docs/spec/level0-builtins.md`). Names are case-insensitive.
//!
//! `Array`, `CreateMap`, `IIf` and `x.As(T)` have special syntax or typing
//! and are handled by the lowering, not by this table.

use crate::ty::Ty;

/// Who a built-in belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Owner {
    /// A keyword or constant used without a receiver: `Log(x)`, `CRLF`.
    Global,
    /// `Bit.And(a, b)`: functions on the static `Bit` object.
    Bit,
    String,
    List,
    Map,
    StringBuilder,
    Exception,
    Array,
    /// Members every `Type` record has.
    Record,
}

/// Parameter and result types of the signature table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum P {
    Bool,
    Byte,
    Int,
    Long,
    Double,
    Char,
    Str,
    Obj,
    List,
    Map,
    Sb,
    Exc,
    Iter,
    Bytes,
    Void,
}

impl P {
    pub fn ty(self) -> Ty {
        match self {
            P::Bool => Ty::Boolean,
            P::Byte => Ty::Byte,
            P::Int => Ty::Int,
            P::Long => Ty::Long,
            P::Double => Ty::Double,
            P::Char => Ty::Char,
            P::Str => Ty::String,
            P::Obj => Ty::Object,
            P::List => Ty::List,
            P::Map => Ty::Map,
            P::Sb => Ty::StringBuilder,
            P::Exc => Ty::Exception,
            P::Iter => Ty::Iterable,
            P::Bytes => Ty::Array(Box::new(Ty::Byte), 1),
            P::Void => Ty::Void,
        }
    }
}

/// One entry of the signature table.
#[derive(Debug, Clone, Copy)]
pub struct Signature {
    pub id: Builtin,
    pub owner: Owner,
    /// Spelling from the reference documentation.
    pub name: &'static str,
    pub params: &'static [P],
    pub ret: P,
}

macro_rules! builtins {
    ($( $id:ident = $owner:ident $name:literal ( $($p:ident),* ) -> $ret:ident ; )*) => {
        /// Every Level 0 built-in.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum Builtin { $($id,)* }

        const SIGNATURES: &[Signature] = &[
            $(Signature {
                id: Builtin::$id,
                owner: Owner::$owner,
                name: $name,
                params: &[$(P::$p),*],
                ret: P::$ret,
            },)*
        ];
    };
}

builtins! {
    // Output.
    Log = Global "Log"(Str) -> Void;
    LogError = Global "LogError"(Str) -> Void;
    LogDebug = Global "LogDebug"(Str) -> Void;
    // Constants.
    Crlf = Global "CRLF"() -> Str;
    Tab = Global "TAB"() -> Str;
    Quote = Global "QUOTE"() -> Str;
    Pi = Global "cPI"() -> Double;
    E = Global "cE"() -> Double;
    LastException = Global "LastException"() -> Exc;
    // Functions.
    Not = Global "Not"(Bool) -> Bool;
    Abs = Global "Abs"(Double) -> Double;
    Sqrt = Global "Sqrt"(Double) -> Double;
    Power = Global "Power"(Double, Double) -> Double;
    Floor = Global "Floor"(Double) -> Double;
    Ceil = Global "Ceil"(Double) -> Double;
    Sin = Global "Sin"(Double) -> Double;
    Cos = Global "Cos"(Double) -> Double;
    Tan = Global "Tan"(Double) -> Double;
    ASin = Global "ASin"(Double) -> Double;
    ACos = Global "ACos"(Double) -> Double;
    ATan = Global "ATan"(Double) -> Double;
    ATan2 = Global "ATan2"(Double, Double) -> Double;
    SinD = Global "SinD"(Double) -> Double;
    CosD = Global "CosD"(Double) -> Double;
    TanD = Global "TanD"(Double) -> Double;
    ASinD = Global "ASinD"(Double) -> Double;
    ACosD = Global "ACosD"(Double) -> Double;
    ATanD = Global "ATanD"(Double) -> Double;
    ATan2D = Global "ATan2D"(Double, Double) -> Double;
    Logarithm = Global "Logarithm"(Double, Double) -> Double;
    Max = Global "Max"(Double, Double) -> Double;
    Min = Global "Min"(Double, Double) -> Double;
    Round = Global "Round"(Double) -> Long;
    Round2 = Global "Round2"(Double, Int) -> Double;
    NumberFormat = Global "NumberFormat"(Double, Int, Int) -> Str;
    NumberFormat2 = Global "NumberFormat2"(Double, Int, Int, Int, Bool) -> Str;
    IsNumber = Global "IsNumber"(Str) -> Bool;
    Asc = Global "Asc"(Char) -> Int;
    Chr = Global "Chr"(Int) -> Char;
    Rnd = Global "Rnd"(Int, Int) -> Int;
    RndSeed = Global "RndSeed"(Long) -> Void;
    ExitApplication = Global "ExitApplication"() -> Void;
    ExitApplication2 = Global "ExitApplication2"(Int) -> Void;
    GetType = Global "GetType"(Obj) -> Str;
    // Bit.
    BitAnd = Bit "And"(Int, Int) -> Int;
    BitOr = Bit "Or"(Int, Int) -> Int;
    BitXor = Bit "Xor"(Int, Int) -> Int;
    BitNot = Bit "Not"(Int) -> Int;
    BitShiftLeft = Bit "ShiftLeft"(Int, Int) -> Int;
    BitShiftRight = Bit "ShiftRight"(Int, Int) -> Int;
    BitUnsignedShiftRight = Bit "UnsignedShiftRight"(Int, Int) -> Int;
    BitToHexString = Bit "ToHexString"(Int) -> Str;
    BitToBinaryString = Bit "ToBinaryString"(Int) -> Str;
    BitToOctalString = Bit "ToOctalString"(Int) -> Str;
    BitParseInt = Bit "ParseInt"(Str, Int) -> Int;
    // String.
    StrCharAt = String "CharAt"(Int) -> Char;
    StrCompareTo = String "CompareTo"(Str) -> Int;
    StrContains = String "Contains"(Str) -> Bool;
    StrStartsWith = String "StartsWith"(Str) -> Bool;
    StrEndsWith = String "EndsWith"(Str) -> Bool;
    StrEqualsIgnoreCase = String "EqualsIgnoreCase"(Str) -> Bool;
    StrIndexOf = String "IndexOf"(Str) -> Int;
    StrIndexOf2 = String "IndexOf2"(Str, Int) -> Int;
    StrLastIndexOf = String "LastIndexOf"(Str) -> Int;
    StrLastIndexOf2 = String "LastIndexOf2"(Str, Int) -> Int;
    StrLength = String "Length"() -> Int;
    StrReplace = String "Replace"(Str, Str) -> Str;
    StrSubString = String "SubString"(Int) -> Str;
    StrSubString2 = String "SubString2"(Int, Int) -> Str;
    StrToLowerCase = String "ToLowerCase"() -> Str;
    StrToUpperCase = String "ToUpperCase"() -> Str;
    StrTrim = String "Trim"() -> Str;
    StrGetBytes = String "GetBytes"(Str) -> Bytes;
    // StringBuilder.
    SbInitialize = StringBuilder "Initialize"() -> Void;
    SbIsInitialized = StringBuilder "IsInitialized"() -> Bool;
    SbAppend = StringBuilder "Append"(Str) -> Sb;
    SbInsert = StringBuilder "Insert"(Int, Str) -> Sb;
    SbRemove = StringBuilder "Remove"(Int, Int) -> Sb;
    SbLength = StringBuilder "Length"() -> Int;
    SbToString = StringBuilder "ToString"() -> Str;
    // List.
    ListInitialize = List "Initialize"() -> Void;
    ListInitialize2 = List "Initialize2"(List) -> Void;
    ListIsInitialized = List "IsInitialized"() -> Bool;
    ListAdd = List "Add"(Obj) -> Void;
    ListAddAll = List "AddAll"(List) -> Void;
    ListAddAllAt = List "AddAllAt"(Int, List) -> Void;
    ListInsertAt = List "InsertAt"(Int, Obj) -> Void;
    ListGet = List "Get"(Int) -> Obj;
    ListSet = List "Set"(Int, Obj) -> Void;
    ListRemoveAt = List "RemoveAt"(Int) -> Void;
    ListClear = List "Clear"() -> Void;
    ListSize = List "Size"() -> Int;
    ListIndexOf = List "IndexOf"(Obj) -> Int;
    ListSubList = List "SubList"(Int, Int) -> List;
    ListSort = List "Sort"(Bool) -> Void;
    ListSortCaseInsensitive = List "SortCaseInsensitive"(Bool) -> Void;
    ListSortType = List "SortType"(Str, Bool) -> Void;
    ListSortTypeCaseInsensitive = List "SortTypeCaseInsensitive"(Str, Bool) -> Void;
    // Map.
    MapInitialize = Map "Initialize"() -> Void;
    MapIsInitialized = Map "IsInitialized"() -> Bool;
    MapPut = Map "Put"(Obj, Obj) -> Obj;
    MapGet = Map "Get"(Obj) -> Obj;
    MapGetDefault = Map "GetDefault"(Obj, Obj) -> Obj;
    MapRemove = Map "Remove"(Obj) -> Obj;
    MapContainsKey = Map "ContainsKey"(Obj) -> Bool;
    MapClear = Map "Clear"() -> Void;
    MapSize = Map "Size"() -> Int;
    MapKeys = Map "Keys"() -> Iter;
    MapValues = Map "Values"() -> Iter;
    MapGetKeyAt = Map "GetKeyAt"(Int) -> Obj;
    MapGetValueAt = Map "GetValueAt"(Int) -> Obj;
    // Exception.
    ExcMessage = Exception "Message"() -> Str;
    ExcIsInitialized = Exception "IsInitialized"() -> Bool;
    // Arrays.
    ArrayLength = Array "Length"() -> Int;
    // Type records.
    RecordInitialize = Record "Initialize"() -> Void;
    RecordIsInitialized = Record "IsInitialized"() -> Bool;
}

impl Builtin {
    pub fn signature(self) -> &'static Signature {
        &SIGNATURES[self as usize]
    }
}

/// Every Level 0 built-in, in declaration order.
pub fn all() -> &'static [Signature] {
    SIGNATURES
}

/// Looks up a member (or, with `Owner::Global`, a keyword) by name.
pub fn lookup(owner: Owner, name: &str) -> Option<&'static Signature> {
    SIGNATURES
        .iter()
        .find(|s| s.owner == owner && s.name.eq_ignore_ascii_case(name))
}

/// The names [`later_level`] knows.
pub fn later_level_names() -> impl Iterator<Item = &'static str> {
    LATER.iter().map(|&(name, _)| name)
}

/// The owner whose members a value of type `ty` has.
pub fn owner_of(ty: &Ty) -> Option<Owner> {
    Some(match ty {
        Ty::String => Owner::String,
        Ty::List => Owner::List,
        Ty::Map => Owner::Map,
        Ty::StringBuilder => Owner::StringBuilder,
        Ty::Exception => Owner::Exception,
        Ty::Array(..) => Owner::Array,
        Ty::Record(_) => Owner::Record,
        _ => return None,
    })
}

/// Keywords and objects of later levels, with what each one needs.
const LATER: &[(&str, &str)] = &[
    ("Sleep", "resumable subs (Level 2)"),
    ("Wait", "resumable subs (Level 2)"),
    ("CallSub", "dynamic calls (Level 2)"),
    ("CallSub2", "dynamic calls (Level 2)"),
    ("CallSub3", "dynamic calls (Level 2)"),
    ("CallSubDelayed", "dynamic calls (Level 2)"),
    ("CallSubDelayed2", "dynamic calls (Level 2)"),
    ("CallSubDelayed3", "dynamic calls (Level 2)"),
    ("SubExists", "dynamic calls (Level 2)"),
    ("Sender", "events (Level 1)"),
    ("Me", "classes (Level 1)"),
    ("StartMessageLoop", "the message loop (Level 2)"),
    ("StopMessageLoop", "the message loop (Level 2)"),
    ("DateTime", "the DateTime object (Level 1)"),
    ("File", "the File object (Level 3)"),
    ("Regex", "the Regex object (Level 1)"),
    ("Timer", "timers and events (Level 1)"),
    (
        "BytesToString",
        "byte and char arrays conversions (Level 1)",
    ),
    (
        "CharsToString",
        "byte and char arrays conversions (Level 1)",
    ),
    (
        "GetEnvironmentVariable",
        "the process environment (Level 1)",
    ),
    ("GetSystemProperty", "the process environment (Level 1)"),
    ("SetSystemProperty", "the process environment (Level 1)"),
    ("IsDevTool", "the process environment (Level 1)"),
    ("Density", "UI support (Level 4)"),
    ("DipToCurrent", "UI support (Level 4)"),
];

/// Keywords and objects of later levels, for a precise diagnostic instead of
/// "unknown name". The text says what is missing.
pub fn later_level(name: &str) -> Option<&'static str> {
    LATER
        .iter()
        .find(|(n, _)| n.eq_ignore_ascii_case(name))
        .map(|&(_, what)| what)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_is_indexed_by_variant() {
        for (i, sig) in SIGNATURES.iter().enumerate() {
            assert_eq!(sig.id as usize, i);
        }
    }

    #[test]
    fn names_are_unique_per_owner() {
        for (i, a) in SIGNATURES.iter().enumerate() {
            for b in &SIGNATURES[i + 1..] {
                assert!(
                    !(a.owner == b.owner && a.name.eq_ignore_ascii_case(b.name)),
                    "{} defined twice",
                    a.name
                );
            }
        }
    }

    #[test]
    fn lookup_is_case_insensitive() {
        assert_eq!(lookup(Owner::Global, "log").unwrap().id, Builtin::Log);
        assert_eq!(lookup(Owner::Map, "KEYS").unwrap().id, Builtin::MapKeys);
        assert!(lookup(Owner::List, "Put").is_none());
        assert!(later_level("datetime").is_some());
    }
}
