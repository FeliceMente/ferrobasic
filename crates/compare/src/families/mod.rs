//! The families of generated programs. Each one covers one area of the
//! language with every combination of a few building blocks, so that a
//! change to how that area is checked shows in which programs it changes.

mod consts;
mod sub_named_loops;

/// A generated program: its name, unique in its family, and its source.
pub struct Program {
    pub name: String,
    pub source: String,
}

/// A family of generated programs.
pub struct Family {
    pub name: &'static str,
    pub about: &'static str,
    pub generate: fn() -> Vec<Program>,
}

pub const FAMILIES: &[Family] = &[
    Family {
        name: "const",
        about: "one to three declarations of a name in Process_Globals \
                (Const or Dim, several types, with and without a value), then \
                one use of it; Process_Globals first or last, a statement \
                between the declarations, the second one inside an If, a \
                second Sub assigning it",
        generate: consts::programs,
    },
    Family {
        name: "sub-named-loops",
        about: "one to three loops (For, For Each with or without As, of \
                several types) over a variable named like a Sub, one after \
                the other or nested, in a Sub, in Process_Globals or in both",
        generate: sub_named_loops::programs,
    },
];

/// Every sequence of `len` items of `items`, in order: `items` to the power
/// of `len`.
fn sequences<T: Copy>(items: &[T], len: usize) -> Vec<Vec<T>> {
    let mut all = vec![Vec::new()];
    for _ in 0..len {
        all = all
            .into_iter()
            .flat_map(|seq| {
                items.iter().map(move |&item| {
                    let mut next = seq.clone();
                    next.push(item);
                    next
                })
            })
            .collect();
    }
    all
}

/// Indents every line of `text` by `depth` levels of four spaces.
fn indent(text: &str, depth: usize) -> String {
    let pad = "    ".repeat(depth);
    text.lines().map(|line| format!("{pad}{line}\n")).collect()
}
