//! `docs/language-support.md` names every built-in the compiler knows, and
//! every construct it reports as belonging to a later level.

use ferro_hir::builtins::{self, Owner};

/// Built-ins with their own syntax or typing, handled by the lowering rather
/// than by the built-in table.
const SPECIAL: &[&str] = &["Array", "CreateMap", "IIf", "As"];

fn page() -> String {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../docs/language-support.md"
    );
    std::fs::read_to_string(path).unwrap()
}

/// Whether the page names `name` in code: `` `Name` ``, or at the start of a
/// longer code span such as `` `Wait For` `` or `` `IIf(c, a, b)` ``, or as a
/// member call such as `` `x.As(T)` ``.
fn named(page: &str, name: &str) -> bool {
    [
        format!("`{name}`"),
        format!("`{name} "),
        format!("`{name}("),
        format!(".{name}("),
    ]
    .iter()
    .any(|form| page.contains(form.as_str()))
}

/// The first cell of the "Built-in objects" row that lists an object's
/// members; `None` for keywords, which may be named anywhere on the page.
fn row_label(owner: Owner) -> Option<&'static str> {
    Some(match owner {
        Owner::Global => return None,
        Owner::Bit => "`Bit`",
        Owner::String => "`String`",
        Owner::List => "`List`",
        Owner::Map => "`Map`",
        Owner::StringBuilder => "`StringBuilder`",
        Owner::Exception => "`Exception` (`LastException`)",
        Owner::Array => "Arrays",
        Owner::Record => "`Type` records",
    })
}

/// The table row whose first cell is `label`.
fn row<'a>(page: &'a str, label: &str) -> Option<&'a str> {
    let start = format!("| {label} |");
    page.lines().find(|line| line.starts_with(&start))
}

/// Each object's members must be named in that object's own row, so that a
/// member shared by several objects (`Get`, `Size`, `Initialize`) is checked
/// for each of them.
#[test]
fn every_builtin_is_on_the_page() {
    let page = page();
    let mut missing: Vec<String> = builtins::all()
        .iter()
        .filter(|signature| match row_label(signature.owner) {
            None => !named(&page, signature.name),
            Some(label) => !row(&page, label).is_some_and(|row| named(row, signature.name)),
        })
        .map(|signature| format!("{:?}.{}", signature.owner, signature.name))
        .collect();
    missing.extend(
        SPECIAL
            .iter()
            .filter(|name| !named(&page, name))
            .map(|name| name.to_string()),
    );
    assert!(
        missing.is_empty(),
        "docs/language-support.md does not name these built-ins: {missing:?}"
    );
}

#[test]
fn every_later_level_construct_is_on_the_page() {
    let page = page();
    let missing: Vec<_> = builtins::later_level_names()
        .filter(|name| !named(&page, name))
        .collect();
    assert!(
        missing.is_empty(),
        "docs/language-support.md does not name these later-level constructs: {missing:?}"
    );
}
