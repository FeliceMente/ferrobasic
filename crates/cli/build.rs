//! Link arguments for the `ferro` executable.
//!
//! On macOS, `ferro bundle` embeds the program with `libsui`, which adds a
//! load command (a segment with one section, 152 bytes) to a copy of the
//! running executable. The load commands must still end before the first
//! section's bytes, so `ferro` is linked with spare header space. Without it,
//! a toolchain that leaves little padding makes the new load command overwrite
//! the start of `__text`, and the bundle crashes at start-up (issue #131).
//! `ferro bundle` also checks the result (`standalone::macho_header_fits`).

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        // 4 KiB: room for libsui's load command, with a wide margin.
        println!("cargo:rustc-link-arg-bins=-Wl,-headerpad,0x1000");
    }
}
