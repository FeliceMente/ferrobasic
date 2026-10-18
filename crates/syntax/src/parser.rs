//! Recursive-descent parser building the lossless CST (`docs/spec/level0.ebnf`).
//!
//! Every token, trivia included, ends up in the tree, so the tree's text is
//! the input. Errors never stop the parse: the offending tokens go into an
//! `ERROR` node and parsing resumes at the next statement separator.

use rowan::{Checkpoint, GreenNode, GreenNodeBuilder, TextRange, TextSize};

use crate::SyntaxKind::{self, *};
use crate::lexer::{self, Token};
use crate::{SyntaxError, SyntaxNode};

/// The result of parsing: a tree and the syntax errors found.
#[derive(Debug, Clone)]
pub struct Parse {
    green: GreenNode,
    errors: Vec<SyntaxError>,
}

impl Parse {
    pub fn syntax(&self) -> SyntaxNode {
        SyntaxNode::new_root(self.green.clone())
    }

    pub fn errors(&self) -> &[SyntaxError] {
        &self.errors
    }
}

/// Parses a module: optional header, then members.
pub fn parse(text: &str) -> Parse {
    let (tokens, lex_errors) = lexer::lex(text);
    let mut starts = Vec::with_capacity(tokens.len() + 1);
    let mut offset = 0u32;
    for token in &tokens {
        starts.push(offset);
        offset += token.len;
    }
    starts.push(offset);
    let mut parser = Parser {
        text,
        tokens,
        starts,
        pos: 0,
        builder: GreenNodeBuilder::new(),
        errors: lex_errors
            .into_iter()
            .map(|e| SyntaxError {
                range: TextRange::empty(TextSize::from(e.offset)),
                message: e.message,
            })
            .collect(),
    };
    parser.source_file();
    let mut errors = parser.errors;
    errors.sort_by_key(|e| e.range.start());
    Parse {
        green: parser.builder.finish(),
        errors,
    }
}

/// Binding powers of the binary operators, lowest first. The relative order
/// of `&` and of `And`/`Or` is provisional (see `docs/open-questions.md`):
/// change it here only, when a recording settles it.
fn infix_binding_power(kind: SyntaxKind) -> Option<(u8, u8)> {
    Some(match kind {
        OR_KW => (1, 2),
        AND_KW => (3, 4),
        EQ | NEQ | LT | GT | LT_EQ | GT_EQ => (5, 6),
        AMP => (7, 8),
        PLUS | MINUS => (9, 10),
        STAR | SLASH | MOD_KW => (11, 12),
        _ => return None,
    })
}

/// Binding power of unary minus.
const PREFIX_BINDING_POWER: u8 = 13;

/// Keywords that B4X also accepts as names. `In` is evidenced by a B4X
/// library that compiles with a parameter named `In`; which other keywords
/// are contextual is open (see `docs/open-questions.md`).
const CONTEXTUAL_KEYWORDS: &[SyntaxKind] = &[IN_KW];

fn is_name_token(kind: SyntaxKind) -> bool {
    kind == IDENT || CONTEXTUAL_KEYWORDS.contains(&kind)
}

struct Parser<'t> {
    text: &'t str,
    tokens: Vec<Token>,
    /// Byte offset of each token, plus the end of the text.
    starts: Vec<u32>,
    /// Index of the next token to put in the tree.
    pos: usize,
    builder: GreenNodeBuilder<'static>,
    errors: Vec<SyntaxError>,
}

