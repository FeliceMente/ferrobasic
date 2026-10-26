//! Helpers shared by the integration tests of `ferro`.

use std::process::{Command, Output};

/// Runs an executable this test process just wrote. Linux refuses to run a
/// file that some process still has open for writing (`ETXTBSY`). Tests run
/// on several threads, and a child process another test is starting can
/// hold a copy of this test's write descriptor for a moment, between its
/// `fork` and its `exec`. So retry briefly instead of failing at random.
pub fn run_written(command: &mut Command) -> Output {
    for _ in 0..200 {
        match command.output() {
            Err(e) if e.kind() == std::io::ErrorKind::ExecutableFileBusy => {
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            result => return result.unwrap_or_else(|e| panic!("{command:?}: {e}")),
        }
    }
    panic!("{command:?}: still \"text file busy\" after 2 s");
}
