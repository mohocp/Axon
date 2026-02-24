//! # al-lexer
//!
//! Lexer for AgentLang MVP v0.1.
//!
//! Tokenises AgentLang source text according to the rules laid out in
//! `specs/GRAMMAR_MVP.ebnf`.  The lexer is responsible for:
//!
//! - Producing a flat `Vec<Spanned<Token>>` from UTF-8 source text.
//! - Tracking bracket nesting so that NEWLINE tokens are suppressed inside
//!   `()`, `[]`, and `{}`.
//! - Collapsing consecutive physical line breaks into a single `NEWLINE`.
//! - Suppressing NEWLINE after tokens that cannot end a statement and before
//!   tokens that continue an expression.
//! - Recognising all keywords, operators, and literal forms specified by the
//!   grammar.

use al_diagnostics::{Diagnostic, ErrorCode, Span};

// ---------------------------------------------------------------------------
// Token
// ---------------------------------------------------------------------------

/// All token types produced by the AgentLang lexer.
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // ── Keywords ──────────────────────────────────────────────────────
    Type,
    Schema,
    Agent,
    Operation,
    Pipeline,
    Body,
    Input,
    Output,
    Require,
    Ensure,
    Invariant,
    Store,
    Mutable,
    Match,
    When,
    Otherwise,
    Loop,
    Emit,
    Assert,
    Retry,
    Escalate,
    Checkpoint,
    Resume,
    Halt,
    Delegate,
    To,
    Fork,
    Join,
    Success,
    Failure,
    True,
    False,
    None,
    And,
    Or,
    Not,
    Eq,
    Neq,
    Gt,
    Gte,
    Lt,
    Lte,

    // ── Operators / Punctuation ───────────────────────────────────────
    /// `->`
    Arrow,
    /// `|>`
    PipeForward,
    /// `=>`
    FatArrow,
    /// `:`
    Colon,
    /// `::`
    DoubleColon,
    /// `?`
    Question,
    /// `@`
    At,
    /// `#`
    Hash,
    /// `..`
    DotDot,
    /// `.`
    Dot,
    /// `,`
    Comma,
    /// `;`
    Semicolon,
    /// `+`
    Plus,
    /// `-`
    Minus,
    /// `*`
    Star,
    /// `/`
    Slash,
    /// `%`
    Percent,
    /// `=`
    Equals,
    /// `(`
    LParen,
    /// `)`
    RParen,
    /// `[`
    LBracket,
    /// `]`
    RBracket,
    /// `{`
    LBrace,
    /// `}`
    RBrace,
    /// `|`
    Pipe,

    // ── Literals ──────────────────────────────────────────────────────
    /// Integer literal, e.g. `42`, `0xFF`, `0b1010`
    Integer(i64),
    /// Floating-point literal, e.g. `3.14`, `1.0e-10`
    Float(f64),
    /// String literal (contents without surrounding quotes)
    StringLit(String),
    /// Duration literal, e.g. `5s`, `100ms`, `2m`, `1h`
    Duration(String),
    /// Size literal, e.g. `256KB`, `1MB`, `4GB`
    Size(String),
    /// Confidence literal, e.g. `~0.95`
    Confidence(f64),
    /// Hash literal, e.g. `SHA256:abcdef1234...`
    HashLit(String),

    // ── Special ───────────────────────────────────────────────────────
    /// An identifier (not matching any keyword)
    Identifier(String),
    /// A logical newline (significant for statement termination)
    Newline,
    /// End of file
    Eof,
}

impl std::fmt::Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Token::Type => write!(f, "TYPE"),
            Token::Schema => write!(f, "SCHEMA"),
            Token::Agent => write!(f, "AGENT"),
            Token::Operation => write!(f, "OPERATION"),
            Token::Pipeline => write!(f, "PIPELINE"),
            Token::Body => write!(f, "BODY"),
            Token::Input => write!(f, "INPUT"),
            Token::Output => write!(f, "OUTPUT"),
            Token::Require => write!(f, "REQUIRE"),
            Token::Ensure => write!(f, "ENSURE"),
            Token::Invariant => write!(f, "INVARIANT"),
            Token::Store => write!(f, "STORE"),
            Token::Mutable => write!(f, "MUTABLE"),
            Token::Match => write!(f, "MATCH"),
            Token::When => write!(f, "WHEN"),
            Token::Otherwise => write!(f, "OTHERWISE"),
            Token::Loop => write!(f, "LOOP"),
            Token::Emit => write!(f, "EMIT"),
            Token::Assert => write!(f, "ASSERT"),
            Token::Retry => write!(f, "RETRY"),
            Token::Escalate => write!(f, "ESCALATE"),
            Token::Checkpoint => write!(f, "CHECKPOINT"),
            Token::Resume => write!(f, "RESUME"),
            Token::Halt => write!(f, "HALT"),
            Token::Delegate => write!(f, "DELEGATE"),
            Token::To => write!(f, "TO"),
            Token::Fork => write!(f, "FORK"),
            Token::Join => write!(f, "JOIN"),
            Token::Success => write!(f, "SUCCESS"),
            Token::Failure => write!(f, "FAILURE"),
            Token::True => write!(f, "TRUE"),
            Token::False => write!(f, "FALSE"),
            Token::None => write!(f, "NONE"),
            Token::And => write!(f, "AND"),
            Token::Or => write!(f, "OR"),
            Token::Not => write!(f, "NOT"),
            Token::Eq => write!(f, "EQ"),
            Token::Neq => write!(f, "NEQ"),
            Token::Gt => write!(f, "GT"),
            Token::Gte => write!(f, "GTE"),
            Token::Lt => write!(f, "LT"),
            Token::Lte => write!(f, "LTE"),
            Token::Arrow => write!(f, "->"),
            Token::PipeForward => write!(f, "|>"),
            Token::FatArrow => write!(f, "=>"),
            Token::Colon => write!(f, ":"),
            Token::DoubleColon => write!(f, "::"),
            Token::Question => write!(f, "?"),
            Token::At => write!(f, "@"),
            Token::Hash => write!(f, "#"),
            Token::DotDot => write!(f, ".."),
            Token::Dot => write!(f, "."),
            Token::Comma => write!(f, ","),
            Token::Semicolon => write!(f, ";"),
            Token::Plus => write!(f, "+"),
            Token::Minus => write!(f, "-"),
            Token::Star => write!(f, "*"),
            Token::Slash => write!(f, "/"),
            Token::Percent => write!(f, "%"),
            Token::Equals => write!(f, "="),
            Token::LParen => write!(f, "("),
            Token::RParen => write!(f, ")"),
            Token::LBracket => write!(f, "["),
            Token::RBracket => write!(f, "]"),
            Token::LBrace => write!(f, "{{"),
            Token::RBrace => write!(f, "}}"),
            Token::Pipe => write!(f, "|"),
            Token::Integer(v) => write!(f, "{}", v),
            Token::Float(v) => write!(f, "{}", v),
            Token::StringLit(s) => write!(f, "\"{}\"", s),
            Token::Duration(s) => write!(f, "{}", s),
            Token::Size(s) => write!(f, "{}", s),
            Token::Confidence(v) => write!(f, "~{}", v),
            Token::HashLit(s) => write!(f, "{}", s),
            Token::Identifier(s) => write!(f, "{}", s),
            Token::Newline => write!(f, "NEWLINE"),
            Token::Eof => write!(f, "EOF"),
        }
    }
}

// ---------------------------------------------------------------------------
// Spanned token
// ---------------------------------------------------------------------------

/// A token paired with its source span.
#[derive(Debug, Clone, PartialEq)]
pub struct Spanned {
    pub token: Token,
    pub span: Span,
}

impl Spanned {
    pub fn new(token: Token, span: Span) -> Self {
        Self { token, span }
    }
}

// ---------------------------------------------------------------------------
// Keyword lookup
// ---------------------------------------------------------------------------

fn keyword_or_ident(word: &str) -> Token {
    match word {
        "TYPE" => Token::Type,
        "SCHEMA" => Token::Schema,
        "AGENT" => Token::Agent,
        "OPERATION" => Token::Operation,
        "PIPELINE" => Token::Pipeline,
        "BODY" => Token::Body,
        "INPUT" => Token::Input,
        "OUTPUT" => Token::Output,
        "REQUIRE" => Token::Require,
        "ENSURE" => Token::Ensure,
        "INVARIANT" => Token::Invariant,
        "STORE" => Token::Store,
        "MUTABLE" => Token::Mutable,
        "MATCH" => Token::Match,
        "WHEN" => Token::When,
        "OTHERWISE" => Token::Otherwise,
        "LOOP" => Token::Loop,
        "EMIT" => Token::Emit,
        "ASSERT" => Token::Assert,
        "RETRY" => Token::Retry,
        "ESCALATE" => Token::Escalate,
        "CHECKPOINT" => Token::Checkpoint,
        "RESUME" => Token::Resume,
        "HALT" => Token::Halt,
        "DELEGATE" => Token::Delegate,
        "TO" => Token::To,
        "FORK" => Token::Fork,
        "JOIN" => Token::Join,
        "SUCCESS" => Token::Success,
        "FAILURE" => Token::Failure,
        "TRUE" => Token::True,
        "FALSE" => Token::False,
        "NONE" => Token::None,
        "AND" => Token::And,
        "OR" => Token::Or,
        "NOT" => Token::Not,
        "EQ" => Token::Eq,
        "NEQ" => Token::Neq,
        "GT" => Token::Gt,
        "GTE" => Token::Gte,
        "LT" => Token::Lt,
        "LTE" => Token::Lte,
        _ => Token::Identifier(word.to_string()),
    }
}