// Token access and tree building.
impl Parser<'_> {
    /// Index of the `n`-th significant token from `pos`.
    fn nth_index(&self, n: usize) -> Option<usize> {
        (self.pos..self.tokens.len())
            .filter(|&i| !self.tokens[i].kind.is_trivia())
            .nth(n)
    }

    fn nth(&self, n: usize) -> Option<SyntaxKind> {
        self.nth_index(n).map(|i| self.tokens[i].kind)
    }

    fn current(&self) -> Option<SyntaxKind> {
        self.nth(0)
    }

    fn at(&self, kind: SyntaxKind) -> bool {
        self.current() == Some(kind)
    }

    fn current_text(&self) -> &str {
        match self.nth_index(0) {
            Some(i) => &self.text[self.starts[i] as usize..self.starts[i + 1] as usize],
            None => "",
        }
    }

    /// End of a statement: line break, `:` or end of input.
    fn at_sep(&self) -> bool {
        matches!(self.current(), None | Some(NEWLINE | COLON))
    }

    fn push_token(&mut self) {
        let i = self.pos;
        let text = &self.text[self.starts[i] as usize..self.starts[i + 1] as usize];
        self.builder.token(self.tokens[i].kind.into(), text);
        self.pos += 1;
    }

    fn eat_trivia(&mut self) {
        while self.pos < self.tokens.len() && self.tokens[self.pos].kind.is_trivia() {
            self.push_token();
        }
    }

    /// Adds the current significant token (and the trivia before it).
    fn bump(&mut self) {
        self.eat_trivia();
        if self.pos < self.tokens.len() {
            self.push_token();
        }
    }

    fn eat(&mut self, kind: SyntaxKind) -> bool {
        let found = self.at(kind);
        if found {
            self.bump();
        }
        found
    }

    fn expect(&mut self, kind: SyntaxKind, what: &str) -> bool {
        let found = self.eat(kind);
        if !found {
            self.error(format!("expected {what}"));
        }
        found
    }

    fn start(&mut self, kind: SyntaxKind) {
        self.eat_trivia();
        self.builder.start_node(kind.into());
    }

    fn checkpoint(&mut self) -> Checkpoint {
        self.eat_trivia();
        self.builder.checkpoint()
    }

    fn start_at(&mut self, checkpoint: Checkpoint, kind: SyntaxKind) {
        self.builder.start_node_at(checkpoint, kind.into());
    }

    fn finish(&mut self) {
        self.builder.finish_node();
    }

    /// Reports an error at the current token.
    fn error(&mut self, message: impl Into<String>) {
        let range = match self.nth_index(0) {
            Some(i) => TextRange::new(self.starts[i].into(), self.starts[i + 1].into()),
            None => TextRange::empty((*self.starts.last().unwrap()).into()),
        };
        self.errors.push(SyntaxError {
            range,
            message: message.into(),
        });
    }

    /// Puts the rest of the statement into an `ERROR` node.
    fn recover_to_sep(&mut self) {
        if self.at_sep() {
            return;
        }
        self.start(ERROR);
        while !self.at_sep() {
            self.bump();
        }
        self.finish();
    }

    /// Reports an error and skips to the end of the statement.
    fn error_and_recover(&mut self, message: impl Into<String>) {
        self.error(message);
        self.recover_to_sep();
    }

    /// Expects the end of a statement (without consuming it).
    fn expect_sep(&mut self) {
        if !self.at_sep() {
            self.error_and_recover("expected end of statement");
        }
    }

    /// `End <second>`.
    fn expect_end(&mut self, second: SyntaxKind, what: &str) {
        if self.at(END_KW) && self.nth(1) == Some(second) {
            self.bump();
            self.bump();
        } else {
            self.error(format!("expected `End {what}`"));
        }
    }

    fn at_sub_start(&self) -> bool {
        match self.current() {
            Some(SUB_KW) => true,
            Some(PRIVATE_KW | PUBLIC_KW) => self.nth(1) == Some(SUB_KW),
            _ => false,
        }
    }
}

