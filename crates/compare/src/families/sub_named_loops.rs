//! Loops over a variable named like a Sub (recording runs 12 and 13).

use super::{Program, indent, sequences};

/// The loops, by a short code. `{}` is where a nested loop goes.
const LOOPS: &[(char, &str)] = &[
    ('C', "For Bump = 1 To 2\n{}Next\n"),
    ('T', "For Each Bump As Int In Array As Int(1)\n{}Next\n"),
    ('U', "For Each Bump In Array As Int(1)\n{}Next\n"),
    ('B', "For Each Bump As Bogus In Array As Int(1)\n{}Next\n"),
    (
        'S',
        "For Each Bump As String In Array As String(\"a\")\n{}Next\n",
    ),
    (
        'D',
        "For Each Bump As Double In Array As Double(1)\n{}Next\n",
    ),
    ('F', "For Bump As Int = 1 To 2\n{}Next\n"),
];

#[derive(Clone, Copy, PartialEq)]
enum Place {
    Sub,
    ProcessGlobals,
    Both,
}

pub fn programs() -> Vec<Program> {
    let mut all = Vec::new();
    for len in 1..=3 {
        for loops in sequences(LOOPS, len) {
            let shapes: &[bool] = if len == 1 { &[false] } else { &[false, true] };
            for &nested in shapes {
                for place in [Place::Sub, Place::ProcessGlobals, Place::Both] {
                    if place == Place::Both && len > 2 {
                        continue;
                    }
                    all.push(program(&loops, nested, place));
                }
            }
        }
    }
    all
}

fn program(loops: &[(char, &str)], nested: bool, place: Place) -> Program {
    let body = if nested {
        loops.iter().rev().fold(String::new(), |inner, (_, code)| {
            code.replacen("{}", &indent(&inner, 1), 1)
        })
    } else {
        loops
            .iter()
            .map(|(_, code)| code.replacen("{}", "", 1))
            .collect()
    };
    let codes: String = loops.iter().map(|(code, _)| *code).collect();
    let shape = if nested { "nest" } else { "seq" };
    let place_name = match place {
        Place::Sub => "sub",
        Place::ProcessGlobals => "pg",
        Place::Both => "both",
    };
    let in_pg = matches!(place, Place::ProcessGlobals | Place::Both);
    let in_sub = matches!(place, Place::Sub | Place::Both);
    let pg = if in_pg {
        indent(&body, 1)
    } else {
        String::new()
    };
    let app = if in_sub {
        indent(&body, 1)
    } else {
        String::new()
    };
    let source = format!(
        "Sub Process_Globals\n{pg}End Sub\n\nSub AppStart (Args() As String)\n{app}    Log(Bump)\nEnd Sub\n\nSub Bump As Int\n    Return 7\nEnd Sub\n"
    );
    Program {
        name: format!("{codes}_{shape}_{place_name}"),
        source,
    }
}