// ---------------------------------------------------------------------------
// NEWLINE suppression helpers
// ---------------------------------------------------------------------------

/// Returns `true` when a NEWLINE immediately following `tok` should be
/// suppressed.  These are tokens that *cannot* end a statement.
fn suppresses_newline_after(tok: &Token) -> bool {
    matches!(
        tok,
        Token::Arrow
            | Token::PipeForward
            | Token::FatArrow
            | Token::Dot
            | Token::Colon
            | Token::DoubleColon
            | Token::Comma
            | Token::LParen
            | Token::LBracket
            | Token::LBrace
            | Token::Equals
    )
}

/// Returns `true` when a NEWLINE immediately *before* `tok` should be
/// suppressed.  These are tokens that *continue* an expression.
fn suppresses_newline_before(tok: &Token) -> bool {
    matches!(
        tok,
        Token::Dot
            | Token::Comma
            | Token::Arrow
            | Token::PipeForward
            | Token::DoubleColon
            | Token::RParen
            | Token::RBracket
            | Token::RBrace
    )
}

// ---------------------------------------------------------------------------
// Lexer
// ---------------------------------------------------------------------------

/// The AgentLang lexer.
pub struct Lexer<'src> {
    /// Source text as a byte slice (must be valid UTF-8).
    source: &'src [u8],
    /// Current byte offset into `source`.
    pos: usize,
    /// Current 1-based line number.
    line: usize,
    /// Current 1-based column (byte-based).
    col: usize,
    /// Bracket / brace / paren nesting depth.  When > 0, NEWLINE tokens
    /// are suppressed entirely.
    nesting_depth: usize,
    /// Raw tokens produced during the scan phase (before NEWLINE filtering).
    tokens: Vec<Spanned>,
    /// Accumulated diagnostics (errors) encountered during scanning.
    diagnostics: Vec<Diagnostic>,
}

impl<'src> Lexer<'src> {
    /// Create a new lexer over the given source text.
    pub fn new(source: &'src str) -> Self {
        Self {
            source: source.as_bytes(),
            pos: 0,
            line: 1,
            col: 1,
            nesting_depth: 0,
            tokens: Vec::new(),
            diagnostics: Vec::new(),
        }
    }

    // -- peek / advance helpers -------------------------------------------------

    fn peek(&self) -> Option<u8> {
        self.source.get(self.pos).copied()
    }

    fn peek_ahead(&self, offset: usize) -> Option<u8> {
        self.source.get(self.pos + offset).copied()
    }

    fn advance(&mut self) -> Option<u8> {
        let ch = self.source.get(self.pos).copied()?;
        self.pos += 1;
        if ch == b'\n' {
            self.line += 1;
            self.col = 1;
        } else {
            self.col += 1;
        }
        Some(ch)
    }

    fn at_end(&self) -> bool {
        self.pos >= self.source.len()
    }

    fn current_span(&self, start_offset: usize, start_line: usize, start_col: usize) -> Span {
        Span::new(start_offset, start_line, start_col, self.pos - start_offset)
    }

    fn emit(&mut self, token: Token, span: Span) {
        self.tokens.push(Spanned::new(token, span));
    }

    fn error(&mut self, msg: impl Into<String>, span: Span) {
        self.diagnostics
            .push(Diagnostic::error(ErrorCode::ParseError, msg, span));
    }

    // -- scanning entry ---------------------------------------------------------

    /// Scan through the entire source, producing raw tokens.
    fn scan_all(&mut self) {
        while !self.at_end() {
            self.scan_token();
        }

        // Append EOF
        let eof_span = Span::new(self.pos, self.line, self.col, 0);
        self.emit(Token::Eof, eof_span);
    }

    /// Scan a single token (or skip whitespace / comments).
    fn scan_token(&mut self) {
        // Skip horizontal whitespace (spaces and tabs, NOT newlines).
        self.skip_horizontal_whitespace();

        if self.at_end() {
            return;
        }

        let ch = self.peek().unwrap();

        // -- Line comments -------------------------------------------------------
        if ch == b'/' && self.peek_ahead(1) == Some(b'/') {
            self.skip_line_comment();
            return;
        }

        // -- Newlines ------------------------------------------------------------
        if ch == b'\n' || (ch == b'\r' && self.peek_ahead(1) == Some(b'\n')) {
            self.scan_newline();
            return;
        }
        if ch == b'\r' {
            // bare \r — treat as newline
            self.scan_newline();
            return;
        }

        // -- Confidence literal: ~DIGIT... ----------------------------------------
        if ch == b'~' && self.peek_ahead(1).is_some_and(|c| c.is_ascii_digit()) {
            self.scan_confidence();
            return;
        }

        // -- String literal -------------------------------------------------------
        if ch == b'"' {
            self.scan_string();
            return;
        }

        // -- Number literal (integer, float, duration, size) ----------------------
        if ch.is_ascii_digit() {
            self.scan_number();
            return;
        }

        // -- Identifier / keyword / hash-literal ----------------------------------
        if ch == b'_' || ch.is_ascii_alphabetic() {
            self.scan_identifier_or_keyword();
            return;
        }

        // -- Operators / punctuation -----------------------------------------------
        self.scan_operator();
    }

    // -- horizontal whitespace ---------------------------------------------------

    fn skip_horizontal_whitespace(&mut self) {
        while let Some(ch) = self.peek() {
            if ch == b' ' || ch == b'\t' {
                self.advance();
            } else {
                break;
            }
        }
    }

    // -- line comment ------------------------------------------------------------

    fn skip_line_comment(&mut self) {
        // consume until end of line (don't consume the newline itself)
        while let Some(ch) = self.peek() {
            if ch == b'\n' || ch == b'\r' {
                break;
            }
            self.advance();
        }
    }

    // -- newline handling --------------------------------------------------------

    fn scan_newline(&mut self) {
        let start_offset = self.pos;
        let start_line = self.line;
        let start_col = self.col;

        // Consume all consecutive newlines (collapsing them).
        while let Some(ch) = self.peek() {
            if ch == b'\n' {
                self.advance();
                // After consuming a newline, skip any horizontal whitespace and
                // additional newlines that follow (collapse).
                self.skip_horizontal_whitespace();
            } else if ch == b'\r' {
                self.advance();
                // Consume optional \n after \r
                if self.peek() == Some(b'\n') {
                    self.advance();
                }
                self.skip_horizontal_whitespace();
            } else if ch == b'/' && self.peek_ahead(1) == Some(b'/') {
                // A comment on a blank line — skip it and continue collapsing.
                self.skip_line_comment();
            } else {
                break;
            }
        }

        // Only emit NEWLINE when not inside brackets.
        if self.nesting_depth == 0 {
            let span = self.current_span(start_offset, start_line, start_col);
            self.emit(Token::Newline, span);
        }
    }

    // -- confidence literal (~0.95) ----------------------------------------------

    fn scan_confidence(&mut self) {
        let start_offset = self.pos;
        let start_line = self.line;
        let start_col = self.col;

        // consume '~'
        self.advance();

        // consume digits and optional '.'
        let num_start = self.pos;
        while let Some(ch) = self.peek() {
            if ch.is_ascii_digit() || ch == b'.' {
                self.advance();
            } else {
                break;
            }
        }

        let num_str = std::str::from_utf8(&self.source[num_start..self.pos]).unwrap();
        let span = self.current_span(start_offset, start_line, start_col);

        match num_str.parse::<f64>() {
            Ok(val) => self.emit(Token::Confidence(val), span),
            Err(_) => {
                self.error(format!("invalid confidence literal: ~{}", num_str), span);
            }
        }
    }

    // -- string literal ----------------------------------------------------------

