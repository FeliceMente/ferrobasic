//! The order in which `For` and `Select` evaluate their parts, as recording
//! run 14 shows (`for_evaluation_order`, `case_values_evaluated`). The
//! consequences pinned here, a step or limit reading the counter and the
//! values of a `Select` that ends in `Case Else`, are recorded too by run
//! 15 (`for_limit_reads_counter`, `case_values_evaluated_else`).

mod common;

fn stdout(source: &str) -> String {
    let (stdout, outcome) = common::run(source);
    assert!(outcome.error.is_none(), "{:?}", outcome.error);
    stdout
}

fn run(body: &str) -> String {
    stdout(&common::app_start(body))
}

#[test]
fn for_evaluates_step_then_limit_then_start() {
    let source = "Sub Process_Globals\nEnd Sub\n\nSub AppStart (Args() As String)\n    \
                  For i = Value(\"start\", 1) To Value(\"limit\", 2) Step Value(\"step\", 1)\n    \
                  Next\nEnd Sub\n\nSub Value (Name As String, N As Int) As Int\n    \
                  Log(Name)\n    Return N\nEnd Sub\n";
    assert_eq!(stdout(source), "step\nlimit\nstart\n");
}

#[test]
fn step_and_limit_read_the_counter_before_the_start_is_assigned() {
    // The limit reads 3, not the start 1.
    assert_eq!(
        run("    Dim i As Int = 3\n    For i = 1 To i\n        Log(i)\n    Next"),
        "1\n2\n3\n"
    );
    // The step reads 0.5, not the start 1.
    assert_eq!(
        run("    Dim d As Double = 0.5\n    For d = 1 To 2 Step d\n        Log(d)\n    Next"),
        "1\n1.5\n2\n"
    );
    // The same for a module variable as the counter.
    let source = "Sub Process_Globals\n    Private g As Int = 2\nEnd Sub\n\n\
                  Sub AppStart (Args() As String)\n    For g = 1 To g\n        Log(g)\n    \
                  Next\nEnd Sub\n";
    assert_eq!(stdout(source), "1\n2\n");
}

#[test]
fn every_case_value_is_evaluated_before_matching() {
    let select = |scrutinee: i32| {
        stdout(&format!(
            "Sub Process_Globals\nEnd Sub\n\nSub AppStart (Args() As String)\n    \
             Select {scrutinee}\n        Case Value(1)\n            Log(\"one\")\n        \
             Case Value(2), Value(3)\n            Log(\"two or three\")\n        \
             Case Else\n            Log(\"else\")\n    End Select\nEnd Sub\n\n\
             Sub Value (N As Int) As Int\n    Log(N)\n    Return N\nEnd Sub\n"
        ))
    };
    assert_eq!(select(1), "1\n2\n3\none\n");
    assert_eq!(select(9), "1\n2\n3\nelse\n");
    // A value is kept as it was evaluated, even if a later one changes
    // the variable it read.
    let source = "Sub Process_Globals\n    Private x As Int = 1\nEnd Sub\n\n\
                  Sub AppStart (Args() As String)\n    Select 1\n        Case x, Bump\n            \
                  Log(\"matched\")\n    End Select\n    Log(x)\nEnd Sub\n\n\
                  Sub Bump As Int\n    x = 5\n    Return 5\nEnd Sub\n";
    assert_eq!(stdout(source), "matched\n5\n");
}
