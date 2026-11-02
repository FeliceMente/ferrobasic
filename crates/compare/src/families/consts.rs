//! `Const` and `Dim` declarations of one name in `Process_Globals`, and
//! what a Sub does with the name afterwards (recording runs 12 and 13).

use super::{Program, indent, sequences};

/// The declarations, by a short code: `C` a `Const`, `D` a `Dim`, then the
/// type (`i` Int, `n` Int with no value, `s` String, `d` Double, `a` an
/// array with bounds, `r` a record).
const DECLARATIONS: &[(&str, &str)] = &[
    ("Ci", "Private Const Size As Int = 10"),
    ("Di", "Private Size As Int = 3"),
    ("Dn", "Private Size As Int"),
    ("Cs", "Private Const Size As String = \"a\""),
    ("Ds", "Private Size As String = \"b\""),
    ("Cd", "Private Const Size As Double = 1"),
    ("Dd", "Private Size As Double = 2"),
    ("Ca", "Private Const Size(3) As Int"),
    ("Cr", "Private Const Size As Point = NewPoint"),
];

/// What `AppStart` does with the name; `pgfor` loops over it in
/// `Process_Globals` instead.
const USES: &[(&str, &str)] = &[
    ("log", "Log(Size)"),
    ("asg", "Size = 5"),
    ("for", "For Size = 1 To 2\nNext"),
    ("fei", "For Each Size As Int In Array As Int(1)\nNext"),
    ("feu", "For Each Size In Array As Int(1)\nNext"),
    ("dimv", "Dim Size As Int = 4"),
    ("dimn", "Dim Size As Int"),
    ("fld", "Size.X = 1"),
    ("itm", "Size(0) = 1"),
    ("pgfor", ""),
];

/// How the parts are put together.
#[derive(Clone, Copy)]
enum Variant {
    /// `Process_Globals` first.
    Plain,
    /// `AppStart` before `Process_Globals`.
    SubFirst,
    /// A statement between the first two declarations.
    Between(&'static str, &'static str),
    /// The second declaration inside an `If`.
    InIf,
    /// Another Sub assigning the name, after or before `Process_Globals`.
    Other { first: bool },
}

const VARIANTS: &[Variant] = &[
    Variant::Plain,
    Variant::SubFirst,
    Variant::Between("midasg", "Size = 4"),
    Variant::Between("midloop", "For Size = 1 To 2\nNext"),
    Variant::Between("midlog", "Log(Size)"),
    Variant::InIf,
    Variant::Other { first: false },
    Variant::Other { first: true },
];

pub fn programs() -> Vec<Program> {
    let mut all = Vec::new();
    for variant in VARIANTS {
        for len in 1..=3 {
            for decls in sequences(DECLARATIONS, len) {
                for &(use_name, use_code) in USES {
                    if let Some(program) = program(*variant, &decls, use_name, use_code) {
                        all.push(program);
                    }
                }
            }
        }
    }
    all
}

/// Where the parts go in the module.
#[derive(Clone, Copy)]
enum Layout {
    ProcessGlobalsFirst,
    SubFirst,
    /// `Sub Other`, assigning the name, before everything or after `AppStart`.
    Other {
        first: bool,
    },
}

fn program(
    variant: Variant,
    decls: &[(&str, &str)],
    use_name: &str,
    use_code: &str,
) -> Option<Program> {
    let mut globals: Vec<String> = decls.iter().map(|(_, code)| code.to_string()).collect();
    let (prefix, layout) = match variant {
        Variant::Plain => (String::new(), Layout::ProcessGlobalsFirst),
        Variant::SubFirst => ("first__".to_string(), Layout::SubFirst),
        Variant::Between(name, code) => {
            if decls.len() < 2 {
                return None;
            }
            globals.insert(1, code.to_string());
            (format!("{name}__"), Layout::ProcessGlobalsFirst)
        }
        Variant::InIf => {
            if decls.len() < 2 {
                return None;
            }
            globals[1] = format!("If True Then\n    {}\nEnd If", globals[1]);
            ("inif__".to_string(), Layout::ProcessGlobalsFirst)
        }
        Variant::Other { first } => {
            let prefix = if first { "otherfirst__" } else { "other__" };
            (prefix.to_string(), Layout::Other { first })
        }
    };
    let codes: Vec<&str> = decls.iter().map(|(code, _)| *code).collect();
    let name = format!("{prefix}{}__{use_name}", codes.join("_"));
    let mut pg_body: String = globals.iter().map(|g| indent(g, 1)).collect();
    let mut app_body = String::new();
    if use_name == "pgfor" {
        pg_body.push_str(&indent("For Size = 1 To 2\nNext", 1));
    } else {
        app_body = indent(use_code, 1);
    }
    let pg = format!("Sub Process_Globals\n    Type Point (X As Int)\n{pg_body}End Sub\n");
    let app = format!("Sub AppStart (Args() As String)\n{app_body}End Sub\n");
    let other = "Sub Other\n    Size = 5\nEnd Sub\n";
    let new_point =
        "Sub NewPoint As Point\n    Dim q As Point\n    q.Initialize\n    Return q\nEnd Sub\n";
    let source = match layout {
        Layout::ProcessGlobalsFirst => format!("{pg}\n{app}\n{new_point}"),
        Layout::SubFirst => format!("{app}\n{pg}\n{new_point}"),
        Layout::Other { first: true } => format!("{other}\n{app}\n{pg}\n{new_point}"),
        Layout::Other { first: false } => format!("{pg}\n{app}\n{other}\n{new_point}"),
    };
    Some(Program { name, source })
}
