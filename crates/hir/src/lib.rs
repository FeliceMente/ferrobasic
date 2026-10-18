//! Name resolution and type checking for Level 0 (`docs/spec/level0.md`).
//!
//! [`check`] parses nothing itself: it takes a syntax tree without syntax
//! errors and returns the typed [`Module`] plus the semantic diagnostics.
//! The module is only meant to run when there are no diagnostics.

pub mod builtins;
pub mod hir;
mod lower;
pub mod ty;

pub use hir::Module;
pub use ty::Ty;

use ferro_syntax::{SyntaxNode, TextRange};

/// A semantic error, with its location in the source text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub range: TextRange,
    pub message: String,
}

/// Resolves names and types of a parsed module.
pub fn check(root: &SyntaxNode) -> (Module, Vec<Diagnostic>) {
    lower::lower(root)
}

#[cfg(test)]
mod tests;
