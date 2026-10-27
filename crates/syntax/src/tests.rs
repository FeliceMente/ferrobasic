use crate::SyntaxKind::{self, *};
use crate::{SyntaxNode, debug_tree, parse};

/// Parses `text`, checks the lossless round trip and returns the root.
fn parse_ok(text: &str) -> SyntaxNode {
    let parse = parse(text);
    let root = parse.syntax();
    assert_eq!(root.to_string(), text, "round trip");
    assert!(
        parse.errors().is_empty(),
        "unexpected errors {:?} in\n{}",
        parse.errors(),
        debug_tree(&root)
    );
    root
}

/// Parses `text` and returns the error messages (the round trip must hold).
fn parse_errors(text: &str) -> Vec<String> {
    let parse = parse(text);
    assert_eq!(parse.syntax().to_string(), text, "round trip");
    parse.errors().iter().map(|e| e.message.clone()).collect()
}

fn in_sub(body: &str) -> String {
    format!("Sub AppStart (Args() As String)\n{body}\nEnd Sub\n")
}

fn count(root: &SyntaxNode, kind: SyntaxKind) -> usize {
    root.descendants().filter(|n| n.kind() == kind).count()
}

/// The expression assigned in `x = <expr>`, as an S-expression of operators.
fn sexpr(expr: &str) -> String {
    let root = parse_ok(&in_sub(&format!("x = {expr}")));
    let assign = root
        .descendants()
        .find(|n| n.kind() == ASSIGN_STMT)
        .unwrap();
    let value = assign.children().nth(1).unwrap();
    fn walk(node: &SyntaxNode) -> String {
        match node.kind() {
            BIN_EXPR => {
                let kids: Vec<_> = node.children().collect();
                let op = node
                    .children_with_tokens()
                    .filter_map(|e| e.into_token())
                    .find(|t| !t.kind().is_trivia())
                    .unwrap();
                format!("({} {} {})", op.text(), walk(&kids[0]), walk(&kids[1]))
            }
            PREFIX_EXPR => format!("(- {})", walk(&node.children().next().unwrap())),
            PAREN_EXPR => walk(&node.children().next().unwrap()),
            _ => node.text().to_string().trim().to_string(),
        }
    }
    walk(&value)
}

#[test]
fn empty_and_trivia_only_files() {
    parse_ok("");
    parse_ok("' just a comment");
    parse_ok("\r\n\r\n' c\r\n");
}

#[test]
fn header_bom_and_crlf() {
    let text = "\u{feff}B4J=true\r\nType=StaticCode\r\n@EndOfDesignText@\r\nSub Process_Globals\r\nEnd Sub\r\n";
    let root = parse_ok(text);
    assert_eq!(count(&root, SUB_DECL), 1);
}

#[test]
fn attributes_and_regions() {
    let root = parse_ok(
        "#Region Project Attributes\n\t#MainFormWidth: 600\n#End Region\n\nSub X\nEnd Sub\n",
    );
    assert_eq!(count(&root, REGION), 2);
    assert_eq!(count(&root, ATTRIBUTE), 1);
}

#[test]
fn subs_with_params_and_return_types() {
    let root = parse_ok(
        "Private Sub Square (x As Int) As Int\n    Return x * x\nEnd Sub\n\
         Public Sub F(a() As String, b, c As List) As String()\nEnd Sub\n\
         Sub NoParens As Int\nEnd Sub\n",
    );
    assert_eq!(count(&root, SUB_DECL), 3);
    assert_eq!(count(&root, PARAM), 4);
}