    fn scan_string(&mut self) {
        let start_offset = self.pos;
        let start_line = self.line;
        let start_col = self.col;

        // consume opening '"'
        self.advance();

        let mut value = String::new();
        let mut terminated = false;

        while let Some(ch) = self.peek() {
            if ch == b'"' {
                self.advance(); // consume closing '"'
                terminated = true;
                break;
            } else if ch == b'\\' {
                self.advance(); // consume '\'
                match self.peek() {
                    Some(b'n') => {
                        self.advance();
                        value.push('\n');
                    }
                    Some(b't') => {
                        self.advance();
                        value.push('\t');
                    }
                    Some(b'r') => {
                        self.advance();
                        value.push('\r');
                    }
                    Some(b'\\') => {
                        self.advance();
                        value.push('\\');
                    }
                    Some(b'"') => {
                        self.advance();
                        value.push('"');
                    }
                    Some(other) => {
                        self.advance();
                        value.push('\\');
                        value.push(other as char);
                    }
                    None => break,
                }
            } else if ch == b'\n' || ch == b'\r' {
                // Unterminated string at end of line
                break;
            } else {
                self.advance();
                value.push(ch as char);
            }
        }

        let span = self.current_span(start_offset, start_line, start_col);

        if !terminated {
            self.error("unterminated string literal", span);
        }

        self.emit(Token::StringLit(value), span);
    }

    // -- number / duration / size ------------------------------------------------

    fn scan_number(&mut self) {
        let start_offset = self.pos;
        let start_line = self.line;
        let start_col = self.col;

        let first = self.peek().unwrap();

        // Check for hex (0x) or binary (0b) prefix
        if first == b'0' {
            if let Some(next) = self.peek_ahead(1) {
                if next == b'x' || next == b'X' {
                    self.scan_hex_integer(start_offset, start_line, start_col);
                    return;
                }
                if next == b'b' || next == b'B' {
                    self.scan_bin_integer(start_offset, start_line, start_col);
                    return;
                }
            }
        }

        // Consume integer part
        while let Some(ch) = self.peek() {
            if ch.is_ascii_digit() || ch == b'_' {
                self.advance();
            } else {
                break;
            }
        }

        let mut is_float = false;

        // Check for fractional part: '.' followed by digit (not '..' range operator)
        if self.peek() == Some(b'.') && self.peek_ahead(1).is_some_and(|c| c.is_ascii_digit()) {
            is_float = true;
            self.advance(); // consume '.'
            while let Some(ch) = self.peek() {
                if ch.is_ascii_digit() || ch == b'_' {
                    self.advance();
                } else {
                    break;
                }
            }
        }

        // Check for exponent
        if let Some(ch) = self.peek() {
            if ch == b'e' || ch == b'E' {
                is_float = true;
                self.advance();
                if let Some(sign) = self.peek() {
                    if sign == b'+' || sign == b'-' {
                        self.advance();
                    }
                }
                while let Some(ch) = self.peek() {
                    if ch.is_ascii_digit() {
                        self.advance();
                    } else {
                        break;
                    }
                }
            }
        }

        // Check for duration suffix: ms, s, m, h
        // Check for size suffix: KB, MB, GB, TB
        if !is_float {
            if let Some((token, span)) =
                self.try_duration_or_size_suffix(start_offset, start_line, start_col)
            {
                self.emit(token, span);
                return;
            }
        }

        let text = std::str::from_utf8(&self.source[start_offset..self.pos]).unwrap();
        // Strip underscores for parsing
        let clean: String = text.chars().filter(|&c| c != '_').collect();
        let span = self.current_span(start_offset, start_line, start_col);

        if is_float {
            match clean.parse::<f64>() {
                Ok(val) => self.emit(Token::Float(val), span),
                Err(_) => self.error(format!("invalid float literal: {}", text), span),
            }
        } else {
            match clean.parse::<i64>() {
                Ok(val) => self.emit(Token::Integer(val), span),
                Err(_) => self.error(format!("invalid integer literal: {}", text), span),
            }
        }
    }

    /// Try to consume a duration suffix (ms, s, m, h) or size suffix (KB, MB, GB, TB).
    /// Returns `Some((token, span))` if a suffix was found, `None` otherwise.
    fn try_duration_or_size_suffix(
        &mut self,
        start_offset: usize,
        start_line: usize,
        start_col: usize,
    ) -> Option<(Token, Span)> {
        let saved_pos = self.pos;
        let saved_line = self.line;
        let saved_col = self.col;

        if let Some(ch) = self.peek() {
            match ch {
                b'm' => {
                    self.advance();
                    if self.peek() == Some(b's') {
                        // "ms" — milliseconds
                        self.advance();
                        let text =
                            std::str::from_utf8(&self.source[start_offset..self.pos]).unwrap();
                        let span = self.current_span(start_offset, start_line, start_col);
                        return Some((Token::Duration(text.to_string()), span));
                    }
                    // "m" — minutes (only if next char is NOT alphanumeric/underscore)
                    if self
                        .peek()
                        .is_none_or(|c| !c.is_ascii_alphanumeric() && c != b'_')
                    {
                        let text =
                            std::str::from_utf8(&self.source[start_offset..self.pos]).unwrap();
                        let span = self.current_span(start_offset, start_line, start_col);
                        return Some((Token::Duration(text.to_string()), span));
                    }
                    // Revert
                    self.pos = saved_pos;
                    self.line = saved_line;
                    self.col = saved_col;
                }
                b's' => {
                    self.advance();
                    if self
                        .peek()
                        .is_none_or(|c| !c.is_ascii_alphanumeric() && c != b'_')
                    {
                        let text =
                            std::str::from_utf8(&self.source[start_offset..self.pos]).unwrap();
                        let span = self.current_span(start_offset, start_line, start_col);
                        return Some((Token::Duration(text.to_string()), span));
                    }
                    self.pos = saved_pos;
                    self.line = saved_line;
                    self.col = saved_col;
                }
                b'h' => {
                    self.advance();
                    if self
                        .peek()
                        .is_none_or(|c| !c.is_ascii_alphanumeric() && c != b'_')
                    {
                        let text =
                            std::str::from_utf8(&self.source[start_offset..self.pos]).unwrap();
                        let span = self.current_span(start_offset, start_line, start_col);
                        return Some((Token::Duration(text.to_string()), span));
                    }
                    self.pos = saved_pos;
                    self.line = saved_line;
                    self.col = saved_col;
                }
                b'K' => {
                    self.advance();
                    if self.peek() == Some(b'B') {
                        self.advance();
                        let text =
                            std::str::from_utf8(&self.source[start_offset..self.pos]).unwrap();
                        let span = self.current_span(start_offset, start_line, start_col);
                        return Some((Token::Size(text.to_string()), span));
                    }
                    self.pos = saved_pos;
                    self.line = saved_line;
                    self.col = saved_col;
                }
                b'M' => {
                    self.advance();
                    if self.peek() == Some(b'B') {
                        self.advance();
                        let text =
                            std::str::from_utf8(&self.source[start_offset..self.pos]).unwrap();
                        let span = self.current_span(start_offset, start_line, start_col);
                        return Some((Token::Size(text.to_string()), span));
                    }
                    self.pos = saved_pos;
                    self.line = saved_line;
                    self.col = saved_col;
                }
                b'G' => {
                    self.advance();
                    if self.peek() == Some(b'B') {
                        self.advance();
                        let text =
                            std::str::from_utf8(&self.source[start_offset..self.pos]).unwrap();
                        let span = self.current_span(start_offset, start_line, start_col);
                        return Some((Token::Size(text.to_string()), span));
                    }
                    self.pos = saved_pos;
                    self.line = saved_line;
                    self.col = saved_col;
                }
                b'T' => {
                    self.advance();
                    if self.peek() == Some(b'B') {
                        self.advance();
                        let text =
                            std::str::from_utf8(&self.source[start_offset..self.pos]).unwrap();
                        let span = self.current_span(start_offset, start_line, start_col);
                        return Some((Token::Size(text.to_string()), span));
                    }
                    self.pos = saved_pos;
                    self.line = saved_line;
                    self.col = saved_col;
                }
                _ => {}
            }
        }

        None
    }

    fn scan_hex_integer(&mut self, start_offset: usize, start_line: usize, start_col: usize) {
        // consume '0'
        self.advance();
        // consume 'x' or 'X'
        self.advance();

        while let Some(ch) = self.peek() {
            if ch.is_ascii_hexdigit() || ch == b'_' {
                self.advance();
            } else {
                break;
            }
        }

        let text = std::str::from_utf8(&self.source[start_offset..self.pos]).unwrap();
        let hex_digits: String = text[2..].chars().filter(|&c| c != '_').collect();
        let span = self.current_span(start_offset, start_line, start_col);

        match i64::from_str_radix(&hex_digits, 16) {
            Ok(val) => self.emit(Token::Integer(val), span),
            Err(_) => self.error(format!("invalid hex literal: {}", text), span),
        }
    }

    fn scan_bin_integer(&mut self, start_offset: usize, start_line: usize, start_col: usize) {
        // consume '0'
        self.advance();
        // consume 'b' or 'B'
        self.advance();

        while let Some(ch) = self.peek() {
            if ch == b'0' || ch == b'1' || ch == b'_' {
                self.advance();
            } else {
                break;
            }
        }

        let text = std::str::from_utf8(&self.source[start_offset..self.pos]).unwrap();
        let bin_digits: String = text[2..].chars().filter(|&c| c != '_').collect();
        let span = self.current_span(start_offset, start_line, start_col);

        match i64::from_str_radix(&bin_digits, 2) {
            Ok(val) => self.emit(Token::Integer(val), span),
            Err(_) => self.error(format!("invalid binary literal: {}", text), span),
        }
    }

