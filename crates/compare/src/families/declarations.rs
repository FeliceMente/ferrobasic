//! One to three statements that declare, assign, read or loop over one name,
//! `n`, in source order, where `n` is otherwise a plain local, a parameter
//! or a module variable of some type, or a variable of `Process_Globals`
//! (the rules of recording runs 14 and 15 on declarations and on `For
//! Each` without `As`).

use super::{Program, indent, sequences};

/// The statements, by a short code.
const STATEMENTS: &[(&str, &str)] = &[
    ("di", "Dim n As Int"),
    ("do", "Dim n As Object"),
    ("ds", "Dim n As String"),
    ("asg", "n = 5"),
    ("log", "Log(n)"),
    ("for", "For n = 1 To 2\nNext"),
    ("fti", "For n As Int = 1 To 2\nNext"),
    ("fei", "For Each n As Int In Array As Int(1)\nNext"),
    ("feo", "For Each n As Object In Array As Int(1)\nNext"),
    ("feu", "For Each n In Array As Int(1)\nNext"),
];

/// Where the statements are, and what else declares `n`.
const CONTEXTS: &[(&str, Context)] = &[
    ("local", Context::Local),
    ("pint", Context::Param("Int")),
    ("pobj", Context::Param("Object")),
    ("gint", Context::Global("Int")),
    ("gobj", Context::Global("Object")),
    ("pg", Context::ProcessGlobals),
];

#[derive(Clone, Copy)]
enum Context {
    /// In `AppStart`, with nothing else named `n`.
    Local,
    /// In a Sub with a parameter `n` of this type.
    Param(&'static str),
    /// In `AppStart`, with a module variable `n` of this type.
    Global(&'static str),
    /// In `Process_Globals`.
    ProcessGlobals,
}

pub fn programs() -> Vec<Program> {
    let mut all = Vec::new();
    for &(context_name, context) in CONTEXTS {
        for len in 1..=3 {
            for statements in sequences(STATEMENTS, len) {
                all.push(program(context_name, context, &statements));
            }
        }
    }
    all
}

fn program(context_name: &str, context: Context, statements: &[(&str, &str)]) -> Program {
    let codes: Vec<&str> = statements.iter().map(|(code, _)| *code).collect();
    let body: String = statements.iter().map(|(_, code)| indent(code, 1)).collect();
    let (pg, app, other) = match context {
        Context::Local => (String::new(), body, String::new()),
        Context::Param(ty) => (
            String::new(),
            "    Show(1)\n".to_string(),
            format!("\nSub Show (n As {ty})\n{body}End Sub\n"),
        ),
        Context::Global(ty) => (format!("    Private n As {ty}\n"), body, String::new()),
        Context::ProcessGlobals => (body, String::new(), String::new()),
    };
    let source = format!(
        "Sub Process_Globals\n{pg}End Sub\n\nSub AppStart (Args() As String)\n{app}End Sub\n{other}"
    );
    Program {
        name: format!("{context_name}__{}", codes.join("_")),
        source,
    }
}
