//! Lexer, parser and lossless CST for the FerroBASIC (B4X-compatible) language.
//!
//! Status: syntax kinds only; lexer and parser in progress. See
//! `docs/spec/level0.ebnf` and `AGENTS.md`.

mod kind;

pub use kind::{FerroLanguage, SyntaxKind};

/// A node of the lossless syntax tree.
pub type SyntaxNode = rowan::SyntaxNode<FerroLanguage>;
/// A token of the lossless syntax tree.
pub type SyntaxToken = rowan::SyntaxToken<FerroLanguage>;