    // -- identifier / keyword / hash literal -------------------------------------

    fn scan_identifier_or_keyword(&mut self) {
        let start_offset = self.pos;
        let start_line = self.line;
        let start_col = self.col;

        while let Some(ch) = self.peek() {
            if ch.is_ascii_alphanumeric() || ch == b'_' {
                self.advance();
            } else {
                break;
            }
        }

        let word = std::str::from_utf8(&self.source[start_offset..self.pos]).unwrap();

        // Check for hash literal: e.g. SHA256:abcdef...
        // Pattern: uppercase letters mixed with digits, then ':' then hex/alphanumeric
        if self.peek() == Some(b':') && self.is_hash_prefix(word) {
            // Peek ahead past ':' to see if there are hex/alphanumeric chars
            if self
                .peek_ahead(1)
                .is_some_and(|c| c.is_ascii_alphanumeric())
            {
                self.advance(); // consume ':'
                                // Consume hex/alphanumeric chars for the hash value
                while let Some(ch) = self.peek() {
                    if ch.is_ascii_alphanumeric() {
                        self.advance();
                    } else {
                        break;
                    }
                }
                let full = std::str::from_utf8(&self.source[start_offset..self.pos]).unwrap();
                let span = self.current_span(start_offset, start_line, start_col);
                self.emit(Token::HashLit(full.to_string()), span);
                return;
            }
        }

        let span = self.current_span(start_offset, start_line, start_col);
        let token = keyword_or_ident(word);
        self.emit(token, span);
    }

    /// Returns true if `word` looks like a hash algorithm prefix
    /// (e.g. "SHA256", "SHA512", "MD5", "BLAKE2b").
    fn is_hash_prefix(&self, word: &str) -> bool {
        if word.len() < 2 {
            return false;
        }
        let mut has_letter = false;
        let mut has_digit = false;
        for ch in word.chars() {
            if ch.is_ascii_uppercase() {
                has_letter = true;
            } else if ch.is_ascii_digit() {
                has_digit = true;
            } else {
                return false;
            }
        }
        has_letter && has_digit
    }

    // -- operators / punctuation -------------------------------------------------

    fn scan_operator(&mut self) {
        let start_offset = self.pos;
        let start_line = self.line;
        let start_col = self.col;

        let ch = self.advance().unwrap();

        match ch {
            b'-' => {
                if self.peek() == Some(b'>') {
                    self.advance();
                    let span = self.current_span(start_offset, start_line, start_col);
                    self.emit(Token::Arrow, span);
                } else {
                    let span = self.current_span(start_offset, start_line, start_col);
                    self.emit(Token::Minus, span);
                }
            }
            b'|' => {
                if self.peek() == Some(b'>') {
                    self.advance();
                    let span = self.current_span(start_offset, start_line, start_col);
                    self.emit(Token::PipeForward, span);
                } else {
                    let span = self.current_span(start_offset, start_line, start_col);
                    self.emit(Token::Pipe, span);
                }
            }
            b'=' => {
                if self.peek() == Some(b'>') {
                    self.advance();
                    let span = self.current_span(start_offset, start_line, start_col);
                    self.emit(Token::FatArrow, span);
                } else {
                    let span = self.current_span(start_offset, start_line, start_col);
                    self.emit(Token::Equals, span);
                }
            }
            b':' => {
                if self.peek() == Some(b':') {
                    self.advance();
                    let span = self.current_span(start_offset, start_line, start_col);
                    self.emit(Token::DoubleColon, span);
                } else {
                    let span = self.current_span(start_offset, start_line, start_col);
                    self.emit(Token::Colon, span);
                }
            }
            b'.' => {
                if self.peek() == Some(b'.') {
                    self.advance();
                    let span = self.current_span(start_offset, start_line, start_col);
                    self.emit(Token::DotDot, span);
                } else {
                    let span = self.current_span(start_offset, start_line, start_col);
                    self.emit(Token::Dot, span);
                }
            }
            b'(' => {
                self.nesting_depth += 1;
                let span = self.current_span(start_offset, start_line, start_col);
                self.emit(Token::LParen, span);
            }
            b')' => {
                if self.nesting_depth > 0 {
                    self.nesting_depth -= 1;
                }
                let span = self.current_span(start_offset, start_line, start_col);
                self.emit(Token::RParen, span);
            }
            b'[' => {
                self.nesting_depth += 1;
                let span = self.current_span(start_offset, start_line, start_col);
                self.emit(Token::LBracket, span);
            }
            b']' => {
                if self.nesting_depth > 0 {
                    self.nesting_depth -= 1;
                }
                let span = self.current_span(start_offset, start_line, start_col);
                self.emit(Token::RBracket, span);
            }
            b'{' => {
                self.nesting_depth += 1;
                let span = self.current_span(start_offset, start_line, start_col);
                self.emit(Token::LBrace, span);
            }
            b'}' => {
                if self.nesting_depth > 0 {
                    self.nesting_depth -= 1;
                }
                let span = self.current_span(start_offset, start_line, start_col);
                self.emit(Token::RBrace, span);
            }
            b'?' => {
                let span = self.current_span(start_offset, start_line, start_col);
                self.emit(Token::Question, span);
            }
            b'@' => {
                let span = self.current_span(start_offset, start_line, start_col);
                self.emit(Token::At, span);
            }
            b'#' => {
                let span = self.current_span(start_offset, start_line, start_col);
                self.emit(Token::Hash, span);
            }
            b',' => {
                let span = self.current_span(start_offset, start_line, start_col);
                self.emit(Token::Comma, span);
            }
            b';' => {
                let span = self.current_span(start_offset, start_line, start_col);
                self.emit(Token::Semicolon, span);
            }
            b'+' => {
                let span = self.current_span(start_offset, start_line, start_col);
                self.emit(Token::Plus, span);
            }
            b'*' => {
                let span = self.current_span(start_offset, start_line, start_col);
                self.emit(Token::Star, span);
            }
            b'/' => {
                let span = self.current_span(start_offset, start_line, start_col);
                self.emit(Token::Slash, span);
            }
            b'%' => {
                let span = self.current_span(start_offset, start_line, start_col);
                self.emit(Token::Percent, span);
            }
            b'~' => {
                // Lone tilde (confidence without digits was already handled)
                let span = self.current_span(start_offset, start_line, start_col);
                self.error("unexpected character: `~`", span);
            }
            _ => {
                let span = self.current_span(start_offset, start_line, start_col);
                self.error(format!("unexpected character: `{}`", ch as char), span);
            }
        }
    }

    // -- NEWLINE filtering pass --------------------------------------------------

