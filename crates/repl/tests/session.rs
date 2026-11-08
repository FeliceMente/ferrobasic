use ferro_repl::{Session, run};

fn script(text: &str, interactive: bool) -> (u8, String, String) {
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let code = run(&mut text.as_bytes(), &mut out, &mut err, interactive);
    (
        code,
        String::from_utf8(out).unwrap(),
        String::from_utf8(err).unwrap(),
    )
}
fn good(text: &str, expected: &str) {
    let (code, out, err) = script(text, false);
    assert_eq!(code, 0, "{err}");
    assert_eq!(out, expected);
    assert_eq!(err, "");
}
#[test]
fn values_and_single_evaluation() {
    good(
        "Dim n As Int = 2\nn + 3\nn = n + 1\nn\n(n = 3)\nLog(n)\nn ' comment\n",
        "5\n3\ntrue\n3\n3\n",
    );
    good(
        "Dim n As Int\nSub Bump As Int\nn = n + 1\nReturn n\nEnd Sub\nBump()\nn\n",
        "1\n1\n",
    );
    good("Dim n As Int = 5\nDim n As Int\nn\n", "0\n");
}
#[test]
fn multiline_strings_blocks_continuations_and_comments() {
    good(
        "'hello\n\nFor n = 1 To 2\n\nLog(n)\nNext\n\"a\nb\"\n1 + _\n2\n",
        "1\n2\na\nb\n3\n",
    );
    good("$\"a${1 + 2}\"$\n", "a3\n");
    good("\"a\n:quit\nb\"\n", "a\n:quit\nb\n");
}
#[test]
fn identities_records_random_and_last_exception_survive() {
    good(
        "Dim l As List\nl.Initialize\nl.Add(1)\nDim alias As List = l\nalias.Add(2)\nl.Size\nType Point(X As Int)\nDim p As Point\np.Initialize\np.X = 7\np.X\nRndSeed(42)\nRnd(0, 100000)\nRnd(0, 100000)\n",
        "2\n7\n31130\n92763\n",
    );
    good(
        "Try\nDim n As Int = \"bad\"\nCatch\nLog(LastException.Message.Contains(\"NumberFormatException\"))\nEnd Try\nLastException.Message.Contains(\"NumberFormatException\")\n",
        "true\ntrue\n",
    );
}
#[test]
fn compatible_replacement_and_sources() {
    good(
        "Sub Twice(n As Int) As Int\nReturn n * 2\nEnd Sub\nSub UseIt As Int\nReturn Twice(3)\nEnd Sub\nUseIt()\nSub Twice(n As Int) As Int\nReturn n * 3\nEnd Sub\nUseIt()\n",
        "6\n9\n",
    );
    let (code, out, err) = script(
        "Sub Bad As Int\nDim n As Int = \"oops\"\nReturn n\nEnd Sub\nBad()\n",
        false,
    );
    assert_eq!(code, 1);
    assert_eq!(out, "");
    assert!(err.contains("<repl:1>:2:"), "{err}");
}
#[test]
fn failed_compile_is_atomic_and_runtime_effects_remain() {
    let (code, out, err) = script(
        "Dim n As Int = 1\nDim n As String = \"bad\"\nn\nSub Value As Int\nReturn 7\nEnd Sub\nSub Value(z As Int) As Int\nReturn z\nEnd Sub\nValue()\nIf True Then\nn = 9\nDim oops As Int = \"bad\"\nEnd If\nn\noops\n",
        true,
    );
    assert_eq!(code, 0);
    assert_eq!(out, "1\n7\n9\n0\n");
    assert!(err.contains("another type"), "{err}");
    assert!(err.contains("replacement Sub"), "{err}");
    assert!(err.contains("NumberFormatException"), "{err}");
}
#[test]
fn reset_cancel_and_pipe_errors() {
    good(
        "If True Then\n:cancel\nDim n As Int = 2\n:reset\nDim n As String = \"fresh\"\nn\n:quit\n",
        "fresh\n",
    );
    assert_eq!(
        script("Dim x As Int = \"bad\"\nLog(\"never\")\n", false).0,
        1
    );
    assert_eq!(script("missing\nLog(\"never\")\n", false).0, 3);
    assert_eq!(script("If True Then\n", false).0, 3);
    assert_eq!(script("\"open\n", false).0, 3);
    assert_eq!(script("ExitApplication2(17)\nLog(\"never\")\n", true).0, 17);
    assert_eq!(script("ExitApplication\nLog(\"never\")\n", true).0, 0);
    let (_, out, err) = script("1\nIf True Then\nLog(2)\nEnd If\n", true);
    assert_eq!(out, "1\n2\n");
    assert_eq!(err, "> > ... ... > ");
}
#[test]
fn constants_and_types_cannot_be_redefined() {
    let (_, out, err) = script(
        "Private Const N As Int = 3\nN = 4\nDim N As Int = 7\nPrivate Const N As Int = 8\nN\nType Point(X As Int)\nType Point(X As String)\n",
        true,
    );
    assert_eq!(out, "3\n");
    assert!(err.contains("constants cannot be redefined"));
    assert!(err.contains("Type cannot be redeclared"));
}
#[test]
fn public_session_engine_and_incomplete_errors() {
    let mut session = Session::default();
    let (mut out, mut err) = (Vec::new(), Vec::new());
    assert_eq!(
        session
            .submit("Sub Process_Globals\nEnd Sub\n", &mut out, &mut err)
            .code,
        3
    );
    assert_eq!(
        session
            .submit("Type Point(X As Int)\nDim p As Point\n", &mut out, &mut err)
            .code,
        3
    );
    assert_eq!(
        session
            .submit("Dim fresh As Int = Missing\n", &mut out, &mut err)
            .code,
        3
    );
    assert_eq!(session.submit("fresh\n", &mut out, &mut err).code, 3);
}

