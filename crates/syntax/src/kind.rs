//! Token and node kinds of the lossless syntax tree.

/// Declares `SyntaxKind` and the tables derived from it, so the enum, the
/// raw-value table and the keyword table cannot drift apart.
macro_rules! syntax_kinds {
    (
        tokens { $($(#[$tok_attr:meta])* $tok:ident),* $(,)? }
        keywords { $($kw:ident = $text:literal),* $(,)? }
        nodes { $($(#[$node_attr:meta])* $node:ident),* $(,)? }
    ) => {
        /// Every token and node kind: tokens, then keywords, then nodes.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        #[repr(u16)]
        #[allow(non_camel_case_types)]
        pub enum SyntaxKind {
            $($(#[$tok_attr])* $tok,)*
            $($kw,)*
            $($(#[$node_attr])* $node,)*
        }

        /// All kinds in discriminant order, for `kind_from_raw`.
        const ALL_KINDS: &[SyntaxKind] = &[
            $(SyntaxKind::$tok,)*
            $(SyntaxKind::$kw,)*
            $(SyntaxKind::$node,)*
        ];

        /// Keyword kinds with their lower-case spelling.
        const KEYWORDS: &[(SyntaxKind, &str)] = &[$((SyntaxKind::$kw, $text),)*];
    };
}

syntax_kinds! {
    tokens {
        // Trivia: kept in the tree, skipped by the parser.
        BOM,
        WHITESPACE,
        COMMENT,
        /// ` _` at the end of a line: the `_`, any spaces after it and the line
        /// break it hides. The whitespace before `_` is a separate token.
        LINE_CONTINUATION,

        /// Module or project header: `Key=Value` lines up to `@EndOfDesignText@`.
        HEADER,
        /// A whole `#...` line: attribute, region marker or directive.
        DIRECTIVE,
        NEWLINE,
        COLON,
        IDENT,
        INT,
        REAL,
        STRING,
        SMART_STRING_START,
        SMART_STRING_TEXT,
        /// `$` + optional formatter + `{`.
        PLACEHOLDER_START,
        PLACEHOLDER_END,
        SMART_STRING_END,
        L_PAREN,
        R_PAREN,
        COMMA,
        DOT,
        EQ,
        NEQ,
        LT,
        GT,
        LT_EQ,
        GT_EQ,
        PLUS,
        MINUS,
        STAR,
        SLASH,
        AMP,
        /// A character the lexer does not recognise.
        ERROR_TOKEN,
    }
    keywords {
        AND_KW = "and",
        AS_KW = "as",
        CASE_KW = "case",
        CATCH_KW = "catch",
        CONTINUE_KW = "continue",
        DIM_KW = "dim",
        DO_KW = "do",
        EACH_KW = "each",
        ELSE_KW = "else",
        END_KW = "end",
        EXIT_KW = "exit",
        FALSE_KW = "false",
        FOR_KW = "for",
        IF_KW = "if",
        IN_KW = "in",
        LOOP_KW = "loop",
        ME_KW = "me",
        MOD_KW = "mod",
        NEXT_KW = "next",
        NULL_KW = "null",
        OR_KW = "or",
        PRIVATE_KW = "private",
        PUBLIC_KW = "public",
        RETURN_KW = "return",
        SELECT_KW = "select",
        STEP_KW = "step",
        SUB_KW = "sub",
        THEN_KW = "then",
        TO_KW = "to",
        TRUE_KW = "true",
        TRY_KW = "try",
        TYPE_KW = "type",
        UNTIL_KW = "until",
        WHILE_KW = "while",
    }
    nodes {
        SOURCE_FILE,
        ATTRIBUTE,
        REGION,
        DIRECTIVE_LINE,
        SUB_DECL,
        PARAM_LIST,
        PARAM,
        TYPE_REF,
        TYPE_DECL,
        FIELD,
        BLOCK,
        DIM_STMT,
        DECLARATOR,
        ARRAY_BOUNDS,
        IF_STMT,
        ELSE_IF_CLAUSE,
        ELSE_CLAUSE,
        SELECT_STMT,
        CASE_CLAUSE,
        FOR_STMT,
        FOR_EACH_STMT,
        DO_STMT,
        TRY_STMT,
        CATCH_CLAUSE,
        RETURN_STMT,
        EXIT_STMT,
        CONTINUE_STMT,
        ASSIGN_STMT,
        EXPR_STMT,
        /// A declared name.
        NAME,
        /// A use of a name.
        NAME_REF,
        LITERAL,
        ME_EXPR,
        PAREN_EXPR,
        PREFIX_EXPR,
        BIN_EXPR,
        /// `f(args)`: a call or an array index, told apart by `hir`.
        CALL_EXPR,
        ARG_LIST,
        /// `x.Name`; the name may be spelled like a keyword (`Bit.And`, `x.As`).
        MEMBER_EXPR,
        SMART_STRING,
        PLACEHOLDER,
        ARRAY_EXPR,
        CREATE_MAP_EXPR,
        MAP_ENTRY,
        ERROR,
    }
}

use SyntaxKind::*;

/// Longest keyword spelling, to bail out early in `from_keyword`.
const MAX_KEYWORD_LEN: usize = 8;

impl SyntaxKind {
    /// Tokens the parser skips; they stay in the tree.
    pub fn is_trivia(self) -> bool {
        matches!(self, BOM | WHITESPACE | COMMENT | LINE_CONTINUATION)
    }

    /// Whether this is a keyword token.
    pub fn is_keyword(self) -> bool {
        KEYWORDS.iter().any(|&(kind, _)| kind == self)
    }

    /// The keyword spelled `text`, compared case-insensitively.
    pub fn from_keyword(text: &str) -> Option<SyntaxKind> {
        if text.len() > MAX_KEYWORD_LEN || !text.is_ascii() {
            return None;
        }
        let mut buf = [0u8; MAX_KEYWORD_LEN];
        let lower = &mut buf[..text.len()];
        lower.copy_from_slice(text.as_bytes());
        lower.make_ascii_lowercase();
        // KEYWORDS is sorted by spelling (checked by a test).
        KEYWORDS
            .binary_search_by(|&(_, kw)| kw.as_bytes().cmp(lower))
            .ok()
            .map(|i| KEYWORDS[i].0)
    }
}

impl From<SyntaxKind> for rowan::SyntaxKind {
    fn from(kind: SyntaxKind) -> Self {
        rowan::SyntaxKind(kind as u16)
    }
}

/// The rowan language tag for FerroBASIC trees.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FerroLanguage {}

impl rowan::Language for FerroLanguage {
    type Kind = SyntaxKind;

    fn kind_from_raw(raw: rowan::SyntaxKind) -> SyntaxKind {
        match ALL_KINDS.get(raw.0 as usize) {
            Some(&kind) => kind,
            None => panic!("invalid syntax kind {}", raw.0),
        }
    }

    fn kind_to_raw(kind: SyntaxKind) -> rowan::SyntaxKind {
        kind.into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rowan::Language;

    #[test]
    fn raw_values_round_trip() {
        for (i, &kind) in ALL_KINDS.iter().enumerate() {
            assert_eq!(kind as usize, i);
            assert_eq!(FerroLanguage::kind_from_raw(kind.into()), kind);
        }
    }

    #[test]
    fn keyword_table_is_sorted_and_lower_case() {
        for pair in KEYWORDS.windows(2) {
            assert!(pair[0].1 < pair[1].1, "{} before {}", pair[0].1, pair[1].1);
        }
        for &(kind, text) in KEYWORDS {
            assert_eq!(text, text.to_ascii_lowercase());
            assert!(text.len() <= MAX_KEYWORD_LEN);
            assert_eq!(format!("{kind:?}"), format!("{}_KW", text.to_uppercase()));
        }
    }

    #[test]
    fn keywords_are_case_insensitive() {
        assert_eq!(SyntaxKind::from_keyword("END"), Some(END_KW));
        assert_eq!(SyntaxKind::from_keyword("eNd"), Some(END_KW));
        assert_eq!(SyntaxKind::from_keyword("While"), Some(WHILE_KW));
        assert_eq!(SyntaxKind::from_keyword("Log"), None);
        assert_eq!(SyntaxKind::from_keyword("Initialize"), None);
        assert!(END_KW.is_keyword());
        assert!(!IDENT.is_keyword());
    }
}