#[test]
fn declarations() {
    let root = parse_ok(&in_sub(
        "Dim x As Int = 5\n\
         Dim a = 1, b = 2, c = 3 As Int\n\
         Private i, j, k As Int\n\
         Private txt = \"test\" As String, value = 1.05 As Double\n\
         Public Data(3, 5, 10) As Int\n\
         Dim Days() As String = Array As String(\"Sunday\", \"Monday\")\n\
         Dim m As Map = CreateMap(\"a\": 1, \"b\": 2)\n\
         Dim n",
    ));
    assert_eq!(count(&root, DIM_STMT), 8);
    assert_eq!(count(&root, DECLARATOR), 13);
    assert_eq!(count(&root, ARRAY_EXPR), 1);
    assert_eq!(count(&root, MAP_ENTRY), 2);
}

#[test]
fn type_declaration() {
    let root = parse_ok(
        "Sub Process_Globals\n    Type MyType (Name As String, Items(10) As Int)\nEnd Sub\n",
    );
    assert_eq!(count(&root, FIELD), 2);
}

#[test]
fn if_forms_and_colon() {
    let root = parse_ok(&in_sub(
        "If a Then Log(1)\n\
         If a Then Log(1) Else Log(2)\n\
         If b = 0 Then a = 1 : c = 1\n\
         If a Then\n  x = 1\nElse If b Then\n  x = 2\nElse\n  x = 3\nEnd If\n\
         If a Then Return\n\
         Log(\"x\") : Log(\"y\")",
    ));
    assert_eq!(count(&root, IF_STMT), 5);
    assert_eq!(count(&root, ELSE_IF_CLAUSE), 1);
    assert_eq!(count(&root, ELSE_CLAUSE), 2);
    // `c = 1` after the colon is a statement of the Sub, not of the If.
    let block = root.descendants().find(|n| n.kind() == BLOCK).unwrap();
    let top_assigns = block.children().filter(|n| n.kind() == ASSIGN_STMT).count();
    assert_eq!(top_assigns, 1);
}

#[test]
fn loops_select_try() {
    let root = parse_ok(&in_sub(
        "For i = 10 To 0 Step -5\n  Select i\n    Case 10, 11\n      Log(\"ten\")\n      Exit\n    Case Else\n      Continue\n  End Select\nNext\n\
         For Each K As String In Prices.Keys\nNext\n\
         Do While n < 100\n  n = n + 7\nLoop\n\
         Do Until n = 0 : n = n - 1 : Loop\n\
         Try\n  L.Add(1)\nCatch\n  Log(LastException.Message)\nEnd Try",
    ));
    assert_eq!(count(&root, FOR_STMT), 1);
    assert_eq!(count(&root, FOR_EACH_STMT), 1);
    assert_eq!(count(&root, CASE_CLAUSE), 2);
    assert_eq!(count(&root, DO_STMT), 2);
    assert_eq!(count(&root, TRY_STMT), 1);
}

#[test]
fn calls_members_and_keyword_named_members() {
    let root = parse_ok(&in_sub(
        "Prices.Initialize\nLog(Bit.And(6, 3))\nDim i As Int = o.As(Int)\nx = m.Get(\"a\").As(Map).Get(\"b\")\nMe.Value = 1",
    ));
    assert_eq!(count(&root, EXPR_STMT), 2);
    assert_eq!(count(&root, MEMBER_EXPR), 7);
}

#[test]
fn in_is_a_contextual_keyword() {
    // From a B4X library that compiles: `In` as a parameter and an argument.
    let root = parse_ok(
        "Public Sub Initialize (EventName As String, In As InputStream, out As OutputStream)\n\
         \tastreams.Initialize(In, out, \"astreams\")\n\
         \tFor Each x As Int In In\n\tNext\nEnd Sub\n",
    );
    assert_eq!(count(&root, PARAM), 3);
    assert_eq!(count(&root, FOR_EACH_STMT), 1);
}

#[test]
fn smart_strings() {
    let root = parse_ok(&in_sub(
        "Log($\"Total: ${Total}\"$)\nLog($\"$2{h}:$2{m} $xml{\"<a>\"} ${$\"in\"$}\"$)",
    ));
    assert_eq!(count(&root, SMART_STRING), 3);
    assert_eq!(count(&root, PLACEHOLDER), 5);
}

