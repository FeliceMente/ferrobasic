//! The real process runner. The test binary runs itself as the child, with
//! one of the ignored helper tests below as its only test.

use std::path::Path;
use std::time::{Duration, Instant};

use ferro_recorder::{Processes, Runner};

fn helper(name: &str, limit: Duration) -> ferro_recorder::Run {
    let me = std::env::current_exe().unwrap();
    let args = [
        "--ignored",
        "--exact",
        name,
        "--nocapture",
        "--test-threads=1",
    ]
    .map(String::from);
    Processes
        .run(&me, &args, Path::new(env!("CARGO_TARGET_TMPDIR")), limit)
        .unwrap()
}

#[test]
fn captures_both_streams_and_the_exit_code() {
    let run = helper("talks", Duration::from_secs(60));
    assert_eq!(run.code, Some(0));
    assert!(!run.timed_out);
    let stdout = String::from_utf8_lossy(&run.stdout);
    assert!(
        stdout.contains(&"out-line ".repeat(10_000)),
        "{stdout:.200}"
    );
    assert!(String::from_utf8_lossy(&run.stderr).contains("err-line"));
    let run = helper("fails", Duration::from_secs(60));
    assert_ne!(run.code, Some(0));
}

#[test]
fn stops_a_program_at_its_time_limit() {
    let start = Instant::now();
    let run = helper("sleeps", Duration::from_millis(500));
    assert!(run.timed_out);
    assert_eq!(run.code, None);
    assert!(start.elapsed() < Duration::from_secs(20));
}

#[test]
fn reports_a_missing_program() {
    let e = Processes
        .run(
            Path::new("/nonexistent/tool"),
            &[],
            Path::new("."),
            Duration::from_secs(1),
        )
        .unwrap_err();
    assert!(e.to_string().contains("/nonexistent/tool"));
}

#[test]
#[ignore = "run by captures_both_streams_and_the_exit_code"]
fn talks() {
    // More than a pipe buffer, on both streams.
    println!("{}", "out-line ".repeat(10_000));
    eprintln!("{}", "err-line ".repeat(10_000));
}

#[test]
#[ignore = "run by captures_both_streams_and_the_exit_code"]
fn fails() {
    panic!("on purpose");
}

#[test]
#[ignore = "run by stops_a_program_at_its_time_limit"]
fn sleeps() {
    std::thread::sleep(Duration::from_secs(60));
}
