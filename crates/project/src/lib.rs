//! Reader for B4X project files (.b4j/.b4a/.b4i/.b4r) and .b4xlib archives.
//!
//! Status: stub. See `docs/architecture.md` and `AGENTS.md`.

/// Returns the crate version; placeholder until real functionality lands.
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    #[test]
    fn version_is_set() {
        assert!(!super::version().is_empty());
    }
}
