//! One to three statements that declare, assign, read or loop over one name
//! in source order, where the name is otherwise a plain local, a parameter
//! or a module variable of some type, or a variable of `Process_Globals`
//! (the rules of recording runs 14 and 15 on declarations and on `For
//! Each` without `As`). The name is an ordinary one, a built-in's, a
//! `Type`'s or a Sub's, as each takes its own path.

use super::{Program, indent, sequences};

/// The names, by a short code: an ordinary name, a built-in's, a `Type`'s
/// and a Sub's. The statements below are written with `n`.
const NAMES: &[(&str, &str)] = &[
    ("n", "n"),
    ("abs", "Abs"),
    ("type", "Point"),
    ("sub", "Bump"),
];

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
    for &(name_code, name) in NAMES {
        for &(context_name, context) in CONTEXTS {
            for len in 1..=3 {
                for statements in sequences(STATEMENTS, len) {
                    all.push(program(name_code, name, context_name, context, &statements));
                }
            }
        }
    }
    all
}

fn program(
    name_code: &str,
    name: &str,
    context_name: &str,
    context: Context,
    statements: &[(&str, &str)],
) -> Program {
    let codes: Vec<&str> = statements.iter().map(|(code, _)| *code).collect();
    let body: String = statements
        .iter()
        .map(|(_, code)| indent(&with_name(code, name), 1))
        .collect();
    let (pg, app, other) = match context {
        Context::Local => (String::new(), body, String::new()),
        Context::Param(ty) => (
            String::new(),
            "    Show(1)\n".to_string(),
            format!("\nSub Show ({name} As {ty})\n{body}End Sub\n"),
        ),
        Context::Global(ty) => (format!("    Private {name} As {ty}\n"), body, String::new()),
        Context::ProcessGlobals => (body, String::new(), String::new()),
    };
    let source = format!(
        "Sub Process_Globals\n    Type Point (X As Int)\n{pg}End Sub\n\nSub AppStart (Args() As \
         String)\n{app}End Sub\n{other}\nSub Bump As Int\n    Return 7\nEnd Sub\n"
    );
    Program {
        name: format!("{name_code}__{context_name}__{}", codes.join("_")),
        source,
    }
}

/// A statement written with `n`, with `name` instead: every `n` that is a
/// whole word (the statements' other words are longer).
fn with_name(code: &str, name: &str) -> String {
    let mut out = String::new();
    let mut word = String::new();
    for c in code.chars().chain(std::iter::once(' ')) {
        if c.is_ascii_alphanumeric() {
            word.push(c);
            continue;
        }
        if !word.is_empty() {
            out.push_str(if word == "n" { name } else { &word });
            word.clear();
        }
        out.push(c);
    }
    out.pop();
    out
}