// Module level.
impl Parser<'_> {
    fn source_file(&mut self) {
        self.builder.start_node(SOURCE_FILE.into());
        if self.at(HEADER) {
            self.bump();
        }
        loop {
            match self.current() {
                None => break,
                Some(NEWLINE | COLON) => self.bump(),
                Some(DIRECTIVE) => self.directive_line(),
                _ if self.at_sub_start() => self.sub_decl(),
                _ => self.error_and_recover("expected `Sub` or a `#` attribute"),
            }
        }
        // Trailing trivia, e.g. a final comment without a line break.
        self.eat_trivia();
        self.finish();
    }

    fn directive_line(&mut self) {
        let text = self.current_text()[1..].trim_start().to_ascii_lowercase();
        let words: Vec<&str> = text.split_whitespace().collect();
        let is_region = words.first() == Some(&"region")
            || (words.first() == Some(&"end") && words.get(1) == Some(&"region"));
        let is_attribute = {
            let name_len = text
                .bytes()
                .take_while(|b| b.is_ascii_alphanumeric() || *b == b'_')
                .count();
            name_len > 0 && text[name_len..].trim_start().starts_with(':')
        };
        let kind = if is_region {
            REGION
        } else if is_attribute {
            ATTRIBUTE
        } else {
            DIRECTIVE_LINE
        };
        self.start(kind);
        self.bump();
        self.finish();
    }

    fn sub_decl(&mut self) {
        self.start(SUB_DECL);
        if !self.eat(PRIVATE_KW) {
            self.eat(PUBLIC_KW);
        }
        self.expect(SUB_KW, "`Sub`");
        self.name();
        if self.at(L_PAREN) {
            self.param_list();
        }
        if self.eat(AS_KW) {
            self.type_ref();
        }
        self.expect_sep();
        self.block();
        self.expect_end(SUB_KW, "Sub");
        self.finish();
    }

    fn at_name(&self) -> bool {
        self.current().is_some_and(is_name_token)
    }

    fn name(&mut self) {
        if self.at_name() {
            self.start(NAME);
            self.bump();
            self.finish();
        } else {
            self.error("expected a name");
        }
    }

    fn param_list(&mut self) {
        self.start(PARAM_LIST);
        self.bump();
        if !self.at(R_PAREN) {
            loop {
                self.param();
                if !self.eat(COMMA) {
                    break;
                }
            }
        }
        self.expect(R_PAREN, "`)`");
        self.finish();
    }

    fn param(&mut self) {
        self.start(PARAM);
        self.name();
        if self.at(L_PAREN) && self.nth(1) == Some(R_PAREN) {
            self.bump();
            self.bump();
        }
        if self.eat(AS_KW) {
            self.type_ref();
        }
        self.finish();
    }

    /// `Name{.Name}`, optionally followed by `()` for an array type.
    fn type_ref(&mut self) {
        self.start(TYPE_REF);
        if self.expect(IDENT, "a type name") {
            while self.at(DOT) && self.nth(1) == Some(IDENT) {
                self.bump();
                self.bump();
            }
            if self.at(L_PAREN) && self.nth(1) == Some(R_PAREN) {
                self.bump();
                self.bump();
            }
        }
        self.finish();
    }
}