#[test]
fn line_continuation() {
    let root = parse_ok(&in_sub("x = 1 + _\n    2"));
    assert_eq!(count(&root, BIN_EXPR), 1);
}

#[test]
fn precedence() {
    assert_eq!(sexpr("1 + 2 * 3"), "(+ 1 (* 2 3))");
    assert_eq!(sexpr("(1 + 2) * 3"), "(* (+ 1 2) 3)");
    assert_eq!(sexpr("n Mod 3 = 0"), "(= (Mod n 3) 0)");
    assert_eq!(sexpr("a < b And c Or d"), "(Or (And (< a b) c) d)");
    assert_eq!(sexpr("\"x\" & 1 + 2"), "(& \"x\" (+ 1 2))");
    assert_eq!(sexpr("1 - 2 - 3"), "(- (- 1 2) 3)");
    assert_eq!(sexpr("-2 * 3"), "(* (- 2) 3)");
    assert_eq!(sexpr("Not(a) And b"), "(And Not(a) b)");
}

#[test]
fn assigning_a_comparison() {
    // B4X compiles this to `_bluetoothstate = _newstate == _admin.STATE_ON;`.
    assert_eq!(
        sexpr("NewState = Admin.STATE_ON"),
        "(= NewState Admin.STATE_ON)"
    );
}

#[test]
fn reals_with_a_leading_point() {
    parse_ok(&in_sub(
        "Log(.08)\nx = Array(320000,.08, .05)\ny = -.25 + .5e2",
    ));
    // After an operand the dot is a member access's, and a digit is not a
    // member name.
    for text in ["Log(a.5)", "x = f(1).5", "x = 1.2.3"] {
        let errors = parse_errors(&in_sub(text));
        assert!(
            errors
                .iter()
                .any(|e| e == "expected a member name after `.`"),
            "{text}: {errors:?}"
        );
    }
}

#[test]
fn rejected_vb_forms() {
    let errors = parse_errors(&in_sub(
        "Dim n As Int = 0\nDo\n  n = n + 1\nLoop While n < 3",
    ));
    assert!(
        errors
            .iter()
            .any(|e| e.contains("`While` or `Until` after `Do`")),
        "{errors:?}"
    );
    assert!(
        errors.iter().any(|e| e.contains("`Loop While`")),
        "{errors:?}"
    );

    let errors = parse_errors(&in_sub("Select v\n  Case 1 To 4\n    Log(1)\nEnd Select"));
    assert!(errors.iter().any(|e| e.contains("ranges")), "{errors:?}");

    let errors = parse_errors(&in_sub("Select Case v\nEnd Select"));
    assert!(
        errors.iter().any(|e| e.contains("Select Case")),
        "{errors:?}"
    );
}

#[test]
fn empty_typed_array_literal() {
    let root = parse_ok(&in_sub(
        "Dim xs() As Int = Array As Int()\nDim ys() As String",
    ));
    let array = root.descendants().find(|n| n.kind() == ARRAY_EXPR).unwrap();
    let element_type = array.children().find(|n| n.kind() == TYPE_REF).unwrap();
    assert_eq!(element_type.text().to_string().trim(), "Int");
    assert!(array.children().any(|n| n.kind() == ARG_LIST));
}

#[test]
fn sub_header_needs_a_line_break() {
    let errors = parse_errors("Sub A : Log(1) : End Sub\n");
    assert!(
        errors.iter().any(|e| e.contains("line break")),
        "{errors:?}"
    );
}

#[test]
fn unterminated_block_does_not_swallow_the_next_sub() {
    let text = "Sub A\nIf True Then\nLog(1)\nSub B\nLog(2)\nEnd Sub\n";
    let errors = parse_errors(text);
    assert!(errors.iter().any(|e| e.contains("End If")), "{errors:?}");
    let root = parse(text).syntax();
    assert_eq!(count(&root, SUB_DECL), 2);
    let b = root
        .children()
        .filter(|n| n.kind() == SUB_DECL)
        .nth(1)
        .unwrap();
    assert!(b.text().to_string().contains("Log(2)"));
}