#[test]
fn new_declarations_do_not_capture_existing_bindings() {
    let (_, out, err) = script(
        "Sub F As Int\nDim local As Int = 4\nReturn local\nEnd Sub\nDim local As Int = 99\nF()\nSub G As Int\nFor counter = 1 To 2\nNext\nReturn counter\nEnd Sub\nSub counter As Int\nReturn 99\nEnd Sub\nG()\n",
        true,
    );
    assert_eq!(out, "4\n3\n");
    assert!(err.contains("binding"), "{err}");
    good(
        "Dim Args As Int = 7\nArgs\nSub Identity(n As Int) As Int\nReturn n\nEnd Sub\nSub n As Int\nReturn 9\nEnd Sub\nIdentity(4)\nn()\n",
        "7\n4\n9\n",
    );
    let (_, out, err) = script(
        "Dim n As Int = 4\nSub n As Int\nReturn 9\nEnd Sub\nn\n",
        true,
    );
    assert_eq!(out, "4\n");
    assert!(err.contains("binding"), "{err}");
}

#[test]
fn dependents_are_checked_before_replacement_is_committed() {
    good(
        "Sub Twice(x As Int) As Int\nReturn x * 2\nEnd Sub\nSub Twice(renamed As Int) As Int\nReturn renamed * 3\nEnd Sub\nTwice(4)\n",
        "12\n",
    );
    let (_, out, err) = script(
        "Sub Twice(x As Int) As Int\nReturn x * 2\nEnd Sub\nSub Caller As Int\nReturn Twice(4)\nEnd Sub\nSub Twice(x As String) As Int\nReturn 9\nEnd Sub\nCaller()\n",
        true,
    );
    assert_eq!(out, "8\n");
    assert!(err.contains("replacement Sub"), "{err}");
}

#[test]
fn array_storage_and_schema_expansion_preserve_values() {
    good(
        "Dim a(2, 2) As Int\na(1, 1) = 7\nDim alias(0, 0) As Int\nalias = a\nType Point(X As Int)\nDim p As Point\np.Initialize\np.X = 8\nType Other(Name As String)\nDim other As Other\nother.Initialize\nalias(1, 1)\np.X\n",
        "7\n8\n",
    );
}

#[test]
fn input_read_failure_is_a_usage_error() {
    struct Broken;
    impl std::io::Read for Broken {
        fn read(&mut self, _: &mut [u8]) -> std::io::Result<usize> {
            Err(std::io::Error::other("test read failure"))
        }
    }
    impl std::io::BufRead for Broken {
        fn fill_buf(&mut self) -> std::io::Result<&[u8]> {
            Err(std::io::Error::other("test read failure"))
        }
        fn consume(&mut self, _: usize) {}
    }
    let (mut out, mut err) = (Vec::new(), Vec::new());
    assert_eq!(run(&mut Broken, &mut out, &mut err, false), 2);
    assert!(out.is_empty());
    assert!(
        String::from_utf8(err)
            .unwrap()
            .contains("cannot read input")
    );
}

#[test]
fn initializer_effects_and_string_constants_are_not_replayed_or_cached() {
    good(
        "Dim count As Int\nSub InitializeValue As Int\ncount = count + 1\nLog(\"initialized\")\nReturn count\nEnd Sub\nDim n As Int = InitializeValue()\n\"alpha\"\n\"beta\"\nn\ncount\n",
        "initialized\nalpha\nbeta\n1\n1\n",
    );
    let (_, out, err) = script("Private Const n As Int = 1 : Dim n As Int = 2\nn\n", true);
    assert!(out.is_empty());
    assert!(err.contains("constants cannot be redefined"));
}

#[test]
fn generated_entry_parameter_does_not_reserve_user_variable_names() {
    good(
        "0\nDim __ferro_repl_args As Int = 7\nDim __ferro_repl_args_ As Int = 8\n__ferro_repl_args + __ferro_repl_args_\n",
        "0\n15\n",
    );
}

#[test]
fn nested_types_are_rejected_without_poisoning_the_session() {
    let (_, out, err) = script(
        "Dim n As Int = 7\nIf True Then\nType Point(X As Int)\nDim p As Point\np.Initialize\nEnd If\nn\nSub F As Int\nType Hidden(X As Int)\nReturn 1\nEnd Sub\nn\nType Point(X As Int)\nDim p As Point\np.Initialize\np.X\n",
        true,
    );
    assert_eq!(out, "7\n7\n0\n");
    assert_eq!(err.matches("outside blocks and Subs").count(), 2, "{err}");
    assert!(!err.contains("InternalError"));
    good("Sub F As Int\nReturn 1\nEnd Sub\nF()\n", "1\n");
}

#[test]
fn trailing_separators_preserve_expression_echo_and_source_locations() {
    good(
        "1:\nAbs(-1):\n1 + 2: ' comment\n(1 = 1):\nLog(4):\n",
        "1\n1\n3\ntrue\n4\n",
    );
    let (code, _, err) = script("    Missing:\n", false);
    assert_eq!(code, 3);
    assert!(err.contains("<repl:1>:1:5:"), "{err}");
}

#[test]
fn expression_runtime_errors_map_to_current_submission_after_definitions() {
    for definition in ["Sub S\nEnd Sub\n", "Type Point(X As Int)\n"] {
        let input =
            format!("{definition}Dim l As List\nl.Initialize\n    l.Get(5): ' trailing comment\n");
        let (code, out, err) = script(&input, false);
        assert_eq!(code, 1);
        assert!(out.is_empty());
        assert!(err.starts_with("<repl:4>:1:5: error:"), "{err}");
        assert!(err.contains("IndexOutOfBoundsException"), "{err}");
    }
}