    /// Post-process the raw token stream to apply NEWLINE suppression rules:
    ///
    /// 1. NEWLINE suppression inside brackets (already handled during scan).
    /// 2. Collapse consecutive NEWLINEs (already handled during scan).
    /// 3. Suppress NEWLINE after tokens that cannot end a statement.
    /// 4. Suppress NEWLINE before tokens that continue an expression.
    fn filter_newlines(raw: Vec<Spanned>) -> Vec<Spanned> {
        if raw.is_empty() {
            return raw;
        }

        let mut filtered: Vec<Spanned> = Vec::with_capacity(raw.len());

        for spanned in raw {
            if spanned.token == Token::Newline {
                // Rule 3: suppress NEWLINE after tokens that cannot end a statement.
                if let Some(prev) = filtered.last() {
                    if suppresses_newline_after(&prev.token) {
                        continue;
                    }
                }
                // Suppress a leading NEWLINE (no previous non-newline token).
                if filtered.is_empty() {
                    continue;
                }
                // Rule 2: collapse consecutive NEWLINEs (guard against edge cases).
                if let Some(prev) = filtered.last() {
                    if prev.token == Token::Newline {
                        continue;
                    }
                }
                // Tentatively add — may be removed by rule 4 when we see the next token.
                filtered.push(spanned);
            } else {
                // Rule 4: suppress NEWLINE *before* tokens that continue an expression.
                if suppresses_newline_before(&spanned.token) {
                    while filtered.last().is_some_and(|t| t.token == Token::Newline) {
                        filtered.pop();
                    }
                }
                filtered.push(spanned);
            }
        }

        filtered
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Tokenise an AgentLang source string.
///
/// Returns `Ok(tokens)` on success (which always ends with `Token::Eof`),
/// or `Err(diagnostics)` if lexical errors were encountered.
///
/// Even when errors are found the lexer attempts to produce as many tokens
/// as possible (error recovery) so diagnostics carry useful context.
pub fn tokenize(source: &str) -> Result<Vec<Spanned>, Vec<Diagnostic>> {
    let mut lexer = Lexer::new(source);
    lexer.scan_all();

    let tokens = Lexer::filter_newlines(lexer.tokens);

    if lexer.diagnostics.is_empty() {
        Ok(tokens)
    } else {
        Err(lexer.diagnostics)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: tokenize and unwrap, then return just the Token values
    /// (excluding Eof).
    fn tok(src: &str) -> Vec<Token> {
        let spanned = tokenize(src).expect("tokenization should succeed");
        spanned
            .into_iter()
            .map(|s| s.token)
            .filter(|t| *t != Token::Eof)
            .collect()
    }

    /// Helper: tokenize and unwrap, returning full Spanned tokens (including Eof).
    fn tok_full(src: &str) -> Vec<Spanned> {
        tokenize(src).expect("tokenization should succeed")
    }

    // ── Keyword tokenization ──────────────────────────────────────────

    #[test]
    fn keywords_all() {
        let input = "TYPE SCHEMA AGENT OPERATION PIPELINE BODY INPUT OUTPUT \
                      REQUIRE ENSURE INVARIANT STORE MUTABLE MATCH WHEN \
                      OTHERWISE LOOP EMIT ASSERT RETRY ESCALATE CHECKPOINT \
                      RESUME HALT DELEGATE TO FORK JOIN SUCCESS FAILURE \
                      TRUE FALSE NONE AND OR NOT EQ NEQ GT GTE LT LTE";
        let tokens = tok(input);
        let expected = vec![
            Token::Type,
            Token::Schema,
            Token::Agent,
            Token::Operation,
            Token::Pipeline,
            Token::Body,
            Token::Input,
            Token::Output,
            Token::Require,
            Token::Ensure,
            Token::Invariant,
            Token::Store,
            Token::Mutable,
            Token::Match,
            Token::When,
            Token::Otherwise,
            Token::Loop,
            Token::Emit,
            Token::Assert,
            Token::Retry,
            Token::Escalate,
            Token::Checkpoint,
            Token::Resume,
            Token::Halt,
            Token::Delegate,
            Token::To,
            Token::Fork,
            Token::Join,
            Token::Success,
            Token::Failure,
            Token::True,
            Token::False,
            Token::None,
            Token::And,
            Token::Or,
            Token::Not,
            Token::Eq,
            Token::Neq,
            Token::Gt,
            Token::Gte,
            Token::Lt,
            Token::Lte,
        ];
        assert_eq!(tokens, expected);
    }

    #[test]
    fn keyword_case_sensitive() {
        // Lowercase should be identifiers, not keywords.
        let tokens = tok("type schema agent");
        assert_eq!(
            tokens,
            vec![
                Token::Identifier("type".into()),
                Token::Identifier("schema".into()),
                Token::Identifier("agent".into()),
            ]
        );
    }

    // ── Operator tokenization ─────────────────────────────────────────

    #[test]
    fn operators_all() {
        let input = "-> |> => : :: ? @ # .. . , ; + - * / % = ( ) [ ] { } |";
        let tokens = tok(input);
        let expected = vec![
            Token::Arrow,
            Token::PipeForward,
            Token::FatArrow,
            Token::Colon,
            Token::DoubleColon,
            Token::Question,
            Token::At,
            Token::Hash,
            Token::DotDot,
            Token::Dot,
            Token::Comma,
            Token::Semicolon,
            Token::Plus,
            Token::Minus,
            Token::Star,
            Token::Slash,
            Token::Percent,
            Token::Equals,
            Token::LParen,
            Token::RParen,
            Token::LBracket,
            Token::RBracket,
            Token::LBrace,
            Token::RBrace,
            Token::Pipe,
        ];
        assert_eq!(tokens, expected);
    }

    #[test]
    fn arrow_vs_minus() {
        let tokens = tok("a -> b - c");
        assert_eq!(
            tokens,
            vec![
                Token::Identifier("a".into()),
                Token::Arrow,
                Token::Identifier("b".into()),
                Token::Minus,
                Token::Identifier("c".into()),
            ]
        );
    }

    #[test]
    fn pipe_forward_vs_pipe() {
        let tokens = tok("a |> b | c");
        assert_eq!(
            tokens,
            vec![
                Token::Identifier("a".into()),
                Token::PipeForward,
                Token::Identifier("b".into()),
                Token::Pipe,
                Token::Identifier("c".into()),
            ]
        );
    }

    #[test]
    fn fat_arrow_vs_equals() {
        let tokens = tok("a => b = c");
        assert_eq!(
            tokens,
            vec![
                Token::Identifier("a".into()),
                Token::FatArrow,
                Token::Identifier("b".into()),
                Token::Equals,
                Token::Identifier("c".into()),
            ]
        );
    }

    #[test]
    fn double_colon_vs_colon() {
        let tokens = tok("a :: b : c");
        assert_eq!(
            tokens,
            vec![
                Token::Identifier("a".into()),
                Token::DoubleColon,
                Token::Identifier("b".into()),
                Token::Colon,
                Token::Identifier("c".into()),
            ]
        );
    }

    #[test]
    fn dot_dot_vs_dot() {
        let tokens = tok("a .. b . c");
        assert_eq!(
            tokens,
            vec![
                Token::Identifier("a".into()),
                Token::DotDot,
                Token::Identifier("b".into()),
                Token::Dot,
                Token::Identifier("c".into()),
            ]
        );
    }

    // ── Integer literals ──────────────────────────────────────────────

    #[test]
    fn integer_decimal() {
        assert_eq!(tok("42"), vec![Token::Integer(42)]);
        assert_eq!(tok("0"), vec![Token::Integer(0)]);
        assert_eq!(tok("1_000_000"), vec![Token::Integer(1_000_000)]);
    }

    #[test]
    fn integer_hex() {
        assert_eq!(tok("0xFF"), vec![Token::Integer(255)]);
        assert_eq!(tok("0x1A"), vec![Token::Integer(26)]);
    }

    #[test]
    fn integer_binary() {
        assert_eq!(tok("0b1010"), vec![Token::Integer(10)]);
        assert_eq!(tok("0b0"), vec![Token::Integer(0)]);
    }

    // ── Float literals ────────────────────────────────────────────────

    #[test]
    fn float_simple() {
        #[allow(clippy::approx_constant)]
        let expected = vec![Token::Float(3.14)];
        assert_eq!(tok("3.14"), expected);
        assert_eq!(tok("0.5"), vec![Token::Float(0.5)]);
    }

    #[test]
    fn float_exponent() {
        assert_eq!(tok("1.0e-10"), vec![Token::Float(1.0e-10)]);
        assert_eq!(tok("2e3"), vec![Token::Float(2e3)]);
    }

    // ── String literals ───────────────────────────────────────────────

    #[test]
    fn string_simple() {
        assert_eq!(tok(r#""hello""#), vec![Token::StringLit("hello".into())]);
    }

    #[test]
    fn string_with_escapes() {
        assert_eq!(
            tok(r#""a\nb\tc\\d\"e""#),
            vec![Token::StringLit("a\nb\tc\\d\"e".into())]
        );
    }

    #[test]
    fn string_empty() {
        assert_eq!(tok(r#""""#), vec![Token::StringLit(String::new())]);
    }

    // ── Duration literals ─────────────────────────────────────────────

    #[test]
    fn duration_seconds() {
        assert_eq!(tok("5s"), vec![Token::Duration("5s".into())]);
    }

    #[test]
    fn duration_milliseconds() {
        assert_eq!(tok("100ms"), vec![Token::Duration("100ms".into())]);
    }

    #[test]
    fn duration_minutes() {
        assert_eq!(tok("2m"), vec![Token::Duration("2m".into())]);
    }

    #[test]
    fn duration_hours() {
        assert_eq!(tok("1h"), vec![Token::Duration("1h".into())]);
    }

    // ── Size literals ─────────────────────────────────────────────────

    #[test]
    fn size_kilobytes() {
        assert_eq!(tok("256KB"), vec![Token::Size("256KB".into())]);
    }

    #[test]
    fn size_megabytes() {
        assert_eq!(tok("1MB"), vec![Token::Size("1MB".into())]);
    }

    #[test]
    fn size_gigabytes() {
        assert_eq!(tok("4GB"), vec![Token::Size("4GB".into())]);
    }

    #[test]
    fn size_terabytes() {
        assert_eq!(tok("2TB"), vec![Token::Size("2TB".into())]);
    }

    // ── Confidence literals ───────────────────────────────────────────

    #[test]
    fn confidence_literal() {
        assert_eq!(tok("~0.95"), vec![Token::Confidence(0.95)]);
        assert_eq!(tok("~0.5"), vec![Token::Confidence(0.5)]);
        assert_eq!(tok("~1.0"), vec![Token::Confidence(1.0)]);
    }

    // ── Hash literals ─────────────────────────────────────────────────

    #[test]
    fn hash_literal_sha256() {
        assert_eq!(
            tok("SHA256:abcdef1234"),
            vec![Token::HashLit("SHA256:abcdef1234".into())]
        );
    }

    #[test]
    fn hash_literal_md5() {
        assert_eq!(
            tok("MD5:deadbeef"),
            vec![Token::HashLit("MD5:deadbeef".into())]
        );
    }

    // ── Identifiers ───────────────────────────────────────────────────

    #[test]
    fn identifiers() {
        assert_eq!(
            tok("foo bar_baz _private x123"),
            vec![
                Token::Identifier("foo".into()),
                Token::Identifier("bar_baz".into()),
                Token::Identifier("_private".into()),
                Token::Identifier("x123".into()),
            ]
        );
    }

    // ── Comments ──────────────────────────────────────────────────────

    #[test]
    fn line_comment() {
        let tokens = tok("foo // this is a comment\nbar");
        assert_eq!(
            tokens,
            vec![
                Token::Identifier("foo".into()),
                Token::Newline,
                Token::Identifier("bar".into()),
            ]
        );
    }

    #[test]
    fn comment_at_end_of_file() {
        let tokens = tok("foo // trailing comment");
        assert_eq!(tokens, vec![Token::Identifier("foo".into())]);
    }

    #[test]
    fn comment_only_lines() {
        let tokens = tok("// first\n// second\nfoo");
        assert_eq!(tokens, vec![Token::Identifier("foo".into())]);
    }

    // ── NEWLINE suppression inside brackets ───────────────────────────

    #[test]
    fn newline_suppressed_inside_parens() {
        let tokens = tok("(\na\n)");
        assert_eq!(
            tokens,
            vec![Token::LParen, Token::Identifier("a".into()), Token::RParen,]
        );
    }

    #[test]
    fn newline_suppressed_inside_brackets() {
        let tokens = tok("[\na\n,\nb\n]");
        assert_eq!(
            tokens,
            vec![
                Token::LBracket,
                Token::Identifier("a".into()),
                Token::Comma,
                Token::Identifier("b".into()),
                Token::RBracket,
            ]
        );
    }

    #[test]
    fn newline_suppressed_inside_braces() {
        let tokens = tok("{\na\n}");
        assert_eq!(
            tokens,
            vec![Token::LBrace, Token::Identifier("a".into()), Token::RBrace,]
        );
    }

    #[test]
    fn newline_suppressed_nested_brackets() {
        let tokens = tok("(\n[\na\n]\n)");
        assert_eq!(
            tokens,
            vec![
                Token::LParen,
                Token::LBracket,
                Token::Identifier("a".into()),
                Token::RBracket,
                Token::RParen,
            ]
        );
    }

    // ── NEWLINE collapse ──────────────────────────────────────────────

    #[test]
    fn newline_collapse_multiple() {
        let tokens = tok("a\n\n\nb");
        assert_eq!(
            tokens,
            vec![
                Token::Identifier("a".into()),
                Token::Newline,
                Token::Identifier("b".into()),
            ]
        );
    }

    #[test]
    fn newline_collapse_with_spaces() {
        let tokens = tok("a\n  \n  \nb");
        assert_eq!(
            tokens,
            vec![
                Token::Identifier("a".into()),
                Token::Newline,
                Token::Identifier("b".into()),
            ]
        );
    }

    // ── NEWLINE suppression after statement-continuing tokens ─────────

    #[test]
    fn newline_suppressed_after_arrow() {
        let tokens = tok("a ->\nb");
        assert_eq!(
            tokens,
            vec![
                Token::Identifier("a".into()),
                Token::Arrow,
                Token::Identifier("b".into()),
            ]
        );
    }

    #[test]
    fn newline_suppressed_after_pipe_forward() {
        let tokens = tok("a |>\nb");
        assert_eq!(
            tokens,
            vec![
                Token::Identifier("a".into()),
                Token::PipeForward,
                Token::Identifier("b".into()),
            ]
        );
    }

    #[test]
    fn newline_suppressed_after_fat_arrow() {
        let tokens = tok("a =>\nb");
        assert_eq!(
            tokens,
            vec![
                Token::Identifier("a".into()),
                Token::FatArrow,
                Token::Identifier("b".into()),
            ]
        );
    }

    #[test]
    fn newline_suppressed_after_dot() {
        let tokens = tok("a.\nb");
        assert_eq!(
            tokens,
            vec![
                Token::Identifier("a".into()),
                Token::Dot,
                Token::Identifier("b".into()),
            ]
        );
    }

    #[test]
    fn newline_suppressed_after_colon() {
        let tokens = tok("a:\nb");
        assert_eq!(
            tokens,
            vec![
                Token::Identifier("a".into()),
                Token::Colon,
                Token::Identifier("b".into()),
            ]
        );
    }

    #[test]
    fn newline_suppressed_after_double_colon() {
        let tokens = tok("a::\nb");
        assert_eq!(
            tokens,
            vec![
                Token::Identifier("a".into()),
                Token::DoubleColon,
                Token::Identifier("b".into()),
            ]
        );
    }

    #[test]
    fn newline_suppressed_after_comma() {
        let tokens = tok("a,\nb");
        assert_eq!(
            tokens,
            vec![
                Token::Identifier("a".into()),
                Token::Comma,
                Token::Identifier("b".into()),
            ]
        );
    }

    #[test]
    fn newline_suppressed_after_equals() {
        let tokens = tok("a =\nb");
        assert_eq!(
            tokens,
            vec![
                Token::Identifier("a".into()),
                Token::Equals,
                Token::Identifier("b".into()),
            ]
        );
    }

    // ── NEWLINE suppression before expression-continuing tokens ───────

    #[test]
    fn newline_suppressed_before_dot() {
        let tokens = tok("a\n.b");
        assert_eq!(
            tokens,
            vec![
                Token::Identifier("a".into()),
                Token::Dot,
                Token::Identifier("b".into()),
            ]
        );
    }

    #[test]
    fn newline_suppressed_before_comma() {
        let tokens = tok("a\n,b");
        assert_eq!(
            tokens,
            vec![
                Token::Identifier("a".into()),
                Token::Comma,
                Token::Identifier("b".into()),
            ]
        );
    }

    #[test]
    fn newline_suppressed_before_arrow() {
        let tokens = tok("a\n-> b");
        assert_eq!(
            tokens,
            vec![
                Token::Identifier("a".into()),
                Token::Arrow,
                Token::Identifier("b".into()),
            ]
        );
    }

    #[test]
    fn newline_suppressed_before_pipe_forward() {
        let tokens = tok("a\n|> b");
        assert_eq!(
            tokens,
            vec![
                Token::Identifier("a".into()),
                Token::PipeForward,
                Token::Identifier("b".into()),
            ]
        );
    }

    #[test]
    fn newline_suppressed_before_double_colon() {
        let tokens = tok("a\n:: b");
        assert_eq!(
            tokens,
            vec![
                Token::Identifier("a".into()),
                Token::DoubleColon,
                Token::Identifier("b".into()),
            ]
        );
    }

    #[test]
    fn newline_suppressed_before_rparen() {
        // Inside parens, newlines are already suppressed by nesting_depth.
        let tokens = tok("(\na\n)");
        assert_eq!(
            tokens,
            vec![Token::LParen, Token::Identifier("a".into()), Token::RParen,]
        );
    }

    #[test]
    fn newline_suppressed_before_rbracket() {
        let tokens = tok("a\n]");
        assert_eq!(tokens, vec![Token::Identifier("a".into()), Token::RBracket]);
    }

    #[test]
    fn newline_suppressed_before_rbrace() {
        let tokens = tok("a\n}");
        assert_eq!(tokens, vec![Token::Identifier("a".into()), Token::RBrace]);
    }

    // ── NEWLINE emitted between normal tokens ─────────────────────────

    #[test]
    fn newline_emitted_between_identifiers() {
        let tokens = tok("a\nb");
        assert_eq!(
            tokens,
            vec![
                Token::Identifier("a".into()),
                Token::Newline,
                Token::Identifier("b".into()),
            ]
        );
    }

    #[test]
    fn newline_emitted_after_integer() {
        let tokens = tok("42\nfoo");
        assert_eq!(
            tokens,
            vec![
                Token::Integer(42),
                Token::Newline,
                Token::Identifier("foo".into()),
            ]
        );
    }

    // ── Span tracking ─────────────────────────────────────────────────

    #[test]
    fn span_line_and_column() {
        let spanned = tok_full("foo\nbar");
        // "foo" at line 1, col 1, offset 0, length 3
        assert_eq!(spanned[0].span.line, 1);
        assert_eq!(spanned[0].span.column, 1);
        assert_eq!(spanned[0].span.offset, 0);
        assert_eq!(spanned[0].span.length, 3);

        // "bar" at line 2, col 1, offset 4, length 3
        let bar = spanned
            .iter()
            .find(|s| s.token == Token::Identifier("bar".into()))
            .unwrap();
        assert_eq!(bar.span.line, 2);
        assert_eq!(bar.span.column, 1);
        assert_eq!(bar.span.offset, 4);
        assert_eq!(bar.span.length, 3);
    }

    #[test]
    fn span_multi_char_operator() {
        let spanned = tok_full("=>");
        assert_eq!(spanned[0].span.offset, 0);
        assert_eq!(spanned[0].span.length, 2);
    }

    // ── Mixed / integration tests ─────────────────────────────────────

    #[test]
    fn type_declaration() {
        let tokens = tok("TYPE UserId = Int64");
        assert_eq!(
            tokens,
            vec![
                Token::Type,
                Token::Identifier("UserId".into()),
                Token::Equals,
                Token::Identifier("Int64".into()),
            ]
        );
    }

    #[test]
    fn schema_declaration() {
        let tokens = tok(r#"SCHEMA User => { name: Str, age: Int64 }"#);
        assert_eq!(
            tokens,
            vec![
                Token::Schema,
                Token::Identifier("User".into()),
                Token::FatArrow,
                Token::LBrace,
                Token::Identifier("name".into()),
                Token::Colon,
                Token::Identifier("Str".into()),
                Token::Comma,
                Token::Identifier("age".into()),
                Token::Colon,
                Token::Identifier("Int64".into()),
                Token::RBrace,
            ]
        );
    }

    #[test]
    fn pipeline_chain() {
        let tokens = tok("a -> b |> c");
        assert_eq!(
            tokens,
            vec![
                Token::Identifier("a".into()),
                Token::Arrow,
                Token::Identifier("b".into()),
                Token::PipeForward,
                Token::Identifier("c".into()),
            ]
        );
    }

    #[test]
    fn store_with_type_annotation() {
        let tokens = tok("STORE x: Int64 = 42;");
        assert_eq!(
            tokens,
            vec![
                Token::Store,
                Token::Identifier("x".into()),
                Token::Colon,
                Token::Identifier("Int64".into()),
                Token::Equals,
                Token::Integer(42),
                Token::Semicolon,
            ]
        );
    }

    #[test]
    fn mutable_with_reason() {
        let tokens = tok(r#"MUTABLE count @reason("loop counter") = 0"#);
        assert_eq!(
            tokens,
            vec![
                Token::Mutable,
                Token::Identifier("count".into()),
                Token::At,
                Token::Identifier("reason".into()),
                Token::LParen,
                Token::StringLit("loop counter".into()),
                Token::RParen,
                Token::Equals,
                Token::Integer(0),
            ]
        );
    }

    #[test]
    fn match_expression() {
        let tokens = tok("MATCH result => { WHEN SUCCESS(val) -> val }");
        assert_eq!(
            tokens,
            vec![
                Token::Match,
                Token::Identifier("result".into()),
                Token::FatArrow,
                Token::LBrace,
                Token::When,
                Token::Success,
                Token::LParen,
                Token::Identifier("val".into()),
                Token::RParen,
                Token::Arrow,
                Token::Identifier("val".into()),
                Token::RBrace,
            ]
        );
    }

    #[test]
    fn agent_trust_level() {
        let tokens = tok("TRUST_LEVEL ~0.95");
        assert_eq!(
            tokens,
            vec![
                Token::Identifier("TRUST_LEVEL".into()),
                Token::Confidence(0.95),
            ]
        );
    }

    #[test]
    fn agent_timeout_and_memory() {
        let tokens = tok("TIMEOUT_DEFAULT 30s\nMEMORY_LIMIT 256MB");
        assert_eq!(
            tokens,
            vec![
                Token::Identifier("TIMEOUT_DEFAULT".into()),
                Token::Duration("30s".into()),
                Token::Newline,
                Token::Identifier("MEMORY_LIMIT".into()),
                Token::Size("256MB".into()),
            ]
        );
    }

    #[test]
    fn range_expression() {
        let tokens = tok("1..10");
        assert_eq!(
            tokens,
            vec![Token::Integer(1), Token::DotDot, Token::Integer(10)]
        );
    }

    #[test]
    fn confidence_query() {
        let tokens = tok("result?");
        assert_eq!(
            tokens,
            vec![Token::Identifier("result".into()), Token::Question]
        );
    }

    #[test]
    fn delegate_statement() {
        let tokens = tok("DELEGATE task TO worker");
        assert_eq!(
            tokens,
            vec![
                Token::Delegate,
                Token::Identifier("task".into()),
                Token::To,
                Token::Identifier("worker".into()),
            ]
        );
    }

    #[test]
    fn fork_join() {
        let tokens = tok("FORK { a: x, b: y } -> JOIN");
        assert_eq!(
            tokens,
            vec![
                Token::Fork,
                Token::LBrace,
                Token::Identifier("a".into()),
                Token::Colon,
                Token::Identifier("x".into()),
                Token::Comma,
                Token::Identifier("b".into()),
                Token::Colon,
                Token::Identifier("y".into()),
                Token::RBrace,
                Token::Arrow,
                Token::Join,
            ]
        );
    }

    #[test]
    fn boolean_and_none_literals() {
        assert_eq!(tok("TRUE"), vec![Token::True]);
        assert_eq!(tok("FALSE"), vec![Token::False]);
        assert_eq!(tok("NONE"), vec![Token::None]);
    }

    #[test]
    fn logical_operators() {
        let tokens = tok("a AND b OR NOT c");
        assert_eq!(
            tokens,
            vec![
                Token::Identifier("a".into()),
                Token::And,
                Token::Identifier("b".into()),
                Token::Or,
                Token::Not,
                Token::Identifier("c".into()),
            ]
        );
    }

    #[test]
    fn comparison_operators() {
        let tokens = tok("a EQ b NEQ c GT d GTE e LT f LTE g");
        assert_eq!(
            tokens,
            vec![
                Token::Identifier("a".into()),
                Token::Eq,
                Token::Identifier("b".into()),
                Token::Neq,
                Token::Identifier("c".into()),
                Token::Gt,
                Token::Identifier("d".into()),
                Token::Gte,
                Token::Identifier("e".into()),
                Token::Lt,
                Token::Identifier("f".into()),
                Token::Lte,
                Token::Identifier("g".into()),
            ]
        );
    }

    #[test]
    fn arithmetic_operators() {
        let tokens = tok("a + b - c * d / e % f");
        assert_eq!(
            tokens,
            vec![
                Token::Identifier("a".into()),
                Token::Plus,
                Token::Identifier("b".into()),
                Token::Minus,
                Token::Identifier("c".into()),
                Token::Star,
                Token::Identifier("d".into()),
                Token::Slash,
                Token::Identifier("e".into()),
                Token::Percent,
                Token::Identifier("f".into()),
            ]
        );
    }

    #[test]
    fn semicolon_as_terminator() {
        let tokens = tok("STORE x = 1;\nSTORE y = 2;");
        assert_eq!(
            tokens,
            vec![
                Token::Store,
                Token::Identifier("x".into()),
                Token::Equals,
                Token::Integer(1),
                Token::Semicolon,
                Token::Newline,
                Token::Store,
                Token::Identifier("y".into()),
                Token::Equals,
                Token::Integer(2),
                Token::Semicolon,
            ]
        );
    }

    #[test]
    fn newline_as_terminator() {
        let tokens = tok("STORE x = 1\nSTORE y = 2");
        assert_eq!(
            tokens,
            vec![
                Token::Store,
                Token::Identifier("x".into()),
                Token::Equals,
                Token::Integer(1),
                Token::Newline,
                Token::Store,
                Token::Identifier("y".into()),
                Token::Equals,
                Token::Integer(2),
            ]
        );
    }

    #[test]
    fn eof_always_last() {
        let spanned = tok_full("");
        assert_eq!(spanned.len(), 1);
        assert_eq!(spanned[0].token, Token::Eof);

        let spanned2 = tok_full("a");
        assert_eq!(spanned2.last().unwrap().token, Token::Eof);
    }

    #[test]
    fn empty_source() {
        let tokens = tok("");
        assert!(tokens.is_empty());
    }

    #[test]
    fn whitespace_only() {
        let tokens = tok("   \t  ");
        assert!(tokens.is_empty());
    }

    #[test]
    fn error_on_unexpected_character() {
        let result = tokenize("foo ` bar");
        assert!(result.is_err());
        let diags = result.unwrap_err();
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("unexpected character"));
    }

    #[test]
    fn error_unterminated_string() {
        let result = tokenize(r#""hello"#);
        assert!(result.is_err());
        let diags = result.unwrap_err();
        assert!(diags[0].message.contains("unterminated string"));
    }

    // ── Multiline continuation tests ──────────────────────────────────

    #[test]
    fn multiline_pipeline_continuation() {
        let tokens = tok("a ->\n  b |>\n  c");
        assert_eq!(
            tokens,
            vec![
                Token::Identifier("a".into()),
                Token::Arrow,
                Token::Identifier("b".into()),
                Token::PipeForward,
                Token::Identifier("c".into()),
            ]
        );
    }

    #[test]
    fn multiline_function_args() {
        let tokens = tok("f(\n  a,\n  b,\n  c\n)");
        assert_eq!(
            tokens,
            vec![
                Token::Identifier("f".into()),
                Token::LParen,
                Token::Identifier("a".into()),
                Token::Comma,
                Token::Identifier("b".into()),
                Token::Comma,
                Token::Identifier("c".into()),
                Token::RParen,
            ]
        );
    }

    #[test]
    fn multiline_list() {
        let tokens = tok("[\n  1,\n  2,\n  3\n]");
        assert_eq!(
            tokens,
            vec![
                Token::LBracket,
                Token::Integer(1),
                Token::Comma,
                Token::Integer(2),
                Token::Comma,
                Token::Integer(3),
                Token::RBracket,
            ]
        );
    }

    #[test]
    fn leading_newlines_suppressed() {
        let tokens = tok("\n\n\nfoo");
        assert_eq!(tokens, vec![Token::Identifier("foo".into())]);
    }

    // ── Complex real-world-like snippet ───────────────────────────────

    #[test]
    fn operation_snippet() {
        let src = "OPERATION Validate =>\n  INPUT data: Record\n  OUTPUT Result[Record]\n  REQUIRE data.fields GT 0\n  BODY {\n    STORE validated = check(data)\n    EMIT validated\n  }";
        let tokens = tok(src);
        assert!(tokens.contains(&Token::Operation));
        assert!(tokens.contains(&Token::Input));
        assert!(tokens.contains(&Token::Output));
        assert!(tokens.contains(&Token::Require));
        assert!(tokens.contains(&Token::Body));
        assert!(tokens.contains(&Token::Store));
        assert!(tokens.contains(&Token::Emit));
        assert!(tokens.contains(&Token::Gt));
    }

    #[test]
    fn agent_snippet() {
        let src = "AGENT Planner =>\n  CAPABILITIES [plan, delegate]\n  TRUST_LEVEL ~0.9\n  TIMEOUT_DEFAULT 30s\n  MEMORY_LIMIT 256MB";
        let tokens = tok(src);
        assert!(tokens.contains(&Token::Agent));
        assert!(tokens.contains(&Token::Confidence(0.9)));
        assert!(tokens.contains(&Token::Duration("30s".into())));
        assert!(tokens.contains(&Token::Size("256MB".into())));
    }

    #[test]
    fn loop_with_max() {
        let tokens = tok("LOOP max: 10 => { EMIT x }");
        assert_eq!(
            tokens,
            vec![
                Token::Loop,
                Token::Identifier("max".into()),
                Token::Colon,
                Token::Integer(10),
                Token::FatArrow,
                Token::LBrace,
                Token::Emit,
                Token::Identifier("x".into()),
                Token::RBrace,
            ]
        );
    }

    #[test]
    fn retry_and_escalate() {
        let tokens = tok("RETRY(3)\nESCALATE");
        assert_eq!(
            tokens,
            vec![
                Token::Retry,
                Token::LParen,
                Token::Integer(3),
                Token::RParen,
                Token::Newline,
                Token::Escalate,
            ]
        );
    }

    #[test]
    fn checkpoint_and_resume() {
        let tokens = tok(r#"CHECKPOINT "save1"; RESUME(state)"#);
        assert_eq!(
            tokens,
            vec![
                Token::Checkpoint,
                Token::StringLit("save1".into()),
                Token::Semicolon,
                Token::Resume,
                Token::LParen,
                Token::Identifier("state".into()),
                Token::RParen,
            ]
        );
    }

    #[test]
    fn halt_statement() {
        let tokens = tok(r#"HALT(timeout, "exceeded limit")"#);
        assert_eq!(
            tokens,
            vec![
                Token::Halt,
                Token::LParen,
                Token::Identifier("timeout".into()),
                Token::Comma,
                Token::StringLit("exceeded limit".into()),
                Token::RParen,
            ]
        );
    }

    #[test]
    fn at_reason_annotation() {
        let tokens = tok(r#"@reason("needs mutation")"#);
        assert_eq!(
            tokens,
            vec![
                Token::At,
                Token::Identifier("reason".into()),
                Token::LParen,
                Token::StringLit("needs mutation".into()),
                Token::RParen,
            ]
        );
    }

    #[test]
    fn constrained_type() {
        let tokens = tok("Int64 :: range(0, 100)");
        assert_eq!(
            tokens,
            vec![
                Token::Identifier("Int64".into()),
                Token::DoubleColon,
                Token::Identifier("range".into()),
                Token::LParen,
                Token::Integer(0),
                Token::Comma,
                Token::Integer(100),
                Token::RParen,
            ]
        );
    }

    #[test]
    fn negative_integer_as_minus_then_int() {
        // The lexer does not handle negative numbers as a single token.
        // The parser combines Minus + Integer.
        let tokens = tok("-7");
        assert_eq!(tokens, vec![Token::Minus, Token::Integer(7)]);
    }

    #[test]
    fn windows_line_endings() {
        let tokens = tok("a\r\nb\r\nc");
        assert_eq!(
            tokens,
            vec![
                Token::Identifier("a".into()),
                Token::Newline,
                Token::Identifier("b".into()),
                Token::Newline,
                Token::Identifier("c".into()),
            ]
        );
    }

    #[test]
    fn mixed_terminators() {
        let tokens = tok("a;\nb\nc;");
        assert_eq!(
            tokens,
            vec![
                Token::Identifier("a".into()),
                Token::Semicolon,
                Token::Newline,
                Token::Identifier("b".into()),
                Token::Newline,
                Token::Identifier("c".into()),
                Token::Semicolon,
            ]
        );
    }

    // ── Property-based tests ────────────────────────────────────────

    mod proptest_lexer {
        use super::*;
        use proptest::prelude::*;

        /// Strategy for valid AgentLang identifiers.
        fn identifier_strategy() -> impl Strategy<Value = String> {
            "[a-zA-Z_][a-zA-Z0-9_]{0,15}".prop_filter("not a keyword", |s| {
                !matches!(
                    s.as_str(),
                    "TYPE" | "SCHEMA" | "AGENT" | "OPERATION" | "PIPELINE"
                        | "BODY" | "INPUT" | "OUTPUT" | "REQUIRE" | "ENSURE"
                        | "INVARIANT" | "STORE" | "MUTABLE" | "MATCH" | "WHEN"
                        | "OTHERWISE" | "LOOP" | "EMIT" | "ASSERT" | "RETRY"
                        | "ESCALATE" | "CHECKPOINT" | "RESUME" | "HALT"
                        | "DELEGATE" | "TO" | "FORK" | "JOIN" | "SUCCESS"
                        | "FAILURE" | "TRUE" | "FALSE" | "NONE" | "AND" | "OR"
                        | "NOT" | "EQ" | "NEQ" | "GT" | "GTE" | "LT" | "LTE"
                        | "max" | "strategy"
                )
            })
        }

        proptest! {
            /// Any valid integer literal should lex to exactly one Integer token + EOF.
            #[test]
            fn lex_integer_never_panics(n in 0i64..1_000_000) {
                let source = n.to_string();
                let tokens = tokenize(&source).unwrap();
                // Should have at least the integer token + EOF
                prop_assert!(tokens.len() >= 2);
                prop_assert!(matches!(&tokens[0].token, Token::Integer(v) if *v == n));
            }

            /// Any valid identifier should lex correctly.
            #[test]
            fn lex_identifier_never_panics(id in identifier_strategy()) {
                let tokens = tokenize(&id).unwrap();
                prop_assert!(tokens.len() >= 2);
                prop_assert!(matches!(&tokens[0].token, Token::Identifier(s) if s == &id));
            }

            /// Arbitrary string inputs should never cause a panic (just errors).
            #[test]
            fn lex_arbitrary_no_panic(source in "[ -~]{0,100}") {
                // Should either succeed or return a diagnostic, never panic.
                let _ = tokenize(&source);
            }

            /// String literals with escaped content should lex.
            #[test]
            fn lex_string_literal(content in "[a-zA-Z0-9 ]{0,30}") {
                let source = format!(r#""{}""#, content);
                let tokens = tokenize(&source).unwrap();
                prop_assert!(tokens.len() >= 2);
                prop_assert!(matches!(&tokens[0].token, Token::StringLit(_)));
            }

            /// Keywords are recognized as tokens, not identifiers.
            #[test]
            fn lex_keyword_recognized(kw in prop::sample::select(vec![
                "TYPE", "SCHEMA", "AGENT", "OPERATION", "PIPELINE",
                "BODY", "INPUT", "OUTPUT", "EMIT", "STORE", "MUTABLE",
            ])) {
                let tokens = tokenize(kw).unwrap();
                prop_assert!(tokens.len() >= 2);
                // Should NOT be an Identifier
                prop_assert!(!matches!(&tokens[0].token, Token::Identifier(_)));
            }

            /// Whitespace-separated tokens always produce valid spans.
            #[test]
            fn lex_spans_valid(
                a in identifier_strategy(),
                b in identifier_strategy()
            ) {
                let source = format!("{} {}", a, b);
                let tokens = tokenize(&source).unwrap();
                for tok in &tokens {
                    prop_assert!(tok.span.line >= 1);
                    prop_assert!(tok.span.column >= 1);
                }
            }
        }
    }
}
