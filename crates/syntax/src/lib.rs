//! Lexer, parser and lossless CST for the FerroBASIC (B4X-compatible) language.
//!
//! The grammar is `docs/spec/level0.ebnf`. [`parse`] never fails: it always
//! returns a tree whose text is exactly the input, plus the syntax errors.

mod kind;
pub mod lexer;
mod parser;

use std::fmt::Write;

pub use kind::{FerroLanguage, SyntaxKind};
pub use parser::{Parse, parse};
pub use rowan::{NodeOrToken, TextRange, TextSize};

/// A node of the lossless syntax tree.
pub type SyntaxNode = rowan::SyntaxNode<FerroLanguage>;
/// A token of the lossless syntax tree.
pub type SyntaxToken = rowan::SyntaxToken<FerroLanguage>;
/// A node or a token.
pub type SyntaxElement = rowan::SyntaxElement<FerroLanguage>;

/// A syntax error with its location in the source text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyntaxError {
    pub range: TextRange,
    pub message: String,
}

/// An indented dump of the tree, one node or token per line, for tests and
/// debugging. Trivia tokens are included.
pub fn debug_tree(node: &SyntaxNode) -> String {
    let mut out = String::new();
    let mut depth = 0;
    for event in node.preorder_with_tokens() {
        match event {
            rowan::WalkEvent::Enter(element) => {
                let indent = "  ".repeat(depth);
                match element {
                    rowan::NodeOrToken::Node(node) => {
                        writeln!(out, "{indent}{:?}", node.kind()).unwrap();
                        depth += 1;
                    }
                    rowan::NodeOrToken::Token(token) => {
                        writeln!(out, "{indent}{:?} {:?}", token.kind(), token.text()).unwrap();
                    }
                }
            }
            rowan::WalkEvent::Leave(rowan::NodeOrToken::Node(_)) => depth -= 1,
            rowan::WalkEvent::Leave(rowan::NodeOrToken::Token(_)) => {}
        }
    }
    out
}

#[cfg(test)]
mod tests;
