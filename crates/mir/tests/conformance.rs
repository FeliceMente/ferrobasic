//! Every Level 0 conformance program that type-checks lowers to a valid MIR
//! program.

use std::fs;
use std::path::Path;

#[test]
fn conformance_programs_lower_and_validate() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/conformance/level0");
    let mut entries: Vec<_> = fs::read_dir(&dir)
        .expect("conformance directory")
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "bas"))
        .collect();
    entries.sort();
    let mut lowered = 0;
    for path in entries {
        let meta = fs::read_to_string(path.with_extension("meta.toml")).unwrap_or_default();
        let negative = meta
            .lines()
            .any(|l| l.trim_start().starts_with("status") && l.contains("\"negative\""));
        if negative {
            continue;
        }
        let name = path.display();
        let text = fs::read_to_string(&path).unwrap();
        let parse = ferro_syntax::parse(&text);
        assert!(parse.errors().is_empty(), "{name}: {:?}", parse.errors());
        let (module, diags) = ferro_hir::check(&parse.syntax());
        assert!(diags.is_empty(), "{name}: {diags:?}");
        let program = ferro_mir::lower(&module);
        if let Err(e) = ferro_mir::validate(&program) {
            panic!("{name}: {e}\n{}", ferro_mir::dump(&program));
        }
        lowered += 1;
    }
    assert!(lowered > 0, "no conformance programs found");
}