#[test]
fn unicode_text_round_trips() {
    parse_ok(&in_sub(
        "Log($\"café ${x} €\"$)\nLog(\"naïve\") ' commento è ok",
    ));
}

#[test]
fn error_recovery_keeps_parsing() {
    let text = "Sub A\n  x = = 1\n  Log(2)\nEnd Sub\nGarbage here\nSub B\n  If x Then\nEnd Sub\n";
    let errors = parse_errors(text);
    assert!(errors.len() >= 3, "{errors:?}");
    let root = parse(text).syntax();
    assert_eq!(count(&root, SUB_DECL), 2);
}

#[test]
fn every_prefix_of_a_program_round_trips() {
    // Truncated input must never panic or lose text.
    let text = in_sub(
        "Dim t As Double = 0\nFor Each K As String In P.Keys\n  t = t + P.Get(K)\nNext\nIf t > 2 Then Log(\"büg\")\nLog($\"Tötal: ${t} €\"$)",
    );
    for (i, _) in text.char_indices() {
        let prefix = &text[..i];
        assert_eq!(parse(prefix).syntax().to_string(), prefix);
    }
}

/// Deep nesting gives one error, never a stack overflow; the text is kept.
#[test]
fn nesting_deeper_than_the_limit_is_one_error() {
    let deep = |body: String| {
        let errors = parse_errors(&in_sub(&body));
        let too_deep: Vec<_> = errors
            .iter()
            .filter(|e| e.contains("nested too deeply"))
            .collect();
        assert_eq!(too_deep.len(), 1, "{errors:?}");
        // The unwinding levels report nothing else.
        assert_eq!(errors.len(), 1, "{errors:?}");
    };
    let n = 20_000;
    deep(format!("x = {}", vec!["1"; n].join(" + ")));
    deep(format!("x = {}", vec!["\"a\""; n].join(" & ")));
    deep(format!("Log({}1{})", "(".repeat(n), ")".repeat(n)));
    deep(format!("Log({}1{})", "Abs(".repeat(n), ")".repeat(n)));
    deep(format!("Log({}1)", "-".repeat(n)));
    deep(format!("Log(s{})", ".Trim".repeat(n)));
    deep(format!("{}Log(1)", "If True Then ".repeat(n)));
    deep(format!(
        "{}Log(1)\n{}",
        "If True Then\n".repeat(n),
        "End If\n".repeat(n)
    ));
    deep(format!(
        "{}Log(1)\n{}",
        "For i = 1 To 2\n".repeat(n),
        "Next\n".repeat(n)
    ));
}

#[test]
fn nesting_resets_for_each_sub() {
    let deep = in_sub(&format!("x = {}", vec!["1"; 5000].join("+")));
    let text = format!(
        "{deep}{}",
        in_sub("x = (1 +\n").replace("AppStart", "Other")
    );
    let errors = parse_errors(&text);
    assert!(errors[0].contains("nested too deeply"), "{errors:?}");
    // The next Sub is parsed and reported normally.
    assert!(errors.len() > 1, "{errors:?}");
}

#[test]
fn nesting_below_the_limit_is_fine() {
    parse_ok(&in_sub(&format!("x = {}", vec!["1"; 900].join(" + "))));
    parse_ok(&in_sub(&format!(
        "x = {}1{}",
        "(".repeat(450),
        ")".repeat(450)
    )));
    parse_ok(&in_sub(&format!(
        "{}Log(1)\n{}",
        "If True Then\n".repeat(400),
        "End If\n".repeat(400)
    )));
}

