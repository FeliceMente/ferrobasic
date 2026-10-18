//! Token and node kinds of the lossless syntax tree.

/// Every token and node kind. Tokens come first, then nodes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u16)]
#[allow(non_camel_case_types)]
pub enum SyntaxKind {
    // Trivia: kept in the tree, ignored by the parser.
    BOM = 0,
    WHITESPACE,
    COMMENT,
    /// `_` at the end of a line, together with the line break it hides.
    LINE_CONTINUATION,

    // Significant tokens.
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

    // Keywords (case-insensitive).
    AND_KW,
    AS_KW,
    CASE_KW,
    CATCH_KW,
    CONTINUE_KW,
    DIM_KW,
    DO_KW,
    EACH_KW,
    ELSE_KW,
    END_KW,
    EXIT_KW,
    FALSE_KW,
    FOR_KW,
    IF_KW,
    IN_KW,
    LOOP_KW,
    ME_KW,
    MOD_KW,
    NEXT_KW,
    NULL_KW,
    OR_KW,
    PRIVATE_KW,
    PUBLIC_KW,
    RETURN_KW,
    SELECT_KW,
    STEP_KW,
    SUB_KW,
    THEN_KW,
    TO_KW,
    TRUE_KW,
    TRY_KW,
    TYPE_KW,
    UNTIL_KW,
    WHILE_KW,

    // Nodes.
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
    NAME,
    NAME_REF,
    LITERAL,
    ME_EXPR,
    PAREN_EXPR,
    PREFIX_EXPR,
    BIN_EXPR,
    CALL_EXPR,
    ARG_LIST,
    MEMBER_EXPR,
    SMART_STRING,
    PLACEHOLDER,
    ARRAY_EXPR,
    CREATE_MAP_EXPR,
    MAP_ENTRY,
    ERROR,
}

use SyntaxKind::*;

impl SyntaxKind {
    /// Tokens the parser skips; they stay in the tree.
    pub fn is_trivia(self) -> bool {
        matches!(self, BOM | WHITESPACE | COMMENT | LINE_CONTINUATION)
    }

    /// Whether this is a keyword token.
    pub fn is_keyword(self) -> bool {
        (AND_KW as u16..=WHILE_KW as u16).contains(&(self as u16))
    }

    /// The keyword spelled `text`, compared case-insensitively.
    pub fn from_keyword(text: &str) -> Option<SyntaxKind> {
        const KEYWORDS: &[(&str, SyntaxKind)] = &[
            ("and", AND_KW),
            ("as", AS_KW),
            ("case", CASE_KW),
            ("catch", CATCH_KW),
            ("continue", CONTINUE_KW),
            ("dim", DIM_KW),
            ("do", DO_KW),
            ("each", EACH_KW),
            ("else", ELSE_KW),
            ("end", END_KW),
            ("exit", EXIT_KW),
            ("false", FALSE_KW),
            ("for", FOR_KW),
            ("if", IF_KW),
            ("in", IN_KW),
            ("loop", LOOP_KW),
            ("me", ME_KW),
            ("mod", MOD_KW),
            ("next", NEXT_KW),
            ("null", NULL_KW),
            ("or", OR_KW),
            ("private", PRIVATE_KW),
            ("public", PUBLIC_KW),
            ("return", RETURN_KW),
            ("select", SELECT_KW),
            ("step", STEP_KW),
            ("sub", SUB_KW),
            ("then", THEN_KW),
            ("to", TO_KW),
            ("true", TRUE_KW),
            ("try", TRY_KW),
            ("type", TYPE_KW),
            ("until", UNTIL_KW),
            ("while", WHILE_KW),
        ];
        KEYWORDS
            .iter()
            .find(|(kw, _)| kw.eq_ignore_ascii_case(text))
            .map(|&(_, kind)| kind)
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
        assert!(raw.0 <= ERROR as u16, "invalid syntax kind {}", raw.0);
        // Safe alternative to a transmute: the kinds are dense from 0.
        ALL_KINDS[raw.0 as usize]
    }

    fn kind_to_raw(kind: SyntaxKind) -> rowan::SyntaxKind {
        kind.into()
    }
}

/// All kinds in discriminant order, for `kind_from_raw`.
const ALL_KINDS: &[SyntaxKind] = &[
    BOM,
    WHITESPACE,
    COMMENT,
    LINE_CONTINUATION,
    HEADER,
    DIRECTIVE,
    NEWLINE,
    COLON,
    IDENT,
    INT,
    REAL,
    STRING,
    SMART_STRING_START,
    SMART_STRING_TEXT,
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
    ERROR_TOKEN,
    AND_KW,
    AS_KW,
    CASE_KW,
    CATCH_KW,
    CONTINUE_KW,
    DIM_KW,
    DO_KW,
    EACH_KW,
    ELSE_KW,
    END_KW,
    EXIT_KW,
    FALSE_KW,
    FOR_KW,
    IF_KW,
    IN_KW,
    LOOP_KW,
    ME_KW,
    MOD_KW,
    NEXT_KW,
    NULL_KW,
    OR_KW,
    PRIVATE_KW,
    PUBLIC_KW,
    RETURN_KW,
    SELECT_KW,
    STEP_KW,
    SUB_KW,
    THEN_KW,
    TO_KW,
    TRUE_KW,
    TRY_KW,
    TYPE_KW,
    UNTIL_KW,
    WHILE_KW,
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
    NAME,
    NAME_REF,
    LITERAL,
    ME_EXPR,
    PAREN_EXPR,
    PREFIX_EXPR,
    BIN_EXPR,
    CALL_EXPR,
    ARG_LIST,
    MEMBER_EXPR,
    SMART_STRING,
    PLACEHOLDER,
    ARRAY_EXPR,
    CREATE_MAP_EXPR,
    MAP_ENTRY,
    ERROR,
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_kinds_is_dense_and_ordered() {
        for (i, kind) in ALL_KINDS.iter().enumerate() {
            assert_eq!(*kind as usize, i, "{kind:?} is out of place");
        }
        assert_eq!(*ALL_KINDS.last().unwrap(), ERROR);
    }

    #[test]
    fn keywords_are_case_insensitive() {
        assert_eq!(SyntaxKind::from_keyword("END"), Some(END_KW));
        assert_eq!(SyntaxKind::from_keyword("eNd"), Some(END_KW));
        assert_eq!(SyntaxKind::from_keyword("Log"), None);
        assert!(END_KW.is_keyword());
        assert!(!IDENT.is_keyword());
    }
}
