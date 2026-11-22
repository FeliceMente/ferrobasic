# macOS program embedding: libsui evaluation

Evaluation for [#146, task 5](https://github.com/FeliceMente/ferrobasic/issues/146),
22 November 2026. Baseline: `origin/dev` at
`3f09334ab7ba7ea33a99dd5fd15efd33059823ee`, FerroBASIC 0.1.3-dev.

**Recommendation: option 3, keep libsui and pin it exactly to `=0.16.4`,
retaining both protections from #132.** The owner adopted option 3:
`crates/cli/Cargo.toml` requires `libsui = "=0.16.4"`. Option 1
is unavailable: the locked libsui has no Cargo features. Removing unused
functionality requires an upstream change, a fork, or a replacement. The
measured release executable saving is about 1%, insufficient by itself to
justify owning a Mach-O writer and signer now.

## What FerroBASIC calls

[`standalone.rs`](../../crates/cli/src/standalone.rs) calls only
`Macho::from`, `write_section`, `build_and_sign` (Apple Silicon), `build`
(Intel), and `find_section`. It formats `libsui::Error` into an I/O error.

- Apple Silicon adds a 152-byte `LC_SEGMENT_64` command with one section,
  `__SUI,__ferro`, immediately before `__LINKEDIT`. It moves the latter's
  file bytes and VM address and adjusts load commands pointing into it.
  `apple_codesign::MachoSigner` rebuilds the ad-hoc signature in memory.
  The runtime reader uses `getsectdata` plus the dyld image slide.
- Intel takes a different path: libsui removes the old signature via
  `codesign` and a temporary file, appends a sentinel, length and package,
  and extends `__LINKEDIT`. FerroBASIC reserves that temporary path with
  `create_new` first. FerroBASIC's `sign_for_intel_mac` in
  [`main.rs`](../../crates/cli/src/main.rs) is passed to
  `standalone::write_standalone` as its `finish` callback: it signs the staged
  output with `codesign --force --sign -`, propagating failure. It does **not** use
  libsui's Intel `build_and_sign` temporary-file path. The runtime reader
  scans its executable on disk for the sentinel.
- Linux and Windows do not use libsui at all. They append FerroBASIC's
  package/trailer to the executable and read it back from the file's end.
  Neither PE icon editing nor ELF note writing is needed by FerroBASIC.

[`build.rs`](../../crates/cli/build.rs) supplies
`-Wl,-headerpad,0x1000` on macOS. `standalone::macho_header_fits` checks the
result before it is written: load commands must end before the first
file-backed section. Keep both. libsui itself does not check the available
header space; without it, its added command overwrote `__text`
([#131](https://github.com/FeliceMente/ferrobasic/issues/131), fixed by
[#132](https://github.com/FeliceMente/ferrobasic/pull/132)).

## Dependency audit at 0.16.4

The authority here is the **published, locked crate**, not libsui's current
main branch. Its archive checksum in `Cargo.lock` is
`3cd44dd0a322ec9fb05619e59067d1c767718238677369d9c5f004ea26a1184b`.
Inspect `Cargo.toml`, `Cargo.toml.orig`, `lib.rs`, `pe_edit.rs`,
`apple_codesign.rs` and `intel_mac.rs` under
`~/.cargo/registry/src/<registry>/libsui-0.16.4/` after `cargo fetch --locked`.
The [published sources](https://docs.rs/crate/libsui/0.16.4/source/) also
identify the version being discussed.

```sh
cargo tree --locked -p ferro-cli --target aarch64-apple-darwin -e normal
cargo tree --locked -p libsui --target aarch64-apple-darwin \
  -e normal --prefix none --no-dedupe | sort -u
```

The second command lists **43 transitive package/version pairs**, excluding
libsui itself, including proc macros and their dependencies (but excluding
build dependencies). These are reachable Cargo dependencies, not proof that
all their machine code survives linking. Several are shared with the REPL
or syntax crates.

| Direct dependency | Why libsui declares it | Used by FerroBASIC's macOS path? |
| --- | --- | --- |
| `zerocopy 0.7.35`, `derive` | Mach-O structures and endian fields, also PE structures | Yes, writer and signer. Includes `byteorder` and derive/proc-macro dependencies. |
| `sha2 0.10.9` | `apple_codesign.rs` page hashes | Yes on Apple Silicon. `digest`, `block-buffer`, `crypto-common`, `generic-array`, `typenum`, `cpufeatures`, `cfg-if` and ARM's `libc` dependency support it. Intel's signing is external. |
| `image 0.25.9`, only `bmp`, `ico`, `png` enabled | `PortableExecutable::set_icon` decodes, resizes and encodes Windows icons | No icon operations. Its unconditional `Error::ImageError` variant and error formatting still connect image error code to the macOS path. |
| `indexmap 2.14.2` | PE resource tables in `pe_edit.rs` | No. Pulls `equivalent` and `hashbrown 0.17.1`. |
| `thiserror 1.0.69` | PE error derives in `pe_edit.rs` | No PE errors; this version is also used independently by `filedescriptor`. |
| `object =0.36.3`, `build`, `read_core`, `elf`, `std` | Leftover from the former ELF builder | No executable `object::` references in `lib.rs` at this version: only a comment about the old implementation. `experiment.rs` uses it but is not the library entry point. Its enabled features add dependencies anyway. |
| `libc 0.2.191` | ELF runtime lookup | No direct use by the Mach-O writer/reader, which declare their own FFI. Still needed transitively by SHA-256 on arm64 and other FerroBASIC dependencies. |
| `windows 0.58.0` | Windows resource lookup, gated by `cfg(windows)` | Not in the macOS normal tree. Still represented in the all-platform lockfile/notices through libsui. |

The image chain comprises `bytemuck`, `byteorder-lite`, `moxcms`, `pxfm`,
`num-traits`, `png`, `bitflags`, `crc32fast`, `cfg-if`, `fdeflate`, `flate2`,
two versions of `miniz_oxide`, `adler2`, and `simd-adler32`. These implement
image handling and PNG compression, not package compression. The `object`
chain also enables hashing/container support through `hashbrown 0.14.5`,
`ahash`, `once_cell`, `zerocopy 0.8.63`, `indexmap`, `memchr` and `crc32fast`.
Some of these packages remain useful elsewhere in FerroBASIC after trimming.

There is **no dependency on the separate `apple-codesign` crate**.
`apple_codesign` is libsui's own approximately 250-line module, using SHA-256
and zerocopy. Removing all hashing would remove the Apple Silicon signer.

## Measurements

Measured on an Apple Silicon Mac, macOS 27.2.1 (build 26B131, reported by
`sw_vers`), using stable
`rustc 1.99.1 (58cf8f9c3 2026-10-13)`, the committed release profile
(`lto = "thin"`), explicit `aarch64-apple-darwin` and
`x86_64-apple-darwin` targets, and no extra `RUSTFLAGS`.
These are on-disk, uncompressed file lengths, not resident memory or archive
sizes. Intel programs were also executed through Rosetta; native Intel CI
is a separate check.

| Variant | arm64 `ferro` bytes | x86_64 `ferro` bytes | Notices bytes | Notices lines |
| --- | ---: | ---: | ---: | ---: |
| Locked libsui, committed notices | 2,792,528 | 2,803,656 | 112,067 | 2,642 |
| Mach-O-only scratch copy, original notices retained as a control | 2,772,752 | 2,791,928 | 112,067 | 2,642 |
| Same scratch copy, regenerated notices | 2,756,240 | 2,775,544 | 93,874 | 2,220 |

Total executable saving: **36,288 bytes (1.30%) on arm64**, **28,112 bytes
(1.00%) on Intel**. Notices save **18,193 bytes (16.23%) and 422 lines**;
the listed crate/version pairs in the `Rust crates:` section drop from
132 to 111. Count only that section, ending at its first blank line; the
later in-tree Unicode notice is not a crate/version pair. The file is embedded
in every `ferro` and bundled program by `include_str!`, so notices contribute
to executable size. Alignment, signature size and dead-code elimination
mean executable deltas do not equal the notices delta exactly. Most unused
image/PE/ELF code was already eliminated; do not interpret the roughly 40
Cargo dependencies as that many linked image libraries.

For the `prices_map.bas` smoke test, the actual signed standalone binaries
were 2,858,546 → 2,822,258 bytes on arm64 and 2,826,256 → 2,797,920 bytes on
Intel (baseline → scratch copy with regenerated notices).

The scratch libsui subtree has 16 transitive package/version pairs rather
than 43. Its direct dependencies are only `sha2` and `zerocopy` (with
`derive`). The full workspace lockfile loses 23 package/version pairs; two
of those (`zlib-rs`, `zerocopy-derive 0.8.63`) were already omitted from the
notices as unused. Shared packages explain why removing 27 entries from the
libsui subtree does not remove 27 packages from the notices.

### Method and reproducibility

All experiments took place in an exported copy under `/tmp`; the registry
source and the baseline repository's manifests, lockfile and notices were
left intact.

1. Export the baseline with `git archive` to a scratch directory, retaining
   `Cargo.lock`. Build both targets with:

   ```sh
   cargo build --offline --locked --release -p ferro-cli --target aarch64-apple-darwin
   cargo build --offline --locked --release -p ferro-cli --target x86_64-apple-darwin
   ```

   Save both binaries, the notices and lockfile outside that copy's build
   directory before modifying it. Count file bytes with `wc -c`, notices
   lines with `wc -l`.
2. In the scratch CLI manifest only, set
   `libsui = { version = "=0.16.4", default-features = false }`.
   Rebuilding arm64 produced a **byte-identical** binary, and `cmp` found
   identical lockfiles and normal dependency trees. Baseline binary SHA-256:
   `f99e7b961ddeec6396da42edaf29982dfd2efc1a3545ae208db5f4340991d5ea`.
   There is no `[features]` table and no optional dependency in libsui's
   published manifest, so there is no feature selection to make instead.
3. Copy the locked libsui source to a second scratch directory, retaining
   its licence files. Reduce its manifest to package name/version/edition,
   MIT licence, `[lib] path = "lib.rs"`, and just the original `sha2` and
   `zerocopy` dependencies. In `lib.rs`, retain:
   - the licence header, `size_of`, `Write`, and zerocopy imports;
   - `apple_codesign` and `intel_mac` modules, and re-export
     `macho::find_section`;
   - `Error` and its impls, removing only `ImageError`, its Display arm,
     and `From<image::ImageError>`;
   - the contiguous block from `SegmentCommand64`'s attributes through the
     end of `mod macho`, immediately before `pub struct Elf` (original
     `lib.rs` lines 438–1110). This preserves the Mach-O implementation;
     omit PE, ELF, utilities and the library's own test module.
   Leave `apple_codesign.rs` and `intel_mac.rs` unchanged. Point only the
   scratch CLI's macOS dependency to this path. This is a **measurement
   fork**, not option 1 or a proposed production patch.
4. Run the first build without `--locked`, offline, so Cargo removes unused
   lockfile entries without refreshing retained versions. Build both targets
   with the original notices for the control row. Then run
   `tools/licenses/generate.sh` with `cargo-about 0.9.3` (its `cli` feature
   enabled), and rebuild both with `--locked`. The same generator reproduced
   the committed baseline notices byte for byte before the experiment.
5. For baseline and scratch binaries on both architectures, bundle
   `tests/conformance/level0/prices_map.bas`, run the result, and require
   `codesign --verify --strict --verbose=2` to succeed. This is a smoke check
   of the measurement fork, not qualification of a replacement writer.

## Options and costs

### 1. Trim through features: unavailable at the locked version

`default-features = false` changes nothing. Selecting imaginary Mach-O or
signing features would be a Cargo error. The image crate already has its
defaults disabled by libsui; FerroBASIC cannot disable the `bmp`, `ico` and
`png` features libsui explicitly enables, because Cargo features are
additive. Target-gating libsui in FerroBASIC already excludes it from Linux
and Windows builds; it does not remove unconditional dependencies inside
libsui on macOS.

An upstream split could make PE/icon and ELF support optional, gate the
`ImageError` variant/imports too, and retain a Mach-O plus signer subset.
That requires a new upstream API/version and compatibility review, or a
maintained fork, rather than a small manifest-only change.
The scratch measurements quantify its possible benefit. No dependency
change is implemented here.

### 2. Replace with a small writer and ad-hoc signer

This is feasible for FerroBASIC's own thin, little-endian 64-bit executables,
but is more than appending a section and hashing the result. Proposed scope:

- Parse the header and commands with checked byte reads, checked arithmetic
  and explicit bounds. Accept only the supported arm64/x86_64 executable
  layouts; diagnose fat, swapped, malformed, unsupported or already embedded
  layouts. Preflight space for the 152-byte segment/section command and,
  if absent, a 16-byte `LC_CODE_SIGNATURE`. Retain header padding and the
  independent overlap check.
- Add a read-only segment/section before `__LINKEDIT` on both architectures.
  Preserve the original text/data VM addresses, entry point and file bytes;
  move `__LINKEDIT` by properly aligned file and VM deltas. Account for each
  supported reference into it: `LC_SYMTAB`, `LC_DYSYMTAB`,
  `LC_DYLD_INFO[_ONLY]`, and `linkedit_data_command` commands including
  exports, chained fixups, function starts, data-in-code and signature.
  Check chained-fixup metadata against the inserted segment index; moving
  a segment is not permission to assume every internal reference is a file
  offset. Unknown offset-bearing layouts must fail explicitly.
- Remove/rebuild the old signature, calculate the final signature location
  and size, and update `LC_CODE_SIGNATURE` and `__LINKEDIT` before hashing.
  Emit the big-endian SuperBlob/index/CodeDirectory, identifier, flags and
  executable-segment fields. Hash every covered 4 KiB signing page with
  SHA-256, including the final header and package and the short last page;
  encode the slot count, limit and offsets. An ad-hoc signature needs no
  certificate or CMS signature. It is **not one whole-file digest**. The
  signing-page size is distinct from VM/file segment alignment. Apple's
  [CodeDirectory structures](https://github.com/apple-oss-distributions/xnu/blob/main/osfmk/kern/cs_blobs.h)
  define the fields; libsui's implementation is a useful existing example.
- Add a matching reader on both architectures. Parsing `current_exe()` on
  disk into an owned package could avoid adding unsafe FFI to the CLI.
  Replacing Intel's sentinel format with a section also replaces its reader
  and temporary signature-removal path; it is not just swapping its writer.
  Preserve FerroBASIC package version 1 and staged-write failure handling.

Apple's [Mach-O command definitions](https://github.com/apple-oss-distributions/xnu/blob/main/EXTERNAL_HEADERS/mach-o/loader.h)
are the reference for those offset fields. Signing should provide equivalent
ad-hoc validation to `codesign -s -`, not necessarily byte-identical signature
metadata. Require strict verification and execution on both native Macs.

Estimate, not an implementation measurement: **700–1,100 lines of production
Rust plus 400–700 lines of focused tests**, keeping a maintained SHA-256
crate and using safe byte parsing/writing. Approximately 250 lines of the
existing source are signer structures/logic; the writer, offset fixups,
reader, validation and diagnostics make up the rest. A private vendored
Mach-O subset would be a similar maintenance commitment and must retain
upstream licence notices. Writing SHA-256 ourselves is unnecessary risk.

The main risks are another header overwrite, stale link-edit references,
incorrect alignment or hash coverage, Intel/arm64 differences, and linker
format evolution. Existing fixtures cover current output, not every future
load command. A follow-up needs malformed/truncated/overflow inputs,
insufficient padding, absent/existing signature, supported command fixups,
section reading and payloads across alignment/signing-page boundaries.
Development and review would take several days, with continued macOS upkeep;
implementation remains a separate project from this evaluation.

### 3. Keep and pin: adopted

Change the existing macOS-only dependency from `"0.16.4"` (a compatible
version range) to `"=0.16.4"`. This is now done. `Cargo.lock` already selects
0.16.4 with an archive checksum, so normal locked builds would not change.
An exact requirement also prevents a later compatible 0.16.x release from
being selected when updating/regenerating the lockfile. It does not pin
transitive versions without the lockfile, fix upstream defects, or replace
review/testing of future upgrades.

Cost: keep the current dependency/notice footprint and the Intel `codesign`
requirement and guarded temporary path. Benefit: retain the currently tested
writer/signer without acquiring a binary-format implementation for a
28–36 KB saving. Preserve `-headerpad` **and** `macho_header_fits`; pinning is
not a substitute for either. Revisit an upstream feature split if available,
or replacement if reduced dependency ownership becomes a product goal.

## Existing regression coverage and verification

The following tests would catch common failures in a changed writer:

| Test | What it detects |
| --- | --- |
| `bundle::binaries_are_validly_signed` | Requires `codesign --verify --strict --verbose=2` on macOS, including Intel. |
| `bundle::standalone_binaries_behave_like_ferro_run` | Bundles every corpus program with expected output, moves it away from the build tree, compares stdout against the recording, and stdout, stderr and exit status against `ferro run`; catches startup, reader and relocation failures. |
| `difftest::the_corpus_passes_as_standalone_binaries` | Full standalone conformance, including deviations and expected refusals; also run explicitly with `cargo run -p ferro-difftest -- --target standalone`. |
| `bundle::arguments_go_to_app_start`, `exit_codes_and_corrupt_binaries`, `default_output_is_the_file_stem` | Checks package discovery, argument routing and execution/exit behavior. The byte-corruption portion is deliberately disabled on macOS because it invalidates the signature. |
| `standalone::tests::load_commands_must_end_before_the_first_section`, `malformed_mach_o_headers_are_errors` | Synthetic coverage for #132's overlap guard and malformed headers; these test the guard, not every writer fixup. |
| `standalone::tests::packages_round_trip`, `payload_round_trips`, `truncated_or_padded_payloads_are_corrupt`, `trailers` | Package integrity independent of Mach-O layout. |
| `standalone::tests::staging_never_touches_other_files` and CLI `safety` tests | Staged finishing failure retains the old output; input/executable paths and non-regular files are protected. |

`cargo test --workspace` in the manually dispatched
[CI workflow](../../.github/workflows/ci.yml) includes the bundle and standalone
conformance integration tests on `macos-latest` (arm64) and `macos-15-intel`.
Cross-target clippy checks compilation, not signing or execution.
At evaluation commit `9992719`, local `scripts/check-all.sh --quick` passed
on Apple Silicon, with only the static Linux build skipped. Interpreter and
standalone conformance each reported 358 tests: 339 pass, 0 fail,
6 deviations, 13 pending, and no provisional outputs.
The [manual CI run](https://github.com/FeliceMente/ferrobasic/actions/runs/45008215191)
passed all nine jobs, including both native Mac architectures and the static
Linux build skipped locally. The baseline and scratch `prices_map.bas`
bundles also executed successfully and passed strict signature verification
on arm64 and x86_64 through Rosetta. These smoke checks do not claim full
qualification of the measurement fork.
