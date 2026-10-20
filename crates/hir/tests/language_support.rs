//! `docs/language-support.md` names every built-in the compiler knows, and
//! every construct it reports as belonging to a later level.

use ferro_hir::builtins;

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

#[test]
fn every_builtin_is_on_the_page() {
    let page = page();
    let missing: Vec<_> = builtins::all()
        .iter()
        .map(|signature| signature.name)
        .chain(SPECIAL.iter().copied())
        .filter(|name| !named(&page, name))
        .collect();
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
