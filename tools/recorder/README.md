# Recorder

Records the expected output of conformance tests by running them on the
ORIGINAL B4X toolchain. It is the only tool allowed to write `*.expected` files.

It runs on a dedicated recording machine (Windows, B4J installed), never in CI.

## Planned behaviour

1. For each `tests/conformance/**/<name>.bas` with `status = "pending-recording"`:
   1. Wrap the code in a minimal B4J console project (template in `template/`).
   2. Build it with the B4J command-line builder (to confirm: see
      `docs/open-questions.md`).
   3. Run it on the JVM, capturing `Log` output, exit status and uncaught exceptions.
   4. Write `<name>.expected` and fill the `[recording]` table in `<name>.meta.toml`
      (B4X version, platform, date); set `status = "recorded"`.
2. Re-record everything when a new B4X version is released and report the diff.

## Licensing

Check that the B4X licence permits automated use before running it. Never
commit or redistribute B4X binaries.
