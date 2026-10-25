# Recorder

`ferro-recorder` records the expected output of the conformance tests by
building and running them with the ORIGINAL toolchain. It is the only tool
allowed to write `*.expected` files (AGENTS.md).

It runs on a recording machine (Windows, with B4J and its JDK installed),
never in CI. It never changes the repository it reads: everything goes to an
output folder, which is then applied to the repository in a pull request.
It contains no B4X code, and B4X binaries are never committed or
redistributed.

## What it does, per test

1. **Selects the tests**: every test with `status = "pending-recording"`,
   `"deviation"` or `"negative"` (`--rerecord` adds the `recorded` ones;
   `--only` picks some).
2. **Wraps the test** in the smallest console project (`<test>.b4j`: an
   `AppType=StandardJava` header with `jcore`, the project attributes a new
   console project starts with, then the test's code unchanged), in
   `<out>/work/<test>/`.
3. **Builds it** with `B4JBuilder.exe -Task=Build -BaseFolder=<folder>
   -Project=<test>.b4j`, run in that folder. Exit code 0 is success.
4. **Runs** `Objects\<test>.jar` (or the only jar there) with the given Java,
   twice:
   - in the reference locale, `en-US`, forced for every locale category;
   - in the machine's own locale (for example `it-IT`), unless it is
     `en-US` already.

   Both runs write UTF-8 (`-Dstdout.encoding=UTF-8`), whatever the console's
   code page. Each run has a time limit (30 s by default).
5. **Writes the results** into `<out>` (see below), and rewrites the report
   after every test, so an interrupted run still leaves one.

What counts is what [docs/compatibility-testing.md](../../docs/compatibility-testing.md)
says: the output (Windows line ends become `\n`), the exit code, and the
*kind* of an uncaught error (the first `java.…Exception` / `…Error` line on
stderr), never its message.

## The output folder

| Path | What |
| --- | --- |
| `report.md` | one row per test, then the details: every difference from what the repository expected (the `.provisional` output, an earlier recording, how the program ends), the output in the machine's locale when it differs, anything written to stderr, builder output on a failure |
| `tests/conformance/…/<test>.expected` | the recorded output, in `en-US` |
| `tests/conformance/…/<test>.<locale>.expected` | the output in the machine's locale, only when it differs |
| `tests/conformance/…/<test>.meta.toml` | `status = "recorded"`, the `[recording]` table (toolchain version, platform, Java, locales, date), no `[provisional]` table; `exit_code` / `uncaught_error` corrected if the program ended otherwise |
| `remove.txt` | the `.provisional` files the recordings replace |
| `deviation-outputs/…/<test>.txt` | what a deviation test prints, to compare by hand with the reference output in its `.meta.toml` (deviation tests get no `.expected`) |
| `work/` | the generated projects, only with `--keep` |

A negative test the toolchain refuses gets `rejected = true` in its
`[recording]` table; one it builds is reported as a failure of the test.

## Running it

You need: B4J (its IDE configured once, so that the builder finds Java and
the libraries), its JDK, and a copy of this repository: a clone, or the ZIP
GitHub offers (Code → Download ZIP), unpacked.

The executable is built from this folder; it needs no Rust on the
recording machine:

```sh
cargo build --release -p ferro-recorder --target x86_64-pc-windows-gnu
# target/x86_64-pc-windows-gnu/release/ferro-recorder.exe
```

(On Linux this needs the MinGW-w64 linker, `gcc-mingw-w64-x86-64`, and
`CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER=x86_64-w64-mingw32-gcc`.)

First one test, keeping its project to look at:

```bat
ferro-recorder.exe --repo C:\ferrobasic --out C:\recordings ^
  --toolchain-version 10.30 --only control_flow --keep
```

`--toolchain-version` is the version Help → About shows. The defaults
assume B4J's usual folders; change them if yours differ:

| Option | Default |
| --- | --- |
| `--toolchain <folder>` | `C:\Program Files\Anywhere Software\B4J` |
| `--builder <file>` | `B4JBuilder.exe` in the toolchain folder |
| `--java <file>` | `C:\Java\jdk-19.0.2\bin\java.exe` |

If the builder does not accept `-BaseFolder`, add `--no-base-folder` (it
still runs in the project's folder). Then everything:

```bat
ferro-recorder.exe --repo C:\ferrobasic --out C:\recordings --toolchain-version 10.30
```

`--dry-run` lists the tests it would record; `--help` lists every option.
It exits with 0 when every test was recorded (disagreements are not
failures: they are what the report is for), 1 otherwise.

## Applying a recording

Copy `tests/` from the output folder over the repository, delete the files
`remove.txt` lists, and open a pull request with `report.md` in its
description. Every disagreement it lists is then fixed where it belongs, in
the interpreter or in `docs/research`, never in the recording. A
`ferro_rejects` test that now has an `.expected` file needs a decision
before the corpus accepts it.

## Licensing

B4J is free, including for commercial use ([b4x.com](https://www.b4x.com/b4j.html)).
Check that its licence permits automated use before running the recorder.
Never commit or redistribute B4X binaries.
