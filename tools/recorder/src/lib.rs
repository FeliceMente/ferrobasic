//! Records the expected output of conformance tests by building and running
//! them with the original toolchain (`tools/recorder/README.md`). It is the
//! only tool that writes `*.expected` files.
//!
//! It never changes the repository it reads: everything goes to an output
//! folder that mirrors the repository (`tests/conformance/...`), with a
//! report, a list of files to delete, and the outputs of deviation tests.

pub mod diff;
pub mod java;
pub mod meta;
pub mod process;
pub mod project;

use std::fmt::Write as _;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::Duration;

use ferro_corpus::{Program, Status};
use java::Machine;
use meta::{Ending, Recording};
pub use process::{Processes, Run, Runner};

/// What to record, and with which tools.
#[derive(Debug, Clone)]
pub struct Config {
    /// The `tests/conformance` folder of a copy of the repository.
    pub corpus: PathBuf,
    /// Where everything is written; created if missing.
    pub out: PathBuf,
    /// The original toolchain's command-line builder.
    pub builder: PathBuf,
    pub java: PathBuf,
    /// The original toolchain's version, as its About box shows it.
    pub version: String,
    /// Tests to record (`level0/x`, or `x` when unique); empty: all of them.
    pub only: Vec<String>,
    /// Record `recorded` tests again too.
    pub rerecord: bool,
    /// Keep each test's generated project folder.
    pub keep: bool,
    pub dry_run: bool,
    /// Pass `-BaseFolder` to the builder (it also runs in that folder).
    pub base_folder: bool,
    pub build_timeout: Duration,
    pub run_timeout: Duration,
    /// Today, `YYYY-MM-DD`.
    pub date: String,
}

/// The outcome for one test.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// `.expected` and `.meta.toml` written.
    Recorded,
    /// A negative test the original toolchain refused, as it should.
    Rejected,
    /// A deviation test's output, written for comparison by hand.
    DeviationOutput,
    /// A negative test the original toolchain built: the test is wrong.
    NegativeAccepted,
    BuildFailed,
    NoJar,
    TimedOut,
    Failed,
}

impl Verdict {
    pub fn is_failure(self) -> bool {
        !matches!(
            self,
            Verdict::Recorded | Verdict::Rejected | Verdict::DeviationOutput
        )
    }