#[test]
fn smart_strings_nested_too_deeply() {
    let text = in_sub(&format!(
        "Log({}x{})",
        "$\"${".repeat(5000),
        "}\"$".repeat(5000)
    ));
    let errors = parse_errors(&text);
    assert!(
        errors
            .iter()
            .any(|e| e.contains("smart strings nested too deeply")),
        "{errors:?}"
    );
    parse_ok(&in_sub(&format!(
        "Log({}x{})",
        "$\"${".repeat(50),
        "}\"$".repeat(50)
    )));
}

/// ` _ ' comment` continues the line, with or without the space before
/// `'`, and keeps the comment as a token.
#[test]
fn a_comment_may_follow_a_line_continuation() {
    for text in [
        "x = 1 + _ ' comment\n    2",
        "x = 1 + _\t' comment\r\n    2",
        "x = 1 + _' comment\n    2",
    ] {
        let root = parse_ok(&in_sub(text));
        assert_eq!(count(&root, BIN_EXPR), 1, "{text:?}");
        let comments = root
            .descendants_with_tokens()
            .filter(|t| t.kind() == COMMENT)
            .count();
        assert_eq!(comments, 1, "{text:?}");
    }
    // The line break after the comment is the only one hidden.
    let root = parse_ok(&in_sub("x = 1 + _ ' comment\n    2\ny = 3"));
    assert_eq!(count(&root, BIN_EXPR), 1);
}

/// An operator, member access or call may wrap a subtree that is already
/// deep: the limit counts the height of the whole tree, not only the path
/// being parsed.
#[test]
fn nesting_counts_the_height_of_wrapped_subtrees() {
    let grouped = |start: &str, tail: &str| {
        let mut e = start.to_string();
        for _ in 0..3 {
            e = format!("({e}){}", tail.repeat(600));
        }
        e
    };
    let deep = |body: String| {
        let errors = parse_errors(&in_sub(&body));
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert!(errors[0].contains("nested too deeply"), "{errors:?}");
    };
    deep(format!("x = {}", grouped("1", " + 1")));
    deep(format!("Log({})", grouped("s", ".Trim")));
    deep(format!("Log({})", grouped("f", "(1)")));
    // Two groups of 300 stay within the limit.
    let mut e = "1".to_string();
    for _ in 0..2 {
        e = format!("({e}){}", " + 1".repeat(300));
    }
    parse_ok(&in_sub(&format!("x = {e}")));
}

/// After a too-deep error inside a smart-string placeholder, the next Sub is
/// still parsed (and its errors reported) on its own.
#[test]
fn a_too_deep_placeholder_does_not_swallow_the_next_sub() {
    let deep = in_sub(&format!("Log($\"${{{}}}\"$)", vec!["1"; 5000].join("+")));
    let next = in_sub("x = = 1").replace("AppStart", "Other");
    let errors = parse_errors(&format!("{deep}{next}"));
    assert!(errors[0].contains("nested too deeply"), "{errors:?}");
    assert!(
        errors.iter().any(|e| e == "expected an expression"),
        "{errors:?}"
    );
    let root = crate::parse(&format!("{deep}{next}")).syntax();
    assert_eq!(count(&root, SUB_DECL), 2);
}