// Statements.
impl Parser<'_> {
    fn at_block_end(&self) -> bool {
        match self.current() {
            None | Some(END_KW | ELSE_KW | CASE_KW | NEXT_KW | LOOP_KW | CATCH_KW) => true,
            _ => self.at_sub_start(),
        }
    }

    fn block(&mut self) {
        self.start(BLOCK);
        loop {
            if matches!(self.current(), Some(NEWLINE | COLON)) {
                self.bump();
                continue;
            }
            if self.at_block_end() {
                break;
            }
            self.statement();
            self.expect_sep();
        }
        self.finish();
    }

    fn statement(&mut self) {
        match self.current() {
            Some(DIM_KW | PRIVATE_KW | PUBLIC_KW) => self.dim_stmt(),
            Some(TYPE_KW) => self.type_decl(),
            Some(IF_KW) => self.if_stmt(),
            Some(SELECT_KW) => self.select_stmt(),
            Some(FOR_KW) if self.nth(1) == Some(EACH_KW) => self.for_each_stmt(),
            Some(FOR_KW) => self.for_stmt(),
            Some(DO_KW) => self.do_stmt(),
            Some(TRY_KW) => self.try_stmt(),
            Some(DIRECTIVE) => self.directive_line(),
            _ => self.simple_statement(),
        }
    }

    /// Statements allowed after `Then` / `Else` of a single-line `If`.
    fn simple_statement(&mut self) {
        match self.current() {
            Some(DIM_KW | PRIVATE_KW | PUBLIC_KW) => self.dim_stmt(),
            Some(IF_KW) => self.if_stmt(),
            Some(RETURN_KW) => {
                self.start(RETURN_STMT);
                self.bump();
                if !self.at_sep() && !self.at(ELSE_KW) {
                    self.expr();
                }
                self.finish();
            }
            Some(EXIT_KW) => self.keyword_stmt(EXIT_STMT),
            Some(CONTINUE_KW) => self.keyword_stmt(CONTINUE_STMT),
            _ if self.at_expr_start() => self.assign_or_call(),
            Some(DO_KW | FOR_KW | SELECT_KW | TRY_KW) => {
                self.error_and_recover("block statements cannot follow `Then` on the same line")
            }
            _ => self.error_and_recover("expected a statement"),
        }
    }

    fn keyword_stmt(&mut self, kind: SyntaxKind) {
        self.start(kind);
        self.bump();
        self.finish();
    }

    fn dim_stmt(&mut self) {
        self.start(DIM_STMT);
        self.bump();
        loop {
            self.declarator();
            if !self.eat(COMMA) {
                break;
            }
        }
        self.finish();
    }

    /// `x [(bounds)] [As T] [= e]` or `x [(bounds)] = e [As T]`.
    fn declarator(&mut self) {
        self.start(DECLARATOR);
        self.name();
        if self.at(L_PAREN) {
            self.array_bounds();
        }
        if self.eat(AS_KW) {
            self.type_ref();
            if self.eat(EQ) {
                self.expr();
            }
        } else if self.eat(EQ) {
            self.expr();
            if self.eat(AS_KW) {
                self.type_ref();
            }
        }
        self.finish();
    }

    fn array_bounds(&mut self) {
        self.start(ARRAY_BOUNDS);
        self.bump();
        if !self.at(R_PAREN) {
            self.expr_list();
        }
        self.expect(R_PAREN, "`)`");
        self.finish();
    }

    fn type_decl(&mut self) {
        self.start(TYPE_DECL);
        self.bump();
        self.name();
        if self.expect(L_PAREN, "`(`") {
            loop {
                self.start(FIELD);
                self.name();
                if self.at(L_PAREN) {
                    self.array_bounds();
                }
                if self.eat(AS_KW) {
                    self.type_ref();
                }
                self.finish();
                if !self.eat(COMMA) {
                    break;
                }
            }
            self.expect(R_PAREN, "`)`");
        }
        self.finish();
    }

    fn if_stmt(&mut self) {
        self.start(IF_STMT);
        self.bump();
        self.expr();
        self.expect(THEN_KW, "`Then`");
        if self.at_sep() {
            self.block();
            loop {
                if self.at(ELSE_KW) && self.nth(1) == Some(IF_KW) {
                    self.start(ELSE_IF_CLAUSE);
                    self.bump();
                    self.bump();
                    self.expr();
                    self.expect(THEN_KW, "`Then`");
                    self.expect_sep();
                    self.block();
                    self.finish();
                } else if self.at(ELSE_KW) {
                    self.start(ELSE_CLAUSE);
                    self.bump();
                    self.expect_sep();
                    self.block();
                    self.finish();
                    break;
                } else {
                    break;
                }
            }
            self.expect_end(IF_KW, "If");
        } else {
            self.simple_statement();
            if self.at(ELSE_KW) {
                self.start(ELSE_CLAUSE);
                self.bump();
                self.simple_statement();
                self.finish();
            }
        }
        self.finish();
    }

    fn select_stmt(&mut self) {
        self.start(SELECT_STMT);
        self.bump();
        if self.at(CASE_KW) {
            self.error("`Select Case` is not supported: write `Select <expression>`");
            self.bump();
        }
        self.expr();
        self.expect_sep();
        loop {
            if matches!(self.current(), Some(NEWLINE | COLON)) {
                self.bump();
                continue;
            }
            if !self.at(CASE_KW) {
                break;
            }
            self.start(CASE_CLAUSE);
            self.bump();
            if !self.eat(ELSE_KW) {
                self.expr_list();
            }
            if self.at(TO_KW) {
                self.error_and_recover("`Case` ranges (`To`) are not supported");
            }
            self.expect_sep();
            self.block();
            self.finish();
        }
        self.expect_end(SELECT_KW, "Select");
        self.finish();
    }

    fn for_stmt(&mut self) {
        self.start(FOR_STMT);
        self.bump();
        self.name();
        if self.eat(AS_KW) {
            self.type_ref();
        }
        self.expect(EQ, "`=`");
        self.expr();
        self.expect(TO_KW, "`To`");
        self.expr();
        if self.eat(STEP_KW) {
            self.expr();
        }
        self.expect_sep();
        self.block();
        self.expect(NEXT_KW, "`Next`");
        self.finish();
    }

    fn for_each_stmt(&mut self) {
        self.start(FOR_EACH_STMT);
        self.bump();
        self.bump();
        self.name();
        if self.eat(AS_KW) {
            self.type_ref();
        }
        self.expect(IN_KW, "`In`");
        self.expr();
        self.expect_sep();
        self.block();
        self.expect(NEXT_KW, "`Next`");
        self.finish();
    }

    fn do_stmt(&mut self) {
        self.start(DO_STMT);
        self.bump();
        if self.eat(WHILE_KW) || self.eat(UNTIL_KW) {
            self.expr();
        } else {
            self.error("expected `While` or `Until` after `Do`: `Do ... Loop` without a condition is not supported");
        }
        self.expect_sep();
        self.block();
        if self.expect(LOOP_KW, "`Loop`") && matches!(self.current(), Some(WHILE_KW | UNTIL_KW)) {
            self.error_and_recover(
                "`Loop While` / `Loop Until` are not supported: put the condition after `Do`",
            );
        }
        self.finish();
    }

    fn try_stmt(&mut self) {
        self.start(TRY_STMT);
        self.bump();
        self.expect_sep();
        self.block();
        self.start(CATCH_CLAUSE);
        if self.expect(CATCH_KW, "`Catch`") {
            self.expect_sep();
            self.block();
        }
        self.finish();
        self.expect_end(TRY_KW, "Try");
        self.finish();
    }

    /// `target = value`, or an expression used as a statement (a call).
    fn assign_or_call(&mut self) {
        let checkpoint = self.checkpoint();
        self.postfix_expr();
        if self.at(EQ) {
            self.start_at(checkpoint, ASSIGN_STMT);
            self.bump();
            self.expr();
        } else {
            self.start_at(checkpoint, EXPR_STMT);
        }
        self.finish();
    }
}