    fn label(self) -> &'static str {
        match self {
            Verdict::Recorded => "recorded",
            Verdict::Rejected => "rejected, as expected",
            Verdict::DeviationOutput => "output saved (deviation)",
            Verdict::NegativeAccepted => "**built, but the test expects a rejection**",
            Verdict::BuildFailed => "**build failed**",
            Verdict::NoJar => "**no jar after the build**",
            Verdict::TimedOut => "**timed out**",
            Verdict::Failed => "**failed**",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Entry {
    pub name: String,
    pub verdict: Verdict,
    /// Whether the recording disagrees with what the repository expected
    /// (the `.provisional` output, an earlier recording, or the metadata).
    pub disagrees: bool,
    /// Markdown for the report.
    pub notes: String,
}

#[derive(Debug, Clone, Default)]
pub struct Summary {
    pub entries: Vec<Entry>,
}

impl Summary {
    pub fn failures(&self) -> usize {
        self.entries
            .iter()
            .filter(|e| e.verdict.is_failure())
            .count()
    }
}

/// The tests `config` selects, in name order.
pub fn select(config: &Config) -> Result<Vec<Program>, String> {
    let all = ferro_corpus::programs_in(&config.corpus);
    if config.only.is_empty() {
        return Ok(all
            .into_iter()
            .filter(|p| config.rerecord || p.status != Status::Recorded)
            .collect());
    }
    let mut picked = Vec::new();
    for wanted in &config.only {
        let wanted = wanted.trim_end_matches(".bas").replace('\\', "/");
        let matches: Vec<&Program> = all
            .iter()
            .filter(|p| p.name == wanted || p.name.rsplit('/').next() == Some(wanted.as_str()))
            .collect();
        match matches[..] {
            [one] => picked.push(one.clone()),
            [] => return Err(format!("no conformance test is called {wanted}")),
            _ => return Err(format!("{wanted} names several tests: use its folder too")),
        }
    }
    picked.sort_by(|a, b| a.name.cmp(&b.name));
    picked.dedup_by(|a, b| a.name == b.name);
    Ok(picked)
}

/// Records every selected test; the report is rewritten after each one.
pub fn record(config: &Config, runner: &dyn Runner) -> Result<Summary, String> {
    let programs = select(config)?;
    let mut summary = Summary::default();
    if config.dry_run {
        for p in &programs {
            println!("{}", p.name);
        }
        println!("{} would be recorded", tests(programs.len()));
        return Ok(summary);
    }
    fs::create_dir_all(&config.out).map_err(|e| format!("{}: {e}", config.out.display()))?;
    let machine = machine(config, runner)?;
    let mut removals = Vec::new();
    for (i, program) in programs.iter().enumerate() {
        println!("[{}/{}] {}", i + 1, programs.len(), program.name);
        let entry = match record_one(config, runner, &machine, program, &mut removals) {
            Ok(entry) => entry,
            Err(e) => Entry {
                name: program.name.clone(),
                verdict: Verdict::Failed,
                disagrees: false,
                notes: format!("{e}\n"),
            },
        };
        println!("    {}", entry.verdict.label().trim_matches('*'));
        summary.entries.push(entry);
        write_report(config, &machine, &summary, &removals)
            .map_err(|e| format!("writing the report: {e}"))?;
    }
    let work = config.out.join("work");
    if !config.keep && work.is_dir() && fs::read_dir(&work).is_ok_and(|mut d| d.next().is_none()) {
        let _ = fs::remove_dir(&work);
    }
    Ok(summary)
}

/// Asks Java for its version, the platform and the machine's locale.
fn machine(config: &Config, runner: &dyn Runner) -> Result<Machine, String> {
    let args = [
        "-XshowSettings:properties".to_string(),
        "-version".to_string(),
    ];
    let run = runner
        .run(&config.java, &args, &config.out, Duration::from_secs(60))
        .map_err(|e| format!("running Java: {e}"))?;
    if run.code != Some(0) {
        return Err(format!(
            "{} -XshowSettings:properties -version failed:\n{}",
            config.java.display(),
            java::normalize(&run.stderr)
        ));
    }
    let text = format!(
        "{}{}",
        java::normalize(&run.stderr),
        java::normalize(&run.stdout)
    );
    Ok(Machine::from_properties(&java::properties(&text)))
}

/// What a program did.
struct Outcome {
    stdout: String,
    stderr: String,
    ending: Ending,
}

fn record_one(
    config: &Config,
    runner: &dyn Runner,
    machine: &Machine,
    program: &Program,
    removals: &mut Vec<String>,
) -> io::Result<Entry> {
    let mut entry = Entry {
        name: program.name.clone(),
        verdict: Verdict::Recorded,
        disagrees: false,
        notes: String::new(),
    };
    let stem = program
        .path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("test")
        .to_string();
    let work = config.out.join("work").join(program.file_name());
    if work.exists() {
        fs::remove_dir_all(&work)?;
    }
    fs::create_dir_all(&work)?;
    let project = format!("{stem}.b4j");
    fs::write(
        work.join(&project),
        project::project_file(&program.source()),
    )?;

    // Build.
    let mut args = vec!["-Task=Build".to_string()];
    if config.base_folder {
        args.push(format!("-BaseFolder={}", work.display()));
    }
    args.push(format!("-Project={project}"));
    let build = runner.run(&config.builder, &args, &work, config.build_timeout)?;
    let built = build.code == Some(0);
    let build_log = format!(
        "{}{}",
        java::normalize(&build.stdout),
        java::normalize(&build.stderr)
    );
    let result = if build.timed_out {
        entry.verdict = Verdict::TimedOut;
        note_block(
            &mut entry.notes,
            "The build took too long. Builder output:",
            &build_log,
        );
        None
    } else if program.status == Status::Negative {
        if built {
            entry.verdict = Verdict::NegativeAccepted;
            entry.disagrees = true;
            let _ = writeln!(
                entry.notes,
                "The original toolchain built it: the test, or its `negative` status, is wrong.\n"
            );
        } else {
            entry.verdict = Verdict::Rejected;
            let rec = recording(config, machine, None, true);
            write_meta(config, program, false, None, &rec)?;
        }
        None
    } else if !built {
        entry.verdict = Verdict::BuildFailed;
        note_block(&mut entry.notes, "Builder output:", &build_log);
        None
    } else {
        match find_jar(&work, &stem)? {
            Err(listing) => {
                entry.verdict = Verdict::NoJar;
                note_block(
                    &mut entry.notes,
                    "Contents of the project folder:",
                    &listing,
                );
                None
            }
            Ok(jar) => {
                let jar = jar.display().to_string();
                match run(config, runner, &work, &jar, true)? {
                    None => {
                        entry.verdict = Verdict::TimedOut;
                        None
                    }
                    Some(reference) => {
                        let machine_run = if machine.locale == java::REFERENCE_LOCALE
                            || machine.locale == "unknown"
                        {
                            None
                        } else {
                            run(config, runner, &work, &jar, false)?
                        };
                        Some((reference, machine_run))
                    }
                }
            }
        }
    };
    if let Some((reference, machine_run)) = result {
        save(
            config,
            machine,
            program,
            &reference,
            machine_run.as_ref(),
            &mut entry,
            removals,
        )?;
    }
    if !config.keep {
        fs::remove_dir_all(&work)?;
    }
    Ok(entry)
}

/// Runs the jar in the reference locale (`forced`) or the machine's;
/// `None` if it timed out.
fn run(
    config: &Config,
    runner: &dyn Runner,
    work: &Path,
    jar: &str,
    forced: bool,
) -> io::Result<Option<Outcome>> {
    let run = runner.run(
        &config.java,
        &java::run_args(jar, forced),
        work,
        config.run_timeout,
    )?;
    if run.timed_out {
        return Ok(None);
    }
    let stderr = java::normalize(&run.stderr);
    let ending = match (java::uncaught_kind(&stderr), run.code) {
        (Some(kind), _) => Ending::Uncaught(kind),
        (None, Some(0)) => Ending::Normal,
        (None, Some(code)) => Ending::Exit(code),
        (None, None) => Ending::Exit(-1),
    };
    Ok(Some(Outcome {
        stdout: java::normalize(&run.stdout),
        stderr,
        ending,
    }))
}

/// Writes a run's results and notes how they compare with the repository.
fn save(
    config: &Config,
    machine: &Machine,
    program: &Program,
    reference: &Outcome,
    machine_run: Option<&Outcome>,
    entry: &mut Entry,
    removals: &mut Vec<String>,
) -> io::Result<()> {
    let same = machine_run.map(|m| m.stdout == reference.stdout && m.ending == reference.ending);
    if same == Some(false) {
        entry.disagrees = true;
        let m = machine_run.expect("compared");
        let _ = writeln!(
            entry.notes,
            "In the machine's locale ({}) the output differs:\n",
            machine.locale
        );
        diff_block(&mut entry.notes, &reference.stdout, &m.stdout);
        if m.ending != reference.ending {
            let _ = writeln!(
                entry.notes,
                "and it ends with {} instead of {}.\n",
                describe(&m.ending),
                describe(&reference.ending)
            );
        }
    }
    if !reference.stderr.is_empty() {
        note_block(
            &mut entry.notes,
            "The program wrote to stderr:",
            &reference.stderr,
        );
    }
    if program.status == Status::Deviation {
        entry.verdict = Verdict::DeviationOutput;
        let base = out_file(&config.out.join("deviation-outputs"), &program.name, "txt");
        write(&base, &reference.stdout)?;
        if let (Some(false), Some(m)) = (same, machine_run) {
            let other = format!("{}.txt", machine.locale);
            write(
                &out_file(&config.out.join("deviation-outputs"), &program.name, &other),
                &m.stdout,
            )?;
        }
        let _ = writeln!(
            entry.notes,
            "Compare with the reference output in its `.meta.toml` ({}). It ends with {}.\n",
            program.deviation.as_deref().unwrap_or("deviation"),
            describe(&reference.ending)
        );
        note_block(&mut entry.notes, "Output:", &reference.stdout);
        return Ok(());
    }

    // What the repository expected: an earlier recording, or the
    // hand-derived output.
    let earlier = ["expected", "provisional"].iter().find_map(|ext| {
        let path = program.path.with_extension(ext);
        fs::read_to_string(&path).ok().map(|text| (*ext, text))
    });
    match &earlier {
        Some((ext, text)) => {
            let d = diff::diff(text, &reference.stdout);
            if d.is_empty() {
                let _ = writeln!(entry.notes, "Same output as the `.{ext}` file.\n");
            } else {
                entry.disagrees = true;
                let _ = writeln!(
                    entry.notes,
                    "**Differs from the `.{ext}` file** (`-` expected, `+` recorded):\n"
                );
                code_block(&mut entry.notes, &d);
            }
        }
        None => {
            let _ = writeln!(
                entry.notes,
                "No output was expected before (an open question).\n"
            );
        }
    }
    if program.path.with_extension("provisional").exists() {
        removals.push(format!("tests/conformance/{}.provisional", program.name));
    }
    let expected_ending = match (&program.uncaught_error, program.exit_code) {
        (Some(kind), _) => Ending::Uncaught(kind.clone()),
        (None, 0) => Ending::Normal,
        (None, code) => Ending::Exit(code),
    };
    let ending = (reference.ending != expected_ending).then_some(&reference.ending);
    if let Some(e) = ending {
        entry.disagrees = true;
        let _ = writeln!(
            entry.notes,
            "**The metadata said the program ends with {}; it ends with {}.** The `.meta.toml` now says so.\n",
            describe(&expected_ending),
            describe(e)
        );
    }
    if program.rejected {
        let _ = writeln!(
            entry.notes,
            "FerroBASIC rejects this test (`ferro_rejects`): decide what to do with it now that its output is known.\n"
        );
    }
    write(
        &out_file(&conformance_out(config), &program.name, "expected"),
        &reference.stdout,
    )?;
    if let (Some(false), Some(m)) = (same, machine_run) {
        let ext = format!("{}.expected", machine.locale);
        write(
            &out_file(&conformance_out(config), &program.name, &ext),
            &m.stdout,
        )?;
    }
    let rec = recording(config, machine, same, false);
    write_meta(config, program, true, ending, &rec)
}

fn recording(config: &Config, machine: &Machine, same: Option<bool>, rejected: bool) -> Recording {
    Recording {
        product: "B4J".into(),
        version: config.version.clone(),
        platform: machine.platform.clone(),
        java: machine.java.clone(),
        machine_locale: machine.locale.clone(),
        machine_output_same: same,
        rejected,
        date: config.date.clone(),
    }
}

fn write_meta(
    config: &Config,
    program: &Program,
    recorded: bool,
    ending: Option<&Ending>,
    rec: &Recording,
) -> io::Result<()> {
    let path = program.path.with_extension("meta.toml");
    let text = fs::read_to_string(&path)?;
    let text = meta::rewrite(&text, recorded, ending, rec);
    write(
        &out_file(&conformance_out(config), &program.name, "meta.toml"),
        &text,
    )
}

/// The jar the build wrote in `Objects`: `<stem>.jar` if it is there, else
/// the only jar. `Err` lists what the folder holds instead.
fn find_jar(work: &Path, stem: &str) -> io::Result<Result<PathBuf, String>> {
    let objects = work.join("Objects");
    let preferred = objects.join(format!("{stem}.jar"));
    if preferred.is_file() {
        return Ok(Ok(preferred));
    }
    let mut jars = Vec::new();
    if objects.is_dir() {
        for e in fs::read_dir(&objects)? {
            let path = e?.path();
            if path
                .extension()
                .is_some_and(|x| x.eq_ignore_ascii_case("jar"))
            {
                jars.push(path);
            }
        }
    }
    if let [jar] = &jars[..] {
        return Ok(Ok(jar.clone()));
    }
    let mut listing = String::new();
    list(work, work, &mut listing, 0)?;
    Ok(Err(listing))
}

fn list(root: &Path, dir: &Path, out: &mut String, depth: usize) -> io::Result<()> {
    let mut entries: Vec<_> = fs::read_dir(dir)?.collect::<Result<_, _>>()?;
    entries.sort_by_key(|e| e.file_name());
    for e in entries {
        let path = e.path();
        let rel = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .display()
            .to_string();
        out.push_str(&rel);
        if path.is_dir() {
            out.push_str("/\n");
            if depth < 3 {
                list(root, &path, out, depth + 1)?;
            }
        } else {
            out.push('\n');
        }
    }
    Ok(())
}

fn conformance_out(config: &Config) -> PathBuf {
    config.out.join("tests").join("conformance")
}

/// `base/<name>.<ext>`, with `name`'s `/` as folders.
fn out_file(base: &Path, name: &str, ext: &str) -> PathBuf {
    let mut path = base.to_path_buf();
    let mut parts = name.split('/').peekable();
    while let Some(part) = parts.next() {
        if parts.peek().is_some() {
            path.push(part);
        } else {
            path.push(format!("{part}.{ext}"));
        }
    }
    path
}

fn write(path: &Path, text: &str) -> io::Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    fs::write(path, text)
}

fn describe(ending: &Ending) -> String {
    match ending {
        Ending::Normal => "exit code 0".into(),
        Ending::Exit(code) => format!("exit code {code}"),
        Ending::Uncaught(kind) => format!("an uncaught `{kind}`"),
    }
}

/// How many lines of a program's or the builder's output the report shows:
/// a stack overflow's trace has thousands.
const MAX_LINES: usize = 40;

fn note_block(notes: &mut String, title: &str, text: &str) {
    let _ = writeln!(notes, "{title}\n");
    let lines = text.lines().count();
    if text.is_empty() {
        code_block(notes, "(nothing)\n");
    } else if lines <= MAX_LINES {
        code_block(notes, text);
    } else {
        let mut cut: String = text
            .lines()
            .take(MAX_LINES)
            .flat_map(|l| [l, "\n"])
            .collect();
        let _ = writeln!(cut, "… {} more lines", lines - MAX_LINES);
        code_block(notes, &cut);
    }
}

/// `1 test`, `2 tests`.
pub fn tests(n: usize) -> String {
    format!("{n} test{}", if n == 1 { "" } else { "s" })
}

fn diff_block(notes: &mut String, old: &str, new: &str) {
    code_block(notes, &diff::diff(old, new));
}

fn code_block(notes: &mut String, text: &str) {
    let fence = if text.contains("```") { "~~~~" } else { "```" };
    let _ = writeln!(
        notes,
        "{fence}text\n{}{}{fence}\n",
        text,
        if text.ends_with('\n') { "" } else { "\n" }
    );
}

fn write_report(
    config: &Config,
    machine: &Machine,
    summary: &Summary,
    removals: &[String],
) -> io::Result<()> {
    let mut r = String::new();
    let _ = writeln!(r, "# Recording report\n");
    let _ = writeln!(
        r,
        "- Original toolchain: {} (builder `{}`)",
        config.version,
        config.builder.display()
    );
    let _ = writeln!(r, "- Java: {} (`{}`)", machine.java, config.java.display());
    let _ = writeln!(r, "- Platform: {}", machine.platform);
    let _ = writeln!(
        r,
        "- Locale: {} (forced) and the machine's, {}",
        java::REFERENCE_LOCALE,
        machine.locale
    );
    let _ = writeln!(r, "- Date: {}\n", config.date);
    let failures = summary.failures();
    let disagreements = summary.entries.iter().filter(|e| e.disagrees).count();
    let _ = writeln!(
        r,
        "{}: {} failed, {} disagree with what the repository expected.\n",
        tests(summary.entries.len()),
        failures,
        disagreements
    );
    let _ = writeln!(r, "| Test | Result | Disagrees |\n| --- | --- | --- |");
    for e in &summary.entries {
        let _ = writeln!(
            r,
            "| `{}` | {} | {} |",
            e.name,
            e.verdict.label(),
            if e.disagrees { "**yes**" } else { "" }
        );
    }
    let _ = writeln!(r);
    for e in &summary.entries {
        if !e.notes.is_empty() {
            let _ = write!(r, "## `{}`\n\n{}", e.name, e.notes);
        }
    }
    let _ = writeln!(r, "## Files\n");
    let _ = writeln!(
        r,
        "- `tests/conformance/`: the new `.expected` and `.meta.toml` files, at their place in the repository."
    );
    let _ = writeln!(
        r,
        "- `deviation-outputs/`: what the deviation tests print, to compare by hand."
    );
    let _ = writeln!(
        r,
        "- `remove.txt`: the `.provisional` files the recordings replace."
    );
    fs::write(config.out.join("report.md"), r)?;
    let mut list = removals.join("\n");
    if !list.is_empty() {
        list.push('\n');
    }
    fs::write(config.out.join("remove.txt"), list)
}