/// Each nested call, `Array As` or `CreateMap` adds two levels to the tree
/// (the call and its `ARG_LIST`, the map and its entry), with no operator
/// after it to trigger the check: the finished subtree is checked too.
#[test]
fn nesting_counts_calls_arrays_and_maps() {
    let forms: [(&str, &str, &str, &str); 5] = [
        ("x = ", "Abs(", ")", ""),
        ("", "F(", ")", ""),
        ("a(", "F(", ")", ") = 1"),
        ("x = ", "Array As Object(", ")", ""),
        ("x = ", "CreateMap(1: ", ")", ""),
    ];
    // The innermost operand is 1 or 2 levels tall, to hit the limit exactly.
    let source = |(before, open, close, after): (&str, &str, &str, &str), n: usize, leaf| {
        in_sub(&format!(
            "{before}{}{leaf}{}{after}",
            open.repeat(n),
            close.repeat(n)
        ))
    };
    // The height of a node's subtree, without recursion.
    let height = |node: &SyntaxNode| {
        let mut tallest = 0;
        let mut levels = 0;
        for event in node.preorder() {
            match event {
                rowan::WalkEvent::Enter(_) => {
                    levels += 1;
                    tallest = tallest.max(levels);
                }
                rowan::WalkEvent::Leave(_) => levels -= 1,
            }
        }
        tallest
    };
    for (form, leaf) in forms.into_iter().flat_map(|f| [(f, "1"), (f, "s.X")]) {
        let errors = parse_errors(&source(form, 600, leaf));
        assert_eq!(errors.len(), 1, "{form:?}: {errors:?}");
        assert!(errors[0].contains("nested too deeply"), "{errors:?}");
        parse_ok(&source(form, 300, leaf));
        // Around the limit, a statement is either rejected or fits in it. It
        // takes two levels (statement and simple statement) for one node, so
        // its subtree is at most 999 tall.
        for n in 495..=505 {
            let parse = parse(&source(form, n, leaf));
            if parse.errors().is_empty() {
                let root = parse.syntax();
                for stmt in root
                    .descendants()
                    .filter(|n| matches!(n.kind(), ASSIGN_STMT | EXPR_STMT))
                {
                    assert!(height(&stmt) < 1000, "{form:?} {leaf} with {n} levels");
                }
            } else {
                assert!(parse.errors()[0].message.contains("nested too deeply"));
            }
        }
    }
}

/// `Const`, `Is`, `Wait For` and inline Java parse without a syntax error,
/// so that `hir` can report each one precisely (#32).
#[test]
fn later_constructs_are_recognised() {
    let root = parse_ok(
        "Sub Process_Globals\n\tPrivate Const Size As Int = 10\n\tPublic CONST A = 1, B = 2 As Int\nEnd Sub\n",
    );
    assert_eq!(count(&root, CONST_MODIFIER), 2);
    assert_eq!(count(&root, DECLARATOR), 3);
    // Without a name after it, `Const` is the variable.
    let root = parse_ok(&in_sub("Dim Const As Int\nConst = 1"));
    assert_eq!(count(&root, CONST_MODIFIER), 0);

    let root = parse_ok(&in_sub(
        "If o Is Int Then Log(1)\nb = Not(x Is String) Or y = 1",
    ));
    assert_eq!(count(&root, IS_EXPR), 2);
    // `x Is String` binds like a comparison, inside `Or`.
    let or = root.descendants().find(|n| n.kind() == BIN_EXPR).unwrap();
    assert_eq!(
        or.children().map(|n| n.kind()).collect::<Vec<_>>(),
        [CALL_EXPR, BIN_EXPR]
    );

    let root = parse_ok(&in_sub(
        "Wait For gmap_Ready\nWait For (rs) Complete (Success As Boolean)\nIf a Then Wait For b Else Log(1)",
    ));
    assert_eq!(count(&root, WAIT_FOR_STMT), 3);
    assert_eq!(count(&root, ELSE_CLAUSE), 1);
    // `Wait` alone is a call to a Sub named `Wait`.
    let root = parse_ok(&in_sub("Wait\nWait(1)"));
    assert_eq!(count(&root, WAIT_FOR_STMT), 0);

    let text =
        "Sub AppStart (Args() As String)\nEnd Sub\n#If Java\npublic static void x() {}\n#End If\n";
    let root = parse_ok(text);
    assert_eq!(count(&root, INLINE_JAVA_BLOCK), 1);
    let errors = parse_errors("Sub AppStart (Args() As String)\nEnd Sub\n#If Java\nint x;\n");
    assert_eq!(
        errors,
        ["missing `#End If` for this `#If Java` block: the rest of the file is read as part of it"]
    );
}
