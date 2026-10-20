# Licence notices

`THIRD-PARTY-NOTICES.txt`, at the repository root, holds the copyright and
licence notices of the third-party code in `ferro`: the Rust crates it links,
the Rust standard library and, in static Linux builds, the musl C library.
`ferro` compiles it in, after FerroBASIC's own `LICENSE`, and `ferro licenses`
prints both. Every program built with `ferro bundle` contains `ferro`, so
whoever distributes one must include them too.

## Regenerating

After adding, removing or updating a dependency (any change to `Cargo.lock`):

```sh
cargo install --locked cargo-about --features cli   # once
tools/licenses/generate.sh
```

`cargo-about` is a development tool, not a dependency of `ferro`. The script
runs it offline on the crates in `Cargo.lock`, so the result depends on the
lock file alone. `notices_match_cargo_lock` in `crates/cli/tests/licenses.rs`
fails while the crate list in the notices and `Cargo.lock` disagree.

- `about.toml` lists the accepted licences, in order of preference: where a
  crate offers a choice, the first one it offers is used. It also points
  `cargo-about` at licence files it does not recognise on its own
  (`[<crate>.clarify]`); their checksums change with each new version of
  those crates, and `generate.sh` then fails until they are updated.
  `cargo about clarify <file> path <crate directory>` prints the new entry.
- `about.hbs` is the template. It also holds the notices that do not come
  from a crate in `Cargo.lock`: musl's `COPYRIGHT` (version 1.2.5), and the
  Rust standard library's. The latter are copied by hand from
  `share/doc/rust/COPYRIGHT-library.html` in the toolchain (`rustc --print
  sysroot`), keeping the parts linked on Linux, Windows and macOS and leaving
  out other platforms' crates and build- or test-only ones. After a Rust
  upgrade, compare that file with the template and update both the list and
  the Rust version it names.

The list covers every platform at once, so a given executable contains only
part of it. It also lists some crates `ferro` never links, such as those
`libsui` uses only on Windows: `cargo-about` cannot tell that `libsui` itself
is linked only on macOS.

## Releases

Binary releases ship `LICENSE` and `THIRD-PARTY-NOTICES.txt` next to `ferro`.
