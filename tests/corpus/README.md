# Real-world corpus

Open source B4X projects used to measure how many real projects FerroBASIC can
parse, type-check, build and run unchanged.

The projects are referenced, never copied into this repository.
`corpus.toml` gives, for each one, where it was published at a fixed
commit, its licence, and the SHA-256 of each file the tests use. To get
them:

```sh
cargo run -p ferro-corpus --bin ferro-corpus-fetch       # into target/corpus
cargo test -p ferro-syntax --test projects -- --ignored  # the tests that need them
```

`ferro-corpus-fetch` downloads each file with `curl` from
`raw.githubusercontent.com` at the commit recorded, and keeps it only if its
SHA-256 is the one recorded; a file already fetched and right is kept. Every
read checks the hash again, so a test never runs on another file. Without
the files, those tests fail and name the command above. They are
`#[ignore]`d, so `cargo test --workspace` needs no network; CI runs them
(`.github/workflows/ci.yml`, job "real-world corpus").

Rules:

- Only reference projects whose licence allows redistribution, from a
  repository that publishes them under it: `license` and `license_url`
  record it.
- Leave out a project whose code another author may own (code copied from a
  forum post, a repository that bundles other people's libraries), and one
  that holds credentials.
- Reference a full commit, never a branch, and only the files the tests use
  (`.b4j`, `.bas`, `.b4j.meta`). Never commit them, nor third-party
  libraries (`.jar`, `.xml`, `.b4xlib`) or B4X binaries.
- A project that FerroBASIC cannot handle is a finding: never patch its files.

`ferro_corpus::projects()` reads `corpus.toml` strictly: every key a project
needs, full commits, GitHub URLs, file names without paths, 64-digit hashes.
Today every project's `.b4j` and `.bas` files must round-trip through the
lossless syntax tree (`crates/syntax/tests/projects.rs`).

## Projects

| Project | Licence | What it is | Needs beyond Level 0 |
| --- | --- | --- | --- |
| [`juros`](https://github.com/jacknpoe/todos_juros) | GPL-3.0 | interest from a price increase, by bisection | nothing: it passes `ferro check` |
| [`helloworld`](https://github.com/rwbl/b4j-howtos) | GPL-3.0 | hello world with the date | `DateTime` |
| [`console-read-input`](https://github.com/pyhoon/console-read-input-b4j) | MIT | reads lines typed on the console | `JavaObject` |
| [`vcardsplitter`](https://github.com/axtens/vcardsplitter) | MIT | splits a file of vCards into one file per card | a module, jStringFunctions, jStringUtils |
| [`mteeval-buildlib`](https://github.com/macthomasengineering/MteEVAL-b4x-library) | BSD-3-Clause | an expression compiler and its tests | four modules, a class; `.08` number literals |
| [`jsonwebtoken`](https://github.com/pyhoon/jsonwebtoken-b4j) | MIT | a JSON Web Token class | a class, `JavaObject`, jStringUtils, the java-jwt and Jackson jars |