// Expressions.
impl Parser<'_> {
    fn at_expr_start(&self) -> bool {
        self.current().is_some_and(|kind| {
            is_name_token(kind)
                || matches!(
                    kind,
                    ME_KW
                        | L_PAREN
                        | INT
                        | REAL
                        | STRING
                        | TRUE_KW
                        | FALSE_KW
                        | NULL_KW
                        | SMART_STRING_START
                        | MINUS
                )
        })
    }

    fn expr(&mut self) {
        self.expr_bp(0);
    }

    fn expr_list(&mut self) {
        loop {
            self.expr();
            if !self.eat(COMMA) {
                break;
            }
        }
    }

    fn expr_bp(&mut self, min_bp: u8) {
        let checkpoint = self.checkpoint();
        if self.at(MINUS) {
            self.start(PREFIX_EXPR);
            self.bump();
            self.expr_bp(PREFIX_BINDING_POWER);
            self.finish();
        } else {
            self.postfix_expr();
        }
        while let Some((left, right)) = self.current().and_then(infix_binding_power) {
            if left < min_bp {
                break;
            }
            self.start_at(checkpoint, BIN_EXPR);
            self.bump();
            self.expr_bp(right);
            self.finish();
        }
    }

    fn postfix_expr(&mut self) {
        let checkpoint = self.checkpoint();
        self.primary();
        loop {
            if self.at(DOT) {
                self.start_at(checkpoint, MEMBER_EXPR);
                self.bump();
                // Member names may be spelled like keywords: `Bit.And`, `x.As(T)`.
                if self.current().is_some_and(|k| k == IDENT || k.is_keyword()) {
                    self.start(NAME_REF);
                    self.bump();
                    self.finish();
                } else {
                    self.error("expected a member name after `.`");
                }
                self.finish();
            } else if self.at(L_PAREN) {
                self.start_at(checkpoint, CALL_EXPR);
                self.arg_list();
                self.finish();
            } else {
                break;
            }
        }
    }

    fn arg_list(&mut self) {
        self.start(ARG_LIST);
        self.bump();
        if !self.at(R_PAREN) {
            self.expr_list();
        }
        self.expect(R_PAREN, "`)`");
        self.finish();
    }

    fn primary(&mut self) {
        match self.current() {
            Some(INT | REAL | STRING | TRUE_KW | FALSE_KW | NULL_KW) => {
                self.start(LITERAL);
                self.bump();
                self.finish();
            }
            Some(IDENT)
                if self.current_text().eq_ignore_ascii_case("Array")
                    && self.nth(1) == Some(AS_KW) =>
            {
                self.start(ARRAY_EXPR);
                self.bump();
                self.bump();
                self.type_ref();
                if self.at(L_PAREN) {
                    self.arg_list();
                } else {
                    self.error("expected `(` after `Array As <type>`");
                }
                self.finish();
            }
            Some(IDENT)
                if self.current_text().eq_ignore_ascii_case("CreateMap")
                    && self.nth(1) == Some(L_PAREN) =>
            {
                self.create_map();
            }
            Some(kind) if is_name_token(kind) => {
                self.start(NAME_REF);
                self.bump();
                self.finish();
            }
            Some(ME_KW) => {
                self.start(ME_EXPR);
                self.bump();
                self.finish();
            }
            Some(L_PAREN) => {
                self.start(PAREN_EXPR);
                self.bump();
                self.expr();
                self.expect(R_PAREN, "`)`");
                self.finish();
            }
            Some(SMART_STRING_START) => self.smart_string(),
            _ => {
                self.error("expected an expression");
                if !self.at_sep() && !self.at_block_end() {
                    self.start(ERROR);
                    self.bump();
                    self.finish();
                }
            }
        }
    }

    /// `CreateMap(k1: v1, k2: v2)`; here `:` is not a statement separator.
    fn create_map(&mut self) {
        self.start(CREATE_MAP_EXPR);
        self.bump();
        self.bump();
        if !self.at(R_PAREN) {
            loop {
                self.start(MAP_ENTRY);
                self.expr();
                self.expect(COLON, "`:` between key and value");
                self.expr();
                self.finish();
                if !self.eat(COMMA) {
                    break;
                }
            }
        }
        self.expect(R_PAREN, "`)`");
        self.finish();
    }

    fn smart_string(&mut self) {
        self.start(SMART_STRING);
        self.bump();
        loop {
            match self.current() {
                Some(SMART_STRING_TEXT) => self.bump(),
                Some(PLACEHOLDER_START) => {
                    self.start(PLACEHOLDER);
                    self.bump();
                    self.expr();
                    if !self.eat(PLACEHOLDER_END) {
                        self.error("expected `}` to close the placeholder");
                        while !matches!(
                            self.current(),
                            None | Some(PLACEHOLDER_END | SMART_STRING_END)
                        ) {
                            self.bump();
                        }
                        self.eat(PLACEHOLDER_END);
                    }
                    self.finish();
                }
                Some(SMART_STRING_END) => {
                    self.bump();
                    break;
                }
                // Unterminated: the lexer already reported it.
                _ => break,
            }
        }
        self.finish();
    }
}
