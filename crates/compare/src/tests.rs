use std::collections::HashSet;

use crate::families::FAMILIES;
use crate::{Change, Compared, Outcome, Report};

fn family(name: &str) -> &'static crate::families::Family {
    FAMILIES.iter().find(|f| f.name == name).unwrap()
}

#[test]
fn families_have_their_sizes_and_unique_names() {
    // The sizes of the comparisons run by hand for #67 and #68.
    for (name, size) in [("const", 65_160), ("sub-named-loops", 1_687)] {
        let programs = (family(name).generate)();
        assert_eq!(programs.len(), size, "{name}");
        let names: HashSet<&str> = programs.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names.len(), size, "{name}: names are unique");
    }
}

#[test]
fn every_generated_program_parses() {
    // Every 20th program of the big family is enough to meet every
    // declaration, use and variant, and keeps this test quick.
    for family in FAMILIES {
        let step = if family.name == "const" { 20 } else { 1 };
        for program in (family.generate)().into_iter().step_by(step) {
            let parse = ferro_syntax::parse(&program.source);
            assert!(
                parse.errors().is_empty(),
                "{}/{}: {:?}\n{}",
                family.name,
                program.name,
                parse.errors(),
                program.source
            );
        }
    }
}

#[test]
fn generated_programs_are_the_expected_shapes() {
    let programs = (family("const").generate)();
    let find = |name: &str| programs.iter().find(|p| p.name == name).unwrap();
    assert!(
        find("Ci_Di__log")
            .source
            .starts_with("Sub Process_Globals\n")
    );
    assert!(find("first__Ci_Di__log").source.starts_with("Sub AppStart"));
    assert!(find("otherfirst__Ci__asg").source.starts_with("Sub Other"));
    assert!(find("midasg__Ci_Di__log").source.contains(
        "    Private Const Size As Int = 10\n    Size = 4\n    Private Size As Int = 3\n"
    ));
    assert!(
        find("inif__Ci_Di__log")
            .source
            .contains("    If True Then\n        Private Size As Int = 3\n    End If\n")
    );
    let loops = (family("sub-named-loops").generate)();
    let nested = loops.iter().find(|p| p.name == "TC_nest_sub").unwrap();
    assert!(nested.source.contains(
        "    For Each Bump As Int In Array As Int(1)\n        For Bump = 1 To 2\n        Next\n    Next\n"
    ));
}

#[test]
fn outcomes_follow_the_exit_codes() {
    assert_eq!(Outcome::from_check(Some(0), ""), Outcome::Accepted);
    let stderr = "a.bas:3:5: error: one\n    x = 1\n    ^\na.bas:4:1: error: two\n2 error(s)\n";
    assert_eq!(
        Outcome::from_check(Some(3), stderr),
        Outcome::Refused(vec![
            "a.bas:3:5: error: one".into(),
            "a.bas:4:1: error: two".into()
        ])
    );
    assert!(matches!(
        Outcome::from_check(Some(101), "panicked"),
        Outcome::Crashed(_)
    ));
    assert!(matches!(Outcome::from_check(None, ""), Outcome::Crashed(_)));
}

#[test]
fn changes_between_outcomes() {
    let refused =
        |errors: &[&str]| Outcome::Refused(errors.iter().map(|e| e.to_string()).collect());
    let crashed = Outcome::Crashed("exit code 101".into());
    let cases = [
        (Outcome::Accepted, Outcome::Accepted, None),
        (refused(&["a"]), refused(&["a"]), None),
        (
            refused(&["a"]),
            Outcome::Accepted,
            Some(Change::NewlyAccepted),
        ),
        (
            Outcome::Accepted,
            refused(&["a"]),
            Some(Change::NewlyRefused),
        ),
        (
            refused(&["a"]),
            refused(&["a", "b"]),
            Some(Change::MoreErrors),
        ),
        (
            refused(&["a", "b"]),
            refused(&["a"]),
            Some(Change::FewerErrors),
        ),
        (
            refused(&["a"]),
            refused(&["b"]),
            Some(Change::OtherMessages),
        ),
        (Outcome::Accepted, crashed.clone(), Some(Change::NewCrash)),
        (crashed.clone(), crashed.clone(), Some(Change::NewCrash)),
        (crashed.clone(), Outcome::Accepted, Some(Change::OldCrash)),
    ];
    for (old, new, change) in cases {
        assert_eq!(Change::between(&old, &new), change, "{old:?} -> {new:?}");
    }
}

#[test]
fn the_report_counts_and_shows_the_smallest_example() {
    let program = |name: &str, len, old, new| Compared {
        family: "const",
        name: name.to_string(),
        path: format!("work/{name}.bas").into(),
        source_len: len,
        old,
        new,
    };
    let refused = Outcome::Refused(vec!["work/x.bas:2:19: error: open".into()]);
    let report = Report {
        old_name: "origin/dev (abc)".into(),
        new_name: "target/debug/ferro".into(),
        programs: vec![
            program("big", 200, refused.clone(), Outcome::Accepted),
            program("small", 100, refused.clone(), Outcome::Accepted),
            program("same", 50, Outcome::Accepted, Outcome::Accepted),
        ],
    };
    let text = report.render(1);
    assert!(
        text.starts_with("3 programs (const 3): old origin/dev (abc), new target/debug/ferro\n")
    );
    let count = |label: &str| {
        text.lines()
            .find(|line| line.trim_start().starts_with(label))
            .and_then(|line| line.split_whitespace().last())
            .map(str::to_string)
    };
    assert_eq!(count("newly accepted").as_deref(), Some("2"), "{text}");
    assert_eq!(count("newly refused").as_deref(), Some("0"), "{text}");
    assert!(
        text.contains("\nnewly accepted (2):\n\n  const/small  work/small.bas\n"),
        "{text}"
    );
    assert!(!text.contains("const/big"), "{text}");
    assert!(
        text.contains("    old: 1 error(s)\n      2:19: open\n    new: accepted\n"),
        "{text}"
    );
    assert!(
        text.ends_with("accepted: old 1, new 3; errors: old 2, new 0\n"),
        "{text}"
    );
    assert_eq!(report.new_crashes(), 0);
}
