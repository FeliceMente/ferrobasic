//! Every conformance program FerroBASIC accepts lowers to a valid MIR
//! program.

#[test]
fn conformance_programs_lower_and_validate() {
    for program in ferro_corpus::programs().into_iter().filter(|p| !p.rejected) {
        let name = &program.name;
        let parse = ferro_syntax::parse(&program.source());
        assert!(parse.errors().is_empty(), "{name}: {:?}", parse.errors());
        let (module, diags) = ferro_hir::check(&parse.syntax());
        assert!(diags.is_empty(), "{name}: {diags:?}");
        let lowered = ferro_mir::lower(&module);
        if let Err(e) = ferro_mir::validate(&lowered) {
            panic!("{name}: {e}\n{}", ferro_mir::dump(&lowered));
        }
    }
}
