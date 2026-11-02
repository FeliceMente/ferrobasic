//! Comparisons between operands of every type, and chains of them
//! (recording run 14).

use super::Program;

/// The operands, by a short code. `c`, `o` and `l` are declared by every
/// program: a Char, an Object holding a String, and a List.
const OPERANDS: &[(&str, &str)] = &[
    ("int", "10"),
    ("dbl", "2.5"),
    ("lng", "3000000000"),
    ("str", "\"10\""),
    ("chr", "c"),
    ("bool", "True"),
    ("obj", "o"),
    ("null", "Null"),
    ("list", "l"),
];

const OPERATORS: &[(&str, &str)] = &[
    ("eq", "="),
    ("ne", "<>"),
    ("lt", "<"),
    ("gt", ">"),
    ("le", "<="),
    ("ge", ">="),
];

pub fn programs() -> Vec<Program> {
    let mut all = Vec::new();
    for (left_name, left) in OPERANDS {
        for (op_name, op) in OPERATORS {
            for (right_name, right) in OPERANDS {
                all.push(program(
                    format!("{left_name}_{op_name}_{right_name}"),
                    &format!("{left} {op} {right}"),
                ));
            }
        }
    }
    // Two comparisons in a row, bare (ending with a number or a Boolean)
    // and with the first in parentheses.
    for (first_name, first) in OPERATORS {
        for (second_name, second) in OPERATORS {
            let name = format!("{first_name}_{second_name}");
            all.push(program(
                format!("chain_{name}"),
                &format!("1 {first} 2 {second} 3"),
            ));
            all.push(program(
                format!("chainb_{name}"),
                &format!("1 {first} 2 {second} False"),
            ));
            all.push(program(
                format!("paren_{name}"),
                &format!("(1 {first} 2) {second} False"),
            ));
        }
    }
    all
}

fn program(name: String, expr: &str) -> Program {
    let source = format!(
        "Sub Process_Globals\nEnd Sub\n\nSub AppStart (Args() As String)\n    Dim c As Char = \"A\"\n    Dim o As Object = \"5\"\n    Dim l As List\n    l.Initialize\n    Log({expr})\n    If {expr} Then\n        Log(\"then\")\n    End If\nEnd Sub\n"
    );
    Program { name, source }
}
