use ferro_syntax::{Completeness::*, FragmentKind, parse_fragment};
#[test]
fn fragments_use_language_grammar() {
    for s in [
        "If True Then\n",
        "For i = 1 To 2\n",
        "Try\n",
        "Sub F As Int\n",
        "\"open\n",
        "$\"open",
        "$\"${1",
        "1 + _\n",
    ] {
        assert_eq!(parse_fragment(s).2, Incomplete, "{s:?}");
    }
    for s in [
        "If Then\n",
        "Dim x As\n",
        "Next\n",
        "End If\n",
        "(1 +\n",
        "Dim x As Int =\n",
    ] {
        assert_eq!(parse_fragment(s).2, Invalid, "{s:?}");
    }
    for s in [
        "1 + 2\n",
        "x = 3\n",
        "(x = 3)\n",
        "If True Then\nLog(1)\nEnd If\n",
        "Type Point(X As Int)\n",
        "Sub F\nEnd Sub\n",
    ] {
        let (p, _, status) = parse_fragment(s);
        assert_eq!(status, Complete, "{s}: {:?}", p.errors());
        assert_eq!(p.syntax().text().to_string(), s);
    }
    assert_eq!(parse_fragment("x = 3\n").1, FragmentKind::Statements);
    assert_eq!(parse_fragment("x + 3\n").1, FragmentKind::Expression);
}
