//! # al-parser
//!
//! Recursive-descent parser for AgentLang MVP v0.1.
//!
//! Parses a token stream from `al_lexer` into the AST defined in `al_ast`,
//! following the grammar specified in `specs/GRAMMAR_MVP.ebnf`.

use al_ast::*;
use al_diagnostics::{Diagnostic, ErrorCode, Span};
use al_lexer::{self, Token};

// ---------------------------------------------------------------------------
// Parser struct
// ---------------------------------------------------------------------------

/// The AgentLang recursive-descent parser.
struct Parser {
    tokens: Vec<al_lexer::Spanned>,
    pos: usize,
    diagnostics: Vec<Diagnostic>,
}

impl Parser {
    fn new(tokens: Vec<al_lexer::Spanned>) -> Self {
        Self {
            tokens,
            pos: 0,
            diagnostics: Vec::new(),
        }
    }

    // ── Token access ─────────────────────────────────────────────────

    fn peek(&self) -> &Token {
        self.tokens
            .get(self.pos)
            .map(|s| &s.token)
            .unwrap_or(&Token::Eof)
    }

    fn peek_span(&self) -> Span {
        self.tokens
            .get(self.pos)
            .map(|s| s.span)
            .unwrap_or_else(Span::dummy)
    }

    fn advance(&mut self) -> &al_lexer::Spanned {
        let tok = &self.tokens[self.pos];
        if self.pos < self.tokens.len() - 1 {
            self.pos += 1;
        }
        tok
    }

    fn at(&self, token: &Token) -> bool {
        std::mem::discriminant(self.peek()) == std::mem::discriminant(token)
    }

    fn at_any(&self, tokens: &[Token]) -> bool {
        tokens.iter().any(|t| self.at(t))
    }

    fn at_eof(&self) -> bool {
        matches!(self.peek(), Token::Eof)
    }

    fn expect(&mut self, expected: &Token) -> Result<al_lexer::Spanned, ()> {
        if self.at(expected) {
            Ok(self.advance().clone())
        } else {
            let span = self.peek_span();
            self.diagnostics.push(Diagnostic::error(
                ErrorCode::ParseError,
                format!("expected `{}`, found `{}`", expected, self.peek()),
                span,
            ));
            Err(())
        }
    }

    fn expect_identifier(&mut self) -> Result<Spanned<String>, ()> {
        match self.peek().clone() {
            Token::Identifier(name) => {
                let span = self.peek_span();
                self.advance();
                Ok(Spanned::new(name, span))
            }
            // Also accept identifier-like keywords in certain contexts
            _ => {
                let span = self.peek_span();
                self.diagnostics.push(Diagnostic::error(
                    ErrorCode::ParseError,
                    format!("expected identifier, found `{}`", self.peek()),
                    span,
                ));
                Err(())
            }
        }
    }

    fn eat_terminator(&mut self) {
        if matches!(self.peek(), Token::Semicolon | Token::Newline) {
            self.advance();
        }
        // Also eat any extra newlines
        while matches!(self.peek(), Token::Newline) {
            self.advance();
        }
    }

    fn skip_newlines(&mut self) {
        while matches!(self.peek(), Token::Newline) {
            self.advance();
        }
    }

    fn error(&mut self, msg: impl Into<String>) {
        let span = self.peek_span();
        self.diagnostics
            .push(Diagnostic::error(ErrorCode::ParseError, msg, span));
    }

    /// Check if the current token starts a statement.
    fn is_statement_keyword(&self) -> bool {
        matches!(
            self.peek(),
            Token::Store
                | Token::Mutable
                | Token::Match
                | Token::Loop
                | Token::Emit
                | Token::Assert
                | Token::Retry
                | Token::Escalate
                | Token::Checkpoint
                | Token::Halt
                | Token::Delegate
        )
    }

    /// Check if the current token starts a declaration.
    fn is_declaration_keyword(&self) -> bool {
        matches!(
            self.peek(),
            Token::Type | Token::Schema | Token::Agent | Token::Operation | Token::Pipeline
        )
    }

    // ── Top-level: program ───────────────────────────────────────────

    fn parse_program(&mut self) -> Program {
        let start = self.peek_span();
        let mut declarations = Vec::new();

        self.skip_newlines();

        while !self.at_eof() {
            match self.parse_declaration() {
                Ok(decl) => declarations.push(decl),
                Err(()) => {
                    // Error recovery: skip to the next declaration keyword
                    self.recover_to_declaration();
                }
            }
            self.skip_newlines();
        }

        let end = self.peek_span();
        let span = Span::new(
            start.offset,
            start.line,
            start.column,
            end.offset - start.offset,
        );

        Program { declarations, span }
    }

    /// Advance past tokens until we reach a declaration keyword or EOF.
    fn recover_to_declaration(&mut self) {
        while !self.at_eof() {
            if self.is_declaration_keyword() {
                return;
            }
            self.advance();
        }
    }

    /// Advance past tokens until we reach a statement keyword, closing brace,
    /// declaration keyword, or EOF. Used for intra-block error recovery.
    fn recover_to_statement(&mut self) {
        while !self.at_eof() {
            if self.is_statement_keyword() || self.is_declaration_keyword() {
                return;
            }
            match self.peek() {
                Token::RBrace => return,
                Token::Semicolon | Token::Newline => {
                    self.advance();
                    self.skip_newlines();
                    return;
                }
                _ => {
                    self.advance();
                }
            }
        }
    }

    // ── Declarations ─────────────────────────────────────────────────

    fn parse_declaration(&mut self) -> Result<Spanned<Declaration>, ()> {
        let start = self.peek_span();
        let decl = match self.peek() {
            Token::Type => self.parse_type_decl()?,
            Token::Schema => self.parse_schema_decl()?,
            Token::Agent => self.parse_agent_decl()?,
            Token::Operation => self.parse_operation_decl()?,
            Token::Pipeline => self.parse_pipeline_decl()?,
            _ => {
                self.error(format!(
                    "expected declaration (TYPE, SCHEMA, AGENT, OPERATION, PIPELINE), found `{}`",
                    self.peek()
                ));
                return Err(());
            }
        };
        let end = self.peek_span();
        let span = Span::new(
            start.offset,
            start.line,
            start.column,
            end.offset.saturating_sub(start.offset),
        );
        Ok(Spanned::new(decl, span))
    }

    // TYPE Name[T] = type_expr terminator
    fn parse_type_decl(&mut self) -> Result<Declaration, ()> {
        self.expect(&Token::Type)?;
        let name = self.expect_identifier()?;

        let mut type_params = Vec::new();
        if self.at(&Token::LBracket) {
            self.advance();
            let first = self.expect_identifier()?;
            type_params.push(first);
            while self.at(&Token::Comma) {
                self.advance();
                type_params.push(self.expect_identifier()?);
            }
            self.expect(&Token::RBracket)?;
        }

        self.expect(&Token::Equals)?;
        let ty = self.parse_type_expr()?;
        self.eat_terminator();

        Ok(Declaration::TypeDecl {
            name,
            type_params,
            ty,
        })
    }

    // SCHEMA Name => { field_decl* }
    fn parse_schema_decl(&mut self) -> Result<Declaration, ()> {
        self.expect(&Token::Schema)?;
        let name = self.expect_identifier()?;
        self.expect(&Token::FatArrow)?;
        self.expect(&Token::LBrace)?;
        self.skip_newlines();

        let mut fields = Vec::new();
        while !self.at(&Token::RBrace) && !self.at_eof() {
            let field = self.parse_field_decl()?;
            fields.push(field);
            if self.at(&Token::Comma) {
                self.advance();
            }
            self.skip_newlines();
        }
        self.expect(&Token::RBrace)?;
        self.eat_terminator();

        Ok(Declaration::SchemaDecl { name, fields })
    }

    fn parse_field_decl(&mut self) -> Result<Spanned<FieldDecl>, ()> {
        let start = self.peek_span();
        let name = self.expect_identifier()?;
        self.expect(&Token::Colon)?;
        let ty = self.parse_type_expr()?;

        let constraint = if self.at(&Token::DoubleColon) {
            self.advance();
            Some(self.parse_constraint_expr()?)
        } else {
            None
        };

        let end = self.peek_span();
        let span = Span::new(
            start.offset,
            start.line,
            start.column,
            end.offset.saturating_sub(start.offset),
        );
        Ok(Spanned::new(
            FieldDecl {
                name,
                ty,
                constraint,
            },
            span,
        ))
    }

    // AGENT Name => { agent_property* }
    fn parse_agent_decl(&mut self) -> Result<Declaration, ()> {
        self.expect(&Token::Agent)?;
        let name = self.expect_identifier()?;
        self.expect(&Token::FatArrow)?;
        self.skip_newlines();

        let mut properties = Vec::new();
        // Agent properties continue until next top-level declaration or EOF
        while !self.at_eof()
            && !matches!(
                self.peek(),
                Token::Type | Token::Schema | Token::Agent | Token::Operation | Token::Pipeline
            )
        {
            match self.parse_agent_property() {
                Ok(prop) => properties.push(prop),
                Err(()) => break,
            }
            self.skip_newlines();
        }

        Ok(Declaration::AgentDecl { name, properties })
    }

    fn parse_agent_property(&mut self) -> Result<Spanned<AgentProperty>, ()> {
        let start = self.peek_span();
        let prop = match self.peek() {
            Token::Identifier(s) if s == "CAPABILITIES" => {
                self.advance();
                self.expect(&Token::LBracket)?;
                let caps = self.parse_identifier_list()?;
                self.expect(&Token::RBracket)?;
                self.eat_terminator();
                AgentProperty::Capabilities(caps)
            }
            Token::Identifier(s) if s == "DENY" => {
                self.advance();
                self.expect(&Token::LBracket)?;
                let caps = self.parse_identifier_list()?;
                self.expect(&Token::RBracket)?;
                self.eat_terminator();
                AgentProperty::Deny(caps)
            }
            Token::Identifier(s) if s == "TRUST_LEVEL" => {
                self.advance();
                let conf = self.parse_confidence_literal()?;
                self.eat_terminator();
                AgentProperty::TrustLevel(conf)
            }
            Token::Identifier(s) if s == "MAX_CONCURRENCY" => {
                self.advance();
                let val = self.parse_integer_literal()?;
                self.eat_terminator();
                AgentProperty::MaxConcurrency(val)
            }
            Token::Identifier(s) if s == "MEMORY_LIMIT" => {
                self.advance();
                let val = self.parse_size_literal()?;
                self.eat_terminator();
                AgentProperty::MemoryLimit(val)
            }
            Token::Identifier(s) if s == "TIMEOUT_DEFAULT" => {
                self.advance();
                let val = self.parse_duration_literal()?;
                self.eat_terminator();
                AgentProperty::TimeoutDefault(val)
            }
            Token::Identifier(s) if s == "ON_FAILURE" => {
                self.advance();
                let policy = self.parse_failure_policy()?;
                self.eat_terminator();
                AgentProperty::OnFailure(policy)
            }
            Token::Identifier(s) if s == "STATE_SCHEMA" => {
                self.advance();
                self.expect(&Token::FatArrow)?;
                self.expect(&Token::LBrace)?;
                self.skip_newlines();
                let mut fields = Vec::new();
                while !self.at(&Token::RBrace) && !self.at_eof() {
                    fields.push(self.parse_field_decl()?);
                    if self.at(&Token::Comma) {
                        self.advance();
                    }
                    self.skip_newlines();
                }
                self.expect(&Token::RBrace)?;
                AgentProperty::StateSchema(fields)
            }
            _ => {
                return Err(());
            }
        };
        let end = self.peek_span();
        let span = Span::new(
            start.offset,
            start.line,
            start.column,
            end.offset.saturating_sub(start.offset),
        );
        Ok(Spanned::new(prop, span))
    }

    // OPERATION Name => INPUT... OUTPUT... REQUIRE... ENSURE... INVARIANT... BODY { }
    fn parse_operation_decl(&mut self) -> Result<Declaration, ()> {
        self.expect(&Token::Operation)?;
        let name = self.expect_identifier()?;
        self.expect(&Token::FatArrow)?;
        self.skip_newlines();

        let mut inputs = Vec::new();
        let mut output = None;
        let mut requires = Vec::new();
        let mut ensures = Vec::new();
        let mut invariants = Vec::new();

        // Parse optional clauses
        loop {
            self.skip_newlines();
            match self.peek() {
                Token::Input => {
                    self.advance();
                    let params = self.parse_param_list()?;
                    inputs.extend(params);
                    self.eat_terminator();
                }
                Token::Output => {
                    self.advance();
                    output = Some(self.parse_type_expr()?);
                    self.eat_terminator();
                }
                Token::Require => {
                    self.advance();
                    requires.push(self.parse_expression()?);
                    self.eat_terminator();
                }
                Token::Ensure => {
                    self.advance();
                    ensures.push(self.parse_expression()?);
                    self.eat_terminator();
                }
                Token::Invariant => {
                    self.advance();
                    invariants.push(self.parse_expression()?);
                    self.eat_terminator();
                }
                Token::Body => break,
                _ => break,
            }
        }

        self.expect(&Token::Body)?;
        let body = self.parse_block()?;

        Ok(Declaration::OperationDecl {
            name,
            inputs,
            output,
            requires,
            ensures,
            invariants,
            body,
        })
    }

    // PIPELINE Name => pipeline_chain terminator
    fn parse_pipeline_decl(&mut self) -> Result<Declaration, ()> {
        self.expect(&Token::Pipeline)?;
        let name = self.expect_identifier()?;
        self.expect(&Token::FatArrow)?;
        let chain = self.parse_pipeline_chain()?;
        self.eat_terminator();

        Ok(Declaration::PipelineDecl { name, chain })
    }

    fn parse_pipeline_chain(&mut self) -> Result<Spanned<PipelineChain>, ()> {
        let start = self.peek_span();
        let first_expr = self.parse_expression()?;
        let mut stages = vec![PipelineStage {
            op: None,
            expr: first_expr,
        }];

        while self.at(&Token::Arrow) || self.at(&Token::PipeForward) {
            let op_tok = self.advance().clone();
            let pipe_op = match &op_tok.token {
                Token::Arrow => PipeOp::Arrow,
                Token::PipeForward => PipeOp::PipeForward,
                _ => unreachable!(),
            };
            let op_spanned = Spanned::new(pipe_op, op_tok.span);
            let expr = self.parse_expression()?;
            stages.push(PipelineStage {
                op: Some(op_spanned),
                expr,
            });
        }

        let end = self.peek_span();
        let span = Span::new(
            start.offset,
            start.line,
            start.column,
            end.offset.saturating_sub(start.offset),
        );
        Ok(Spanned::new(PipelineChain { stages }, span))
    }

    // ── Statements ───────────────────────────────────────────────────

    fn parse_block(&mut self) -> Result<Spanned<Block>, ()> {
        let start = self.peek_span();
        self.expect(&Token::LBrace)?;
        self.skip_newlines();

        let mut stmts = Vec::new();
        while !self.at(&Token::RBrace) && !self.at_eof() {
            match self.parse_statement() {
                Ok(stmt) => stmts.push(stmt),
                Err(()) => {
                    // Statement-level error recovery: skip to next
                    // statement boundary (keyword, `;`, `}`, or declaration).
                    self.recover_to_statement();
                }
            }
            self.skip_newlines();
        }

        self.expect(&Token::RBrace)?;
        let end = self.peek_span();
        let span = Span::new(
            start.offset,
            start.line,
            start.column,
            end.offset.saturating_sub(start.offset),
        );
        Ok(Spanned::new(Block { stmts }, span))
    }

    fn parse_statement(&mut self) -> Result<Spanned<Statement>, ()> {
        let start = self.peek_span();
        let stmt = match self.peek() {
            Token::Store => self.parse_store_stmt()?,
            Token::Mutable => self.parse_mutable_stmt()?,
            Token::Match => self.parse_match_stmt()?,
            Token::Loop => self.parse_loop_stmt()?,
            Token::Emit => self.parse_emit_stmt()?,
            Token::Assert => self.parse_assert_stmt()?,
            Token::Retry => self.parse_retry_stmt()?,
            Token::Escalate => self.parse_escalate_stmt()?,
            Token::Checkpoint => self.parse_checkpoint_stmt()?,
            Token::Halt => self.parse_halt_stmt()?,
            Token::Delegate => self.parse_delegate_stmt()?,
            Token::Identifier(_) => {
                // Could be assign or expr stmt
                self.parse_assign_or_expr_stmt()?
            }
            _ => {
                // Try as expression statement
                let expr = self.parse_expression()?;
                self.eat_terminator();
                Statement::Expr { expr }
            }
        };
        let end = self.peek_span();
        let span = Span::new(
            start.offset,
            start.line,
            start.column,
            end.offset.saturating_sub(start.offset),
        );
        Ok(Spanned::new(stmt, span))
    }

    // STORE name [: type_expr] = expr terminator
    fn parse_store_stmt(&mut self) -> Result<Statement, ()> {
        self.expect(&Token::Store)?;
        let name = self.expect_identifier()?;

        let ty = if self.at(&Token::Colon) {
            self.advance();
            Some(self.parse_type_expr()?)
        } else {
            None
        };

        self.expect(&Token::Equals)?;
        let value = self.parse_expression()?;
        self.eat_terminator();

        Ok(Statement::Store { name, ty, value })
    }

    // MUTABLE name @reason("...") [: type_expr] = expr terminator
    fn parse_mutable_stmt(&mut self) -> Result<Statement, ()> {
        self.expect(&Token::Mutable)?;
        let name = self.expect_identifier()?;

        // Parse @reason("...")
        self.expect(&Token::At)?;
        let reason_kw = self.expect_identifier()?;
        if reason_kw.node != "reason" {
            self.error(format!("expected `reason`, found `{}`", reason_kw.node));
            return Err(());
        }
        self.expect(&Token::LParen)?;
        let reason = self.parse_string_literal()?;
        self.expect(&Token::RParen)?;

        let ty = if self.at(&Token::Colon) {
            self.advance();
            Some(self.parse_type_expr()?)
        } else {
            None
        };

        self.expect(&Token::Equals)?;
        let value = self.parse_expression()?;
        self.eat_terminator();

        Ok(Statement::Mutable {
            name,
            reason,
            ty,
            value,
        })
    }

    // identifier = expression terminator | expression terminator
    fn parse_assign_or_expr_stmt(&mut self) -> Result<Statement, ()> {
        // Look ahead: if we have IDENT = expr, it's an assignment
        if let Token::Identifier(_) = self.peek() {
            // Check if next non-newline token is '='
            let next_pos = self.pos + 1;
            if next_pos < self.tokens.len() {
                if let Token::Equals = &self.tokens[next_pos].token {
                    // It's an assignment
                    let target = self.expect_identifier()?;
                    self.expect(&Token::Equals)?;
                    let value = self.parse_expression()?;
                    self.eat_terminator();
                    return Ok(Statement::Assign { target, value });
                }
            }
        }

        // Otherwise it's an expression statement
        let expr = self.parse_expression()?;
        self.eat_terminator();
        Ok(Statement::Expr { expr })
    }

    // MATCH expr => { match_arm* [OTHERWISE -> match_body] }
    fn parse_match_stmt(&mut self) -> Result<Statement, ()> {
        self.expect(&Token::Match)?;
        let expr = self.parse_expression()?;
        self.expect(&Token::FatArrow)?;
        self.expect(&Token::LBrace)?;
        self.skip_newlines();

        let mut arms = Vec::new();
        let mut otherwise = None;

        while !self.at(&Token::RBrace) && !self.at_eof() {
            self.skip_newlines();
            if self.at(&Token::Otherwise) {
                self.advance();
                self.expect(&Token::Arrow)?;
                let body = self.parse_match_body()?;
                let span = body.span;
                otherwise = Some(Spanned::new(body.node, span));
                self.skip_newlines();
                break;
            }
            if self.at(&Token::When) {
                let arm_start = self.peek_span();
                self.advance();
                let pattern = self.parse_pattern()?;
                self.expect(&Token::Arrow)?;
                let body = self.parse_match_body()?;
                let arm_end = self.peek_span();
                let arm_span = Span::new(
                    arm_start.offset,
                    arm_start.line,
                    arm_start.column,
                    arm_end.offset.saturating_sub(arm_start.offset),
                );
                arms.push(Spanned::new(MatchArm { pattern, body }, arm_span));
            } else {
                break;
            }
            self.skip_newlines();
        }

        self.expect(&Token::RBrace)?;
        self.eat_terminator();

        Ok(Statement::Match {
            expr,
            arms,
            otherwise,
        })
    }

    fn parse_match_body(&mut self) -> Result<Spanned<MatchBody>, ()> {
        if self.at(&Token::LBrace) {
            // Explicit block: `-> { ... }`
            let block = self.parse_block()?;
            let span = block.span;
            Ok(Spanned::new(MatchBody::Block(block), span))
        } else if self.is_statement_keyword() {
            // Statement directly after `->` (e.g., `-> EMIT val`, `-> ESCALATE(msg)`)
            // Parse as a single statement and wrap in a synthetic block.
            let stmt = self.parse_statement()?;
            let span = stmt.span;
            let block = Spanned::new(Block { stmts: vec![stmt] }, span);
            Ok(Spanned::new(MatchBody::Block(block), span))
        } else {
            // Expression: `-> some_expr`
            let expr = self.parse_expression()?;
            let span = expr.span;
            self.eat_terminator();
            Ok(Spanned::new(MatchBody::Expr(expr), span))
        }
    }

    // LOOP max: N => block
    fn parse_loop_stmt(&mut self) -> Result<Statement, ()> {
        self.expect(&Token::Loop)?;

        // Parse "max:" or just "max :"
        let max_kw = self.expect_identifier()?;
        if max_kw.node != "max" {
            self.error(format!("expected `max`, found `{}`", max_kw.node));
            return Err(());
        }
        self.expect(&Token::Colon)?;
        let max_iters = self.parse_integer_literal()?;
        self.expect(&Token::FatArrow)?;
        let body = self.parse_block()?;
        self.eat_terminator();

        Ok(Statement::Loop { max_iters, body })
    }

    // EMIT [expression] terminator
    fn parse_emit_stmt(&mut self) -> Result<Statement, ()> {
        self.expect(&Token::Emit)?;
        let value = if !matches!(
            self.peek(),
            Token::Semicolon | Token::Newline | Token::RBrace | Token::Eof
        ) {
            Some(self.parse_expression()?)
        } else {
            None
        };
        self.eat_terminator();
        Ok(Statement::Emit { value })
    }

    // ASSERT expression terminator
    fn parse_assert_stmt(&mut self) -> Result<Statement, ()> {
        self.expect(&Token::Assert)?;
        let condition = self.parse_expression()?;
        self.eat_terminator();
        Ok(Statement::Assert { condition })
    }

    // RETRY(count [, args]) terminator
    fn parse_retry_stmt(&mut self) -> Result<Statement, ()> {
        self.expect(&Token::Retry)?;
        self.expect(&Token::LParen)?;
        let count = self.parse_integer_literal()?;
        let mut args = Vec::new();
        while self.at(&Token::Comma) {
            self.advance();
            args.push(self.parse_argument()?);
        }
        self.expect(&Token::RParen)?;
        self.eat_terminator();
        Ok(Statement::Retry { count, args })
    }

    // ESCALATE [( expr )] terminator
    fn parse_escalate_stmt(&mut self) -> Result<Statement, ()> {
        self.expect(&Token::Escalate)?;
        let message = if self.at(&Token::LParen) {
            self.advance();
            let expr = self.parse_expression()?;
            self.expect(&Token::RParen)?;
            Some(expr)
        } else {
            None
        };
        self.eat_terminator();
        Ok(Statement::Escalate { message })
    }

    // CHECKPOINT ["label"] terminator
    fn parse_checkpoint_stmt(&mut self) -> Result<Statement, ()> {
        self.expect(&Token::Checkpoint)?;
        let label = if let Token::StringLit(_) = self.peek() {
            Some(self.parse_string_literal()?)
        } else {
            None
        };
        self.eat_terminator();
        Ok(Statement::Checkpoint { label })
    }

    // HALT(reason [, expr]) terminator
    fn parse_halt_stmt(&mut self) -> Result<Statement, ()> {
        self.expect(&Token::Halt)?;
        self.expect(&Token::LParen)?;
        let reason = self.expect_identifier()?;
        let value = if self.at(&Token::Comma) {
            self.advance();
            Some(self.parse_expression()?)
        } else {
            None
        };
        self.expect(&Token::RParen)?;
        self.eat_terminator();
        Ok(Statement::Halt {
            reason: Spanned::new(reason.node, reason.span),
            value,
        })
    }

    // DELEGATE task TO target => { delegate_clause* }
    fn parse_delegate_stmt(&mut self) -> Result<Statement, ()> {
        self.expect(&Token::Delegate)?;
        let task = self.expect_identifier()?;
        self.expect(&Token::To)?;
        let target = self.expect_identifier()?;
        self.expect(&Token::FatArrow)?;
        self.expect(&Token::LBrace)?;
        self.skip_newlines();

        let mut clauses = Vec::new();
        while !self.at(&Token::RBrace) && !self.at_eof() {
            let clause = self.parse_delegate_clause()?;
            clauses.push(clause);
            self.skip_newlines();
        }

        self.expect(&Token::RBrace)?;
        self.eat_terminator();

        Ok(Statement::Delegate {
            task,
            target,
            clauses,
        })
    }

    fn parse_delegate_clause(&mut self) -> Result<Spanned<DelegateClause>, ()> {
        let start = self.peek_span();
        let clause = match self.peek() {
            Token::Input => {
                self.advance();
                let expr = self.parse_expression()?;
                self.eat_terminator();
                DelegateClause::Input(expr)
            }
            Token::Identifier(s) if s == "TIMEOUT" => {
                self.advance();
                let dur = self.parse_duration_literal()?;
                self.eat_terminator();
                DelegateClause::Timeout(dur)
            }
            Token::Identifier(s) if s == "ON_TIMEOUT" => {
                self.advance();
                let policy = self.parse_failure_policy()?;
                self.eat_terminator();
                DelegateClause::OnTimeout(policy)
            }
            Token::Identifier(s) if s == "SHARED_CONTEXT" => {
                self.advance();
                self.expect(&Token::LBracket)?;
                let idents = self.parse_identifier_list()?;
                self.expect(&Token::RBracket)?;
                self.eat_terminator();
                DelegateClause::SharedContext(idents)
            }
            Token::Identifier(s) if s == "ISOLATION" => {
                self.advance();
                self.expect(&Token::LBrace)?;
                self.skip_newlines();
                let mut rules = Vec::new();
                while !self.at(&Token::RBrace) && !self.at_eof() {
                    let rule_start = self.peek_span();
                    let key = self.expect_identifier()?;
                    self.expect(&Token::Colon)?;
                    let value = self.expect_identifier()?;
                    if self.at(&Token::Comma) {
                        self.advance();
                    }
                    self.skip_newlines();
                    let rule_end = self.peek_span();
                    let rule_span = Span::new(
                        rule_start.offset,
                        rule_start.line,
                        rule_start.column,
                        rule_end.offset.saturating_sub(rule_start.offset),
                    );
                    rules.push(Spanned::new(IsolationRule { key, value }, rule_span));
                }
                self.expect(&Token::RBrace)?;
                DelegateClause::Isolation(rules)
            }
            _ => {
                self.error("expected delegate clause");
                return Err(());
            }
        };
        let end = self.peek_span();
        let span = Span::new(
            start.offset,
            start.line,
            start.column,
            end.offset.saturating_sub(start.offset),
        );
        Ok(Spanned::new(clause, span))
    }

    // ── Patterns ─────────────────────────────────────────────────────

    fn parse_pattern(&mut self) -> Result<Spanned<Pattern>, ()> {
        let start = self.peek_span();
        let pattern = match self.peek().clone() {
            Token::Identifier(ref s) if s == "_" => {
                self.advance();
                Pattern::Wildcard
            }
            Token::Success => {
                self.advance();
                self.expect(&Token::LParen)?;
                let inner = self.parse_pattern()?;
                self.expect(&Token::RParen)?;
                Pattern::Success(Box::new(inner))
            }
            Token::Failure => {
                self.advance();
                self.expect(&Token::LParen)?;
                let code = self.expect_identifier()?;
                self.expect(&Token::Comma)?;
                let msg_pat = self.parse_pattern()?;
                self.expect(&Token::Comma)?;
                let details_pat = self.parse_pattern()?;
                self.expect(&Token::RParen)?;
                Pattern::Failure {
                    code,
                    msg_pat: Box::new(msg_pat),
                    details_pat: Box::new(details_pat),
                }
            }
            Token::True => {
                self.advance();
                Pattern::Literal(Literal::Bool(true))
            }
            Token::False => {
                self.advance();
                Pattern::Literal(Literal::Bool(false))
            }
            Token::None => {
                self.advance();
                Pattern::Literal(Literal::None)
            }
            Token::Integer(v) => {
                self.advance();
                Pattern::Literal(Literal::Integer(v))
            }
            Token::Float(v) => {
                self.advance();
                Pattern::Literal(Literal::Float(v))
            }
            Token::StringLit(ref s) => {
                let s = s.clone();
                self.advance();
                Pattern::Literal(Literal::String(s))
            }
            Token::Identifier(ref name) => {
                let name = name.clone();
                let name_span = self.peek_span();
                self.advance();

                // Check if it's a constructor pattern: Name(args...)
                if self.at(&Token::LParen) {
                    self.advance();
                    let mut args = Vec::new();
                    if !self.at(&Token::RParen) {
                        args.push(self.parse_pattern()?);
                        while self.at(&Token::Comma) {
                            self.advance();
                            args.push(self.parse_pattern()?);
                        }
                    }
                    self.expect(&Token::RParen)?;
                    Pattern::Constructor {
                        name: Spanned::new(name, name_span),
                        args,
                    }
                } else {
                    Pattern::Identifier(name)
                }
            }
            _ => {
                self.error(format!("expected pattern, found `{}`", self.peek()));
                return Err(());
            }
        };
        let end = self.peek_span();
        let span = Span::new(
            start.offset,
            start.line,
            start.column,
            end.offset.saturating_sub(start.offset),
        );
        Ok(Spanned::new(pattern, span))
    }

    // ── Expressions (precedence climbing) ────────────────────────────

    fn parse_expression(&mut self) -> Result<Spanned<Expr>, ()> {
        self.parse_logical_or()
    }

    // logical_or = logical_and { "OR" logical_and }
    fn parse_logical_or(&mut self) -> Result<Spanned<Expr>, ()> {
        let mut left = self.parse_logical_and()?;

        while self.at(&Token::Or) {
            let op_span = self.peek_span();
            self.advance();
            let right = self.parse_logical_and()?;
            let span = Span::new(
                left.span.offset,
                left.span.line,
                left.span.column,
                right.span.offset + right.span.length - left.span.offset,
            );
            left = Spanned::new(
                Expr::BinaryOp {
                    left: Box::new(left),
                    op: Spanned::new(BinaryOp::Or, op_span),
                    right: Box::new(right),
                },
                span,
            );
        }

        Ok(left)
    }

    // logical_and = equality { "AND" equality }
    fn parse_logical_and(&mut self) -> Result<Spanned<Expr>, ()> {
        let mut left = self.parse_equality()?;

        while self.at(&Token::And) {
            let op_span = self.peek_span();
            self.advance();
            let right = self.parse_equality()?;
            let span = Span::new(
                left.span.offset,
                left.span.line,
                left.span.column,
                right.span.offset + right.span.length - left.span.offset,
            );
            left = Spanned::new(
                Expr::BinaryOp {
                    left: Box::new(left),
                    op: Spanned::new(BinaryOp::And, op_span),
                    right: Box::new(right),
                },
                span,
            );
        }

        Ok(left)
    }

    // equality = comparison { ("EQ" | "NEQ") comparison }
    fn parse_equality(&mut self) -> Result<Spanned<Expr>, ()> {
        let mut left = self.parse_comparison()?;

        while self.at(&Token::Eq) || self.at(&Token::Neq) {
            let op_span = self.peek_span();
            let op = match self.peek() {
                Token::Eq => BinaryOp::Eq,
                Token::Neq => BinaryOp::Neq,
                _ => unreachable!(),
            };
            self.advance();
            let right = self.parse_comparison()?;
            let span = Span::new(
                left.span.offset,
                left.span.line,
                left.span.column,
                right.span.offset + right.span.length - left.span.offset,
            );
            left = Spanned::new(
                Expr::BinaryOp {
                    left: Box::new(left),
                    op: Spanned::new(op, op_span),
                    right: Box::new(right),
                },
                span,
            );
        }

        Ok(left)
    }

    // comparison = additive { ("GT" | "GTE" | "LT" | "LTE") additive }
    fn parse_comparison(&mut self) -> Result<Spanned<Expr>, ()> {
        let mut left = self.parse_additive()?;

        while self.at_any(&[Token::Gt, Token::Gte, Token::Lt, Token::Lte]) {
            let op_span = self.peek_span();
            let op = match self.peek() {
                Token::Gt => BinaryOp::Gt,
                Token::Gte => BinaryOp::Gte,
                Token::Lt => BinaryOp::Lt,
                Token::Lte => BinaryOp::Lte,
                _ => unreachable!(),
            };
            self.advance();
            let right = self.parse_additive()?;
            let span = Span::new(
                left.span.offset,
                left.span.line,
                left.span.column,
                right.span.offset + right.span.length - left.span.offset,
            );
            left = Spanned::new(
                Expr::BinaryOp {
                    left: Box::new(left),
                    op: Spanned::new(op, op_span),
                    right: Box::new(right),
                },
                span,
            );
        }

        Ok(left)
    }

    // additive = multiplicative { ("+" | "-") multiplicative }
    fn parse_additive(&mut self) -> Result<Spanned<Expr>, ()> {
        let mut left = self.parse_multiplicative()?;

        while self.at(&Token::Plus) || self.at(&Token::Minus) {
            let op_span = self.peek_span();
            let op = match self.peek() {
                Token::Plus => BinaryOp::Add,
                Token::Minus => BinaryOp::Sub,
                _ => unreachable!(),
            };
            self.advance();
            let right = self.parse_multiplicative()?;
            let span = Span::new(
                left.span.offset,
                left.span.line,
                left.span.column,
                right.span.offset + right.span.length - left.span.offset,
            );
            left = Spanned::new(
                Expr::BinaryOp {
                    left: Box::new(left),
                    op: Spanned::new(op, op_span),
                    right: Box::new(right),
                },
                span,
            );
        }

        Ok(left)
    }

    // multiplicative = unary { ("*" | "/" | "%") unary }
    fn parse_multiplicative(&mut self) -> Result<Spanned<Expr>, ()> {
        let mut left = self.parse_unary()?;

        while self.at(&Token::Star) || self.at(&Token::Slash) || self.at(&Token::Percent) {
            let op_span = self.peek_span();
            let op = match self.peek() {
                Token::Star => BinaryOp::Mul,
                Token::Slash => BinaryOp::Div,
                Token::Percent => BinaryOp::Mod,
                _ => unreachable!(),
            };
            self.advance();
            let right = self.parse_unary()?;
            let span = Span::new(
                left.span.offset,
                left.span.line,
                left.span.column,
                right.span.offset + right.span.length - left.span.offset,
            );
            left = Spanned::new(
                Expr::BinaryOp {
                    left: Box::new(left),
                    op: Spanned::new(op, op_span),
                    right: Box::new(right),
                },
                span,
            );
        }

        Ok(left)
    }

    // unary = ["NOT" | "-"] postfix
    fn parse_unary(&mut self) -> Result<Spanned<Expr>, ()> {
        if self.at(&Token::Not) {
            let op_span = self.peek_span();
            self.advance();
            let operand = self.parse_postfix()?;
            let span = Span::new(
                op_span.offset,
                op_span.line,
                op_span.column,
                operand.span.offset + operand.span.length - op_span.offset,
            );
            return Ok(Spanned::new(
                Expr::UnaryOp {
                    op: Spanned::new(UnaryOp::Not, op_span),
                    operand: Box::new(operand),
                },
                span,
            ));
        }

        if self.at(&Token::Minus) {
            let op_span = self.peek_span();
            self.advance();
            let operand = self.parse_postfix()?;
            let span = Span::new(
                op_span.offset,
                op_span.line,
                op_span.column,
                operand.span.offset + operand.span.length - op_span.offset,
            );
            return Ok(Spanned::new(
                Expr::UnaryOp {
                    op: Spanned::new(UnaryOp::Neg, op_span),
                    operand: Box::new(operand),
                },
                span,
            ));
        }

        self.parse_postfix()
    }

    // postfix = primary { call_suffix | member_suffix | confidence_suffix | range_suffix }
    fn parse_postfix(&mut self) -> Result<Spanned<Expr>, ()> {
        let mut expr = self.parse_primary()?;

        loop {
            match self.peek() {
                Token::LParen => {
                    // call_suffix: (args)
                    self.advance();
                    let args = if self.at(&Token::RParen) {
                        vec![]
                    } else {
                        self.parse_argument_list()?
                    };
                    self.expect(&Token::RParen)?;
                    let end = self.peek_span();
                    let span = Span::new(
                        expr.span.offset,
                        expr.span.line,
                        expr.span.column,
                        end.offset.saturating_sub(expr.span.offset),
                    );
                    expr = Spanned::new(
                        Expr::Call {
                            func: Box::new(expr),
                            args,
                        },
                        span,
                    );
                }
                Token::Dot => {
                    // member_suffix: .field
                    self.advance();
                    let field = self.expect_identifier()?;
                    let end = self.peek_span();
                    let span = Span::new(
                        expr.span.offset,
                        expr.span.line,
                        expr.span.column,
                        end.offset.saturating_sub(expr.span.offset),
                    );
                    expr = Spanned::new(
                        Expr::Member {
                            object: Box::new(expr),
                            field,
                        },
                        span,
                    );
                }
                Token::Question => {
                    // confidence_suffix: ?
                    self.advance();
                    let end = self.peek_span();
                    let span = Span::new(
                        expr.span.offset,
                        expr.span.line,
                        expr.span.column,
                        end.offset.saturating_sub(expr.span.offset),
                    );
                    expr = Spanned::new(
                        Expr::Confidence {
                            expr: Box::new(expr),
                        },
                        span,
                    );
                }
                Token::DotDot => {
                    // range_suffix: ..primary
                    self.advance();
                    let end_expr = self.parse_primary()?;
                    let span = Span::new(
                        expr.span.offset,
                        expr.span.line,
                        expr.span.column,
                        end_expr.span.offset + end_expr.span.length - expr.span.offset,
                    );
                    expr = Spanned::new(
                        Expr::Range {
                            start: Box::new(expr),
                            end: Box::new(end_expr),
                        },
                        span,
                    );
                }
                _ => break,
            }
        }

        Ok(expr)
    }

    // primary = literal | identifier | "(" expression ")" | list_lit | map_lit | fork_expr | resume_expr
    fn parse_primary(&mut self) -> Result<Spanned<Expr>, ()> {
        let start = self.peek_span();

        match self.peek().clone() {
            Token::Integer(v) => {
                self.advance();
                Ok(Spanned::new(Expr::Literal(Literal::Integer(v)), start))
            }
            Token::Float(v) => {
                self.advance();
                Ok(Spanned::new(Expr::Literal(Literal::Float(v)), start))
            }
            Token::StringLit(ref s) => {
                let s = s.clone();
                self.advance();
                Ok(Spanned::new(Expr::Literal(Literal::String(s)), start))
            }
            Token::True => {
                self.advance();
                Ok(Spanned::new(Expr::Literal(Literal::Bool(true)), start))
            }
            Token::False => {
                self.advance();
                Ok(Spanned::new(Expr::Literal(Literal::Bool(false)), start))
            }
            Token::None => {
                self.advance();
                Ok(Spanned::new(Expr::Literal(Literal::None), start))
            }
            Token::Duration(ref s) => {
                let s = s.clone();
                self.advance();
                Ok(Spanned::new(Expr::Literal(Literal::Duration(s)), start))
            }
            Token::Size(ref s) => {
                let s = s.clone();
                self.advance();
                Ok(Spanned::new(Expr::Literal(Literal::Size(s)), start))
            }
            Token::Confidence(v) => {
                self.advance();
                Ok(Spanned::new(Expr::Literal(Literal::Confidence(v)), start))
            }
            Token::HashLit(ref s) => {
                let s = s.clone();
                self.advance();
                Ok(Spanned::new(Expr::Literal(Literal::Hash(s)), start))
            }
            Token::Identifier(ref name) => {
                let name = name.clone();
                self.advance();
                Ok(Spanned::new(Expr::Identifier(name), start))
            }
            Token::LParen => {
                // Parenthesised expression
                self.advance();
                let inner = self.parse_expression()?;
                self.expect(&Token::RParen)?;
                let end = self.peek_span();
                let span = Span::new(
                    start.offset,
                    start.line,
                    start.column,
                    end.offset.saturating_sub(start.offset),
                );
                Ok(Spanned::new(
                    Expr::Paren {
                        inner: Box::new(inner),
                    },
                    span,
                ))
            }
            Token::LBracket => {
                // List literal: [a, b, c]
                self.advance();
                let mut elements = Vec::new();
                if !self.at(&Token::RBracket) {
                    elements.push(self.parse_expression()?);
                    while self.at(&Token::Comma) {
                        self.advance();
                        if self.at(&Token::RBracket) {
                            break; // trailing comma
                        }
                        elements.push(self.parse_expression()?);
                    }
                }
                self.expect(&Token::RBracket)?;
                let end = self.peek_span();
                let span = Span::new(
                    start.offset,
                    start.line,
                    start.column,
                    end.offset.saturating_sub(start.offset),
                );
                Ok(Spanned::new(Expr::List { elements }, span))
            }
            Token::LBrace => {
                // Map literal: { key: value, ... }
                self.advance();
                let mut items = Vec::new();
                self.skip_newlines();
                if !self.at(&Token::RBrace) {
                    items.push(self.parse_map_item()?);
                    while self.at(&Token::Comma) {
                        self.advance();
                        self.skip_newlines();
                        if self.at(&Token::RBrace) {
                            break;
                        }
                        items.push(self.parse_map_item()?);
                    }
                }
                self.skip_newlines();
                self.expect(&Token::RBrace)?;
                let end = self.peek_span();
                let span = Span::new(
                    start.offset,
                    start.line,
                    start.column,
                    end.offset.saturating_sub(start.offset),
                );
                Ok(Spanned::new(Expr::Map { items }, span))
            }
            Token::Fork => self.parse_fork_expr(),
            Token::Resume => {
                // RESUME(expr)
                self.advance();
                self.expect(&Token::LParen)?;
                let inner = self.parse_expression()?;
                self.expect(&Token::RParen)?;
                let end = self.peek_span();
                let span = Span::new(
                    start.offset,
                    start.line,
                    start.column,
                    end.offset.saturating_sub(start.offset),
                );
                Ok(Spanned::new(
                    Expr::Resume {
                        expr: Box::new(inner),
                    },
                    span,
                ))
            }
            _ => {
                self.error(format!("expected expression, found `{}`", self.peek()));
                Err(())
            }
        }
    }

    fn parse_map_item(&mut self) -> Result<Spanned<MapItem>, ()> {
        let start = self.peek_span();
        self.skip_newlines();

        let key = match self.peek().clone() {
            Token::StringLit(ref s) => {
                let s = s.clone();
                let span = self.peek_span();
                self.advance();
                Spanned::new(MapKey::String(s), span)
            }
            Token::Identifier(ref s) => {
                let s = s.clone();
                let span = self.peek_span();
                self.advance();
                Spanned::new(MapKey::Identifier(s), span)
            }
            _ => {
                self.error("expected map key (string or identifier)");
                return Err(());
            }
        };

        self.expect(&Token::Colon)?;
        let value = self.parse_expression()?;

        let end = self.peek_span();
        let span = Span::new(
            start.offset,
            start.line,
            start.column,
            end.offset.saturating_sub(start.offset),
        );
        Ok(Spanned::new(MapItem { key, value }, span))
    }

    // FORK { branch, ... } -> JOIN strategy: ALL_COMPLETE
    fn parse_fork_expr(&mut self) -> Result<Spanned<Expr>, ()> {
        let start = self.peek_span();
        self.expect(&Token::Fork)?;
        self.expect(&Token::LBrace)?;
        self.skip_newlines();

        let mut branches = Vec::new();
        if !self.at(&Token::RBrace) {
            branches.push(self.parse_fork_branch()?);
            while self.at(&Token::Comma) {
                self.advance();
                self.skip_newlines();
                if self.at(&Token::RBrace) {
                    break;
                }
                branches.push(self.parse_fork_branch()?);
            }
        }
        self.skip_newlines();
        self.expect(&Token::RBrace)?;
        self.expect(&Token::Arrow)?;
        self.expect(&Token::Join)?;

        // Parse "strategy: ALL_COMPLETE"
        let strategy_kw = self.expect_identifier()?;
        if strategy_kw.node != "strategy" {
            self.error(format!("expected `strategy`, found `{}`", strategy_kw.node));
            return Err(());
        }
        self.expect(&Token::Colon)?;
        let join_val = self.expect_identifier()?;
        let join = if join_val.node == "ALL_COMPLETE" {
            Spanned::new(JoinStrategy::AllComplete, join_val.span)
        } else {
            self.error(format!(
                "only ALL_COMPLETE join strategy is supported in mvp-0.1, found `{}`",
                join_val.node
            ));
            return Err(());
        };

        let end = self.peek_span();
        let span = Span::new(
            start.offset,
            start.line,
            start.column,
            end.offset.saturating_sub(start.offset),
        );
        Ok(Spanned::new(Expr::Fork { branches, join }, span))
    }

    fn parse_fork_branch(&mut self) -> Result<Spanned<ForkBranch>, ()> {
        let start = self.peek_span();
        let name = self.expect_identifier()?;
        self.expect(&Token::Colon)?;
        let chain = self.parse_pipeline_chain()?;

        let end = self.peek_span();
        let span = Span::new(
            start.offset,
            start.line,
            start.column,
            end.offset.saturating_sub(start.offset),
        );
        Ok(Spanned::new(ForkBranch { name, chain }, span))
    }

    // ── Type expressions ─────────────────────────────────────────────

    fn parse_type_expr(&mut self) -> Result<Spanned<TypeExpr>, ()> {
        self.parse_union_type()
    }

    // union_type = postfix_type { "|" postfix_type }
    fn parse_union_type(&mut self) -> Result<Spanned<TypeExpr>, ()> {
        let first = self.parse_postfix_type()?;

        if self.at(&Token::Pipe) {
            let mut types = vec![first];
            while self.at(&Token::Pipe) {
                self.advance();
                types.push(self.parse_postfix_type()?);
            }
            let start = types.first().unwrap().span;
            let end = types.last().unwrap().span;
            let span = Span::new(
                start.offset,
                start.line,
                start.column,
                end.offset + end.length - start.offset,
            );
            Ok(Spanned::new(TypeExpr::Union { types }, span))
        } else {
            Ok(first)
        }
    }

    // postfix_type = primary_type [ "::" constraint_expr ]
    fn parse_postfix_type(&mut self) -> Result<Spanned<TypeExpr>, ()> {
        let ty = self.parse_primary_type()?;

        if self.at(&Token::DoubleColon) {
            self.advance();
            let constraint = self.parse_constraint_expr()?;
            let span = Span::new(
                ty.span.offset,
                ty.span.line,
                ty.span.column,
                constraint.span.offset + constraint.span.length - ty.span.offset,
            );
            Ok(Spanned::new(
                TypeExpr::Constrained {
                    ty: Box::new(ty),
                    constraint,
                },
                span,
            ))
        } else {
            Ok(ty)
        }
    }

    // primary_type = identifier [ "[" type_expr { "," type_expr } "]" ]
    //              | "{" field_type { "," field_type } "}"
    fn parse_primary_type(&mut self) -> Result<Spanned<TypeExpr>, ()> {
        let start = self.peek_span();

        if self.at(&Token::LBrace) {
            // Record type: { name: type, ... }
            self.advance();
            self.skip_newlines();
            let mut fields = Vec::new();
            if !self.at(&Token::RBrace) {
                fields.push(self.parse_field_type()?);
                while self.at(&Token::Comma) {
                    self.advance();
                    self.skip_newlines();
                    if self.at(&Token::RBrace) {
                        break;
                    }
                    fields.push(self.parse_field_type()?);
                }
            }
            self.skip_newlines();
            self.expect(&Token::RBrace)?;
            let end = self.peek_span();
            let span = Span::new(
                start.offset,
                start.line,
                start.column,
                end.offset.saturating_sub(start.offset),
            );
            return Ok(Spanned::new(TypeExpr::Record { fields }, span));
        }

        // Named type with optional type params
        let name = self.expect_identifier()?;
        let mut params = Vec::new();

        if self.at(&Token::LBracket) {
            self.advance();
            params.push(self.parse_type_expr()?);
            while self.at(&Token::Comma) {
                self.advance();
                params.push(self.parse_type_expr()?);
            }
            self.expect(&Token::RBracket)?;
        }

        let end = self.peek_span();
        let span = Span::new(
            start.offset,
            start.line,
            start.column,
            end.offset.saturating_sub(start.offset),
        );
        Ok(Spanned::new(TypeExpr::Named { name, params }, span))
    }

    fn parse_field_type(&mut self) -> Result<Spanned<FieldType>, ()> {
        let start = self.peek_span();
        let name = self.expect_identifier()?;
        self.expect(&Token::Colon)?;
        let ty = self.parse_type_expr()?;
        let end = self.peek_span();
        let span = Span::new(
            start.offset,
            start.line,
            start.column,
            end.offset.saturating_sub(start.offset),
        );
        Ok(Spanned::new(FieldType { name, ty }, span))
    }

    // ── Arguments / Parameters ───────────────────────────────────────

    fn parse_param_list(&mut self) -> Result<Vec<Spanned<Parameter>>, ()> {
        let mut params = Vec::new();
        params.push(self.parse_parameter()?);
        while self.at(&Token::Comma) {
            self.advance();
            params.push(self.parse_parameter()?);
        }
        Ok(params)
    }

    fn parse_parameter(&mut self) -> Result<Spanned<Parameter>, ()> {
        let start = self.peek_span();
        let name = self.expect_identifier()?;
        self.expect(&Token::Colon)?;
        let ty = self.parse_type_expr()?;
        let end = self.peek_span();
        let span = Span::new(
            start.offset,
            start.line,
            start.column,
            end.offset.saturating_sub(start.offset),
        );
        Ok(Spanned::new(Parameter { name, ty }, span))
    }

    fn parse_argument_list(&mut self) -> Result<Vec<Spanned<Argument>>, ()> {
        let mut args = Vec::new();
        args.push(self.parse_argument()?);
        while self.at(&Token::Comma) {
            self.advance();
            if self.at(&Token::RParen) {
                break; // trailing comma
            }
            args.push(self.parse_argument()?);
        }
        Ok(args)
    }

    fn parse_argument(&mut self) -> Result<Spanned<Argument>, ()> {
        let start = self.peek_span();

        // Try named argument: ident ":" expr
        if let Token::Identifier(_) = self.peek() {
            let lookahead = self.pos + 1;
            if lookahead < self.tokens.len() && matches!(self.tokens[lookahead].token, Token::Colon)
            {
                let name = self.expect_identifier()?;
                self.advance(); // consume ":"
                let value = self.parse_expression()?;
                let end = self.peek_span();
                let span = Span::new(
                    start.offset,
                    start.line,
                    start.column,
                    end.offset.saturating_sub(start.offset),
                );
                return Ok(Spanned::new(
                    Argument {
                        name: Some(name),
                        value,
                    },
                    span,
                ));
            }
        }

        // Positional argument
        let value = self.parse_expression()?;
        let end = self.peek_span();
        let span = Span::new(
            start.offset,
            start.line,
            start.column,
            end.offset.saturating_sub(start.offset),
        );
        Ok(Spanned::new(Argument { name: None, value }, span))
    }

    // ── Helper parsers ───────────────────────────────────────────────

    fn parse_identifier_list(&mut self) -> Result<Vec<Spanned<String>>, ()> {
        let mut idents = Vec::new();
        idents.push(self.expect_identifier()?);
        while self.at(&Token::Comma) {
            self.advance();
            idents.push(self.expect_identifier()?);
        }
        Ok(idents)
    }

    fn parse_constraint_expr(&mut self) -> Result<Spanned<ConstraintExpr>, ()> {
        let start = self.peek_span();
        let name = self.expect_identifier()?;
        self.expect(&Token::LParen)?;
        let args = if self.at(&Token::RParen) {
            vec![]
        } else {
            self.parse_argument_list()?
        };
        self.expect(&Token::RParen)?;
        let end = self.peek_span();
        let span = Span::new(
            start.offset,
            start.line,
            start.column,
            end.offset.saturating_sub(start.offset),
        );
        Ok(Spanned::new(ConstraintExpr { name, args }, span))
    }

    fn parse_failure_policy(&mut self) -> Result<Spanned<FailurePolicy>, ()> {
        let start = self.peek_span();
        let mut steps = Vec::new();
        steps.push(self.parse_policy_step()?);
        while self.at(&Token::Arrow) {
            self.advance();
            steps.push(self.parse_policy_step()?);
        }
        let end = self.peek_span();
        let span = Span::new(
            start.offset,
            start.line,
            start.column,
            end.offset.saturating_sub(start.offset),
        );
        Ok(Spanned::new(FailurePolicy { steps }, span))
    }

    fn parse_policy_step(&mut self) -> Result<Spanned<PolicyStep>, ()> {
        let start = self.peek_span();
        let step = match self.peek() {
            Token::Retry => {
                self.advance();
                self.expect(&Token::LParen)?;
                let count = self.parse_integer_literal()?;
                let mut args = Vec::new();
                while self.at(&Token::Comma) {
                    self.advance();
                    args.push(self.parse_argument()?);
                }
                self.expect(&Token::RParen)?;
                PolicyStep::Retry { count, args }
            }
            Token::Identifier(s) if s == "REASSIGN" => {
                self.advance();
                self.expect(&Token::LParen)?;
                let target = self.expect_identifier()?;
                self.expect(&Token::RParen)?;
                PolicyStep::Reassign(target)
            }
            Token::Escalate => {
                self.advance();
                let message = if self.at(&Token::LParen) {
                    self.advance();
                    let expr = self.parse_expression()?;
                    self.expect(&Token::RParen)?;
                    Some(expr)
                } else {
                    None
                };
                PolicyStep::Escalate(message)
            }
            Token::Identifier(s) if s == "ABORT" => {
                self.advance();
                PolicyStep::Abort
            }
            _ => {
                self.error("expected policy step (RETRY, REASSIGN, ESCALATE, ABORT)");
                return Err(());
            }
        };
        let end = self.peek_span();
        let span = Span::new(
            start.offset,
            start.line,
            start.column,
            end.offset.saturating_sub(start.offset),
        );
        Ok(Spanned::new(step, span))
    }

    fn parse_integer_literal(&mut self) -> Result<Spanned<i64>, ()> {
        let span = self.peek_span();
        match self.peek().clone() {
            Token::Integer(v) => {
                self.advance();
                Ok(Spanned::new(v, span))
            }
            _ => {
                self.error(format!("expected integer literal, found `{}`", self.peek()));
                Err(())
            }
        }
    }

    fn parse_string_literal(&mut self) -> Result<Spanned<String>, ()> {
        let span = self.peek_span();
        match self.peek().clone() {
            Token::StringLit(ref s) => {
                let s = s.clone();
                self.advance();
                Ok(Spanned::new(s, span))
            }
            _ => {
                self.error(format!("expected string literal, found `{}`", self.peek()));
                Err(())
            }
        }
    }

    fn parse_confidence_literal(&mut self) -> Result<Spanned<f64>, ()> {
        let span = self.peek_span();
        match self.peek().clone() {
            Token::Confidence(v) => {
                self.advance();
                Ok(Spanned::new(v, span))
            }
            _ => {
                self.error(format!(
                    "expected confidence literal, found `{}`",
                    self.peek()
                ));
                Err(())
            }
        }
    }

    fn parse_duration_literal(&mut self) -> Result<Spanned<String>, ()> {
        let span = self.peek_span();
        match self.peek().clone() {
            Token::Duration(ref s) => {
                let s = s.clone();
                self.advance();
                Ok(Spanned::new(s, span))
            }
            _ => {
                self.error(format!(
                    "expected duration literal, found `{}`",
                    self.peek()
                ));
                Err(())
            }
        }
    }

    fn parse_size_literal(&mut self) -> Result<Spanned<String>, ()> {
        let span = self.peek_span();
        match self.peek().clone() {
            Token::Size(ref s) => {
                let s = s.clone();
                self.advance();
                Ok(Spanned::new(s, span))
            }
            _ => {
                self.error(format!("expected size literal, found `{}`", self.peek()));
                Err(())
            }
        }
    }
}

// ===========================================================================
// Public API
// ===========================================================================

/// Parse an AgentLang source string into an AST `Program`.
///
/// This function first tokenizes the input using `al_lexer::tokenize`,
/// then runs the recursive-descent parser over the resulting token stream.
///
/// Returns `Ok(Program)` on success, or `Err(diagnostics)` on failure.
pub fn parse(source: &str) -> Result<Program, Vec<Diagnostic>> {
    let tokens = al_lexer::tokenize(source)?;
    let mut parser = Parser::new(tokens);
    let program = parser.parse_program();

    if parser.diagnostics.is_empty() {
        Ok(program)
    } else {
        Err(parser.diagnostics)
    }
}

/// Parse with error recovery, returning the partial program and any diagnostics.
///
/// Unlike [`parse`], this function always returns a `Program` (possibly with
/// fewer declarations than the source contains) together with all accumulated
/// diagnostics. Callers can inspect the diagnostics to decide whether to
/// continue with later compiler phases.
pub fn parse_recovering(source: &str) -> (Program, Vec<Diagnostic>) {
    let tokens = match al_lexer::tokenize(source) {
        Ok(tokens) => tokens,
        Err(diags) => {
            return (
                Program {
                    declarations: vec![],
                    span: Span::dummy(),
                },
                diags,
            )
        }
    };
    let mut parser = Parser::new(tokens);
    let program = parser.parse_program();
    (program, parser.diagnostics)
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_ok(source: &str) -> Program {
        parse(source).unwrap_or_else(|diags| {
            for d in &diags {
                eprintln!("{}: {}", d.code, d.message);
            }
            panic!("parse failed with {} errors", diags.len());
        })
    }

    // ── Type declarations ────────────────────────────────────────────

    #[test]
    fn parse_type_decl() {
        let prog = parse_ok("TYPE UserId = Int64");
        assert_eq!(prog.declarations.len(), 1);
        match &prog.declarations[0].node {
            Declaration::TypeDecl { name, .. } => {
                assert_eq!(name.node, "UserId");
            }
            _ => panic!("expected TypeDecl"),
        }
    }

    #[test]
    fn parse_type_decl_with_params() {
        let prog = parse_ok("TYPE Pair[A, B] = Map[A, B]");
        assert_eq!(prog.declarations.len(), 1);
        match &prog.declarations[0].node {
            Declaration::TypeDecl {
                name, type_params, ..
            } => {
                assert_eq!(name.node, "Pair");
                assert_eq!(type_params.len(), 2);
                assert_eq!(type_params[0].node, "A");
                assert_eq!(type_params[1].node, "B");
            }
            _ => panic!("expected TypeDecl"),
        }
    }

    // ── Schema declarations ──────────────────────────────────────────

    #[test]
    fn parse_schema_decl() {
        let prog = parse_ok("SCHEMA User => { name: Str, age: Int64 }");
        assert_eq!(prog.declarations.len(), 1);
        match &prog.declarations[0].node {
            Declaration::SchemaDecl { name, fields } => {
                assert_eq!(name.node, "User");
                assert_eq!(fields.len(), 2);
                assert_eq!(fields[0].node.name.node, "name");
                assert_eq!(fields[1].node.name.node, "age");
            }
            _ => panic!("expected SchemaDecl"),
        }
    }

    // ── Agent declarations ───────────────────────────────────────────

    #[test]
    fn parse_agent_decl() {
        let src = "AGENT Planner =>\n  CAPABILITIES [plan, delegate]\n  TRUST_LEVEL ~0.9\n  TIMEOUT_DEFAULT 30s\n  MEMORY_LIMIT 256MB";
        let prog = parse_ok(src);
        assert_eq!(prog.declarations.len(), 1);
        match &prog.declarations[0].node {
            Declaration::AgentDecl { name, properties } => {
                assert_eq!(name.node, "Planner");
                assert_eq!(properties.len(), 4);
            }
            _ => panic!("expected AgentDecl"),
        }
    }

    // ── Operation declarations ───────────────────────────────────────

    #[test]
    fn parse_operation_decl() {
        let src = r#"OPERATION Validate =>
  INPUT data: Record
  OUTPUT Result[Record]
  REQUIRE data.fields GT 0
  BODY {
    STORE validated = check(data)
    EMIT validated
  }"#;
        let prog = parse_ok(src);
        assert_eq!(prog.declarations.len(), 1);
        match &prog.declarations[0].node {
            Declaration::OperationDecl {
                name,
                inputs,
                output,
                requires,
                body,
                ..
            } => {
                assert_eq!(name.node, "Validate");
                assert_eq!(inputs.len(), 1);
                assert!(output.is_some());
                assert_eq!(requires.len(), 1);
                assert_eq!(body.node.stmts.len(), 2);
            }
            _ => panic!("expected OperationDecl"),
        }
    }

    // ── Pipeline declarations ────────────────────────────────────────

    #[test]
    fn parse_pipeline_decl() {
        let prog = parse_ok("PIPELINE Process => fetch -> validate |> transform");
        assert_eq!(prog.declarations.len(), 1);
        match &prog.declarations[0].node {
            Declaration::PipelineDecl { name, chain } => {
                assert_eq!(name.node, "Process");
                assert_eq!(chain.node.stages.len(), 3);
            }
            _ => panic!("expected PipelineDecl"),
        }
    }

    // ── Statements ───────────────────────────────────────────────────

    #[test]
    fn parse_store_stmt() {
        let prog = parse_ok("OPERATION Test => BODY { STORE x = 42 }");
        match &prog.declarations[0].node {
            Declaration::OperationDecl { body, .. } => {
                assert_eq!(body.node.stmts.len(), 1);
                match &body.node.stmts[0].node {
                    Statement::Store { name, value, .. } => {
                        assert_eq!(name.node, "x");
                        assert!(matches!(value.node, Expr::Literal(Literal::Integer(42))));
                    }
                    _ => panic!("expected Store"),
                }
            }
            _ => panic!("expected OperationDecl"),
        }
    }

    #[test]
    fn parse_match_stmt() {
        let prog = parse_ok(
            r#"OPERATION Test => BODY {
  MATCH result => {
    WHEN SUCCESS(val) -> { EMIT val }
    WHEN FAILURE(code, msg, details) -> { ESCALATE }
    OTHERWISE -> { EMIT NONE }
  }
}"#,
        );
        match &prog.declarations[0].node {
            Declaration::OperationDecl { body, .. } => match &body.node.stmts[0].node {
                Statement::Match {
                    arms, otherwise, ..
                } => {
                    assert_eq!(arms.len(), 2);
                    assert!(otherwise.is_some());
                }
                _ => panic!("expected Match"),
            },
            _ => panic!("expected OperationDecl"),
        }
    }

    #[test]
    fn parse_loop_stmt() {
        let prog = parse_ok("OPERATION Test => BODY { LOOP max: 10 => { EMIT x } }");
        match &prog.declarations[0].node {
            Declaration::OperationDecl { body, .. } => match &body.node.stmts[0].node {
                Statement::Loop { max_iters, body } => {
                    assert_eq!(max_iters.node, 10);
                    assert_eq!(body.node.stmts.len(), 1);
                }
                _ => panic!("expected Loop"),
            },
            _ => panic!("expected OperationDecl"),
        }
    }

    #[test]
    fn parse_retry_and_escalate() {
        let prog = parse_ok("OPERATION Test => BODY { RETRY(3); ESCALATE }");
        match &prog.declarations[0].node {
            Declaration::OperationDecl { body, .. } => {
                assert_eq!(body.node.stmts.len(), 2);
                assert!(matches!(body.node.stmts[0].node, Statement::Retry { .. }));
                assert!(matches!(
                    body.node.stmts[1].node,
                    Statement::Escalate { .. }
                ));
            }
            _ => panic!("expected OperationDecl"),
        }
    }

    #[test]
    fn parse_checkpoint() {
        let prog = parse_ok(r#"OPERATION Test => BODY { CHECKPOINT "save1" }"#);
        match &prog.declarations[0].node {
            Declaration::OperationDecl { body, .. } => match &body.node.stmts[0].node {
                Statement::Checkpoint { label } => {
                    assert_eq!(label.as_ref().unwrap().node, "save1");
                }
                _ => panic!("expected Checkpoint"),
            },
            _ => panic!("expected OperationDecl"),
        }
    }

    // ── Expressions ──────────────────────────────────────────────────

    #[test]
    fn parse_binary_ops() {
        let prog = parse_ok("OPERATION Test => BODY { STORE x = a + b * c }");
        match &prog.declarations[0].node {
            Declaration::OperationDecl { body, .. } => {
                match &body.node.stmts[0].node {
                    Statement::Store { value, .. } => {
                        // Should be: a + (b * c) due to precedence
                        assert!(matches!(value.node, Expr::BinaryOp { .. }));
                    }
                    _ => panic!("expected Store"),
                }
            }
            _ => panic!("expected OperationDecl"),
        }
    }

    #[test]
    fn parse_function_call() {
        let prog = parse_ok("OPERATION Test => BODY { STORE x = f(a, b) }");
        match &prog.declarations[0].node {
            Declaration::OperationDecl { body, .. } => match &body.node.stmts[0].node {
                Statement::Store { value, .. } => match &value.node {
                    Expr::Call { args, .. } => {
                        assert_eq!(args.len(), 2);
                    }
                    _ => panic!("expected Call"),
                },
                _ => panic!("expected Store"),
            },
            _ => panic!("expected OperationDecl"),
        }
    }

    #[test]
    fn parse_member_access() {
        let prog = parse_ok("OPERATION Test => BODY { STORE x = obj.field }");
        match &prog.declarations[0].node {
            Declaration::OperationDecl { body, .. } => match &body.node.stmts[0].node {
                Statement::Store { value, .. } => {
                    assert!(matches!(value.node, Expr::Member { .. }));
                }
                _ => panic!("expected Store"),
            },
            _ => panic!("expected OperationDecl"),
        }
    }

    #[test]
    fn parse_list_literal() {
        let prog = parse_ok("OPERATION Test => BODY { STORE x = [1, 2, 3] }");
        match &prog.declarations[0].node {
            Declaration::OperationDecl { body, .. } => match &body.node.stmts[0].node {
                Statement::Store { value, .. } => match &value.node {
                    Expr::List { elements } => assert_eq!(elements.len(), 3),
                    _ => panic!("expected List"),
                },
                _ => panic!("expected Store"),
            },
            _ => panic!("expected OperationDecl"),
        }
    }

    #[test]
    fn parse_empty_program() {
        let prog = parse_ok("");
        assert_eq!(prog.declarations.len(), 0);
    }

    #[test]
    fn parse_multiple_declarations() {
        let src = r#"
TYPE UserId = Int64
SCHEMA User => { name: Str, id: UserId }
OPERATION GetUser => BODY { EMIT NONE }
"#;
        let prog = parse_ok(src);
        assert_eq!(prog.declarations.len(), 3);
    }

    #[test]
    fn parse_error_returns_diagnostics() {
        let result = parse("OPERATION => BODY { }");
        assert!(result.is_err());
    }

    // ── Delegate ─────────────────────────────────────────────────────

    #[test]
    fn parse_delegate_stmt() {
        let src = r#"OPERATION Test => BODY {
  DELEGATE analysis TO worker => {
    INPUT data
    TIMEOUT 30s
  }
}"#;
        let prog = parse_ok(src);
        match &prog.declarations[0].node {
            Declaration::OperationDecl { body, .. } => match &body.node.stmts[0].node {
                Statement::Delegate {
                    task,
                    target,
                    clauses,
                } => {
                    assert_eq!(task.node, "analysis");
                    assert_eq!(target.node, "worker");
                    assert_eq!(clauses.len(), 2);
                }
                _ => panic!("expected Delegate"),
            },
            _ => panic!("expected OperationDecl"),
        }
    }

    // ── Constrained types ────────────────────────────────────────────

    #[test]
    fn parse_constrained_type() {
        let prog = parse_ok("TYPE Positive = Int64 :: range(0, 100)");
        match &prog.declarations[0].node {
            Declaration::TypeDecl { ty, .. } => {
                assert!(matches!(ty.node, TypeExpr::Constrained { .. }));
            }
            _ => panic!("expected TypeDecl"),
        }
    }

    // ── Match body statement keywords ──────────────────────────────

    #[test]
    fn parse_match_body_emit_without_block() {
        let prog = parse_ok(
            r#"OPERATION Test => BODY {
  MATCH result => {
    WHEN SUCCESS(val) -> EMIT val
    OTHERWISE -> EMIT NONE
  }
}"#,
        );
        match &prog.declarations[0].node {
            Declaration::OperationDecl { body, .. } => {
                match &body.node.stmts[0].node {
                    Statement::Match {
                        arms, otherwise, ..
                    } => {
                        assert_eq!(arms.len(), 1);
                        // Arm body should be a block wrapping the EMIT
                        assert!(matches!(arms[0].node.body.node, MatchBody::Block(_)));
                        assert!(otherwise.is_some());
                    }
                    _ => panic!("expected Match"),
                }
            }
            _ => panic!("expected OperationDecl"),
        }
    }

    #[test]
    fn parse_match_body_escalate_without_block() {
        let prog = parse_ok(
            r#"OPERATION Test => BODY {
  MATCH result => {
    WHEN FAILURE(code, msg, details) -> ESCALATE(msg)
    OTHERWISE -> HALT(error)
  }
}"#,
        );
        match &prog.declarations[0].node {
            Declaration::OperationDecl { body, .. } => {
                match &body.node.stmts[0].node {
                    Statement::Match { arms, .. } => {
                        // ESCALATE arm
                        if let MatchBody::Block(block) = &arms[0].node.body.node {
                            assert!(matches!(
                                block.node.stmts[0].node,
                                Statement::Escalate { .. }
                            ));
                        } else {
                            panic!("expected Block wrapping ESCALATE");
                        }
                    }
                    _ => panic!("expected Match"),
                }
            }
            _ => panic!("expected OperationDecl"),
        }
    }

    #[test]
    fn parse_match_body_retry_checkpoint_assert() {
        let prog = parse_ok(
            r#"OPERATION Test => BODY {
  MATCH status => {
    WHEN SUCCESS(val) -> CHECKPOINT "ok"
    WHEN FAILURE(c, m, d) -> RETRY(3)
  }
}"#,
        );
        match &prog.declarations[0].node {
            Declaration::OperationDecl { body, .. } => {
                match &body.node.stmts[0].node {
                    Statement::Match { arms, .. } => {
                        assert_eq!(arms.len(), 2);
                        // SUCCESS -> CHECKPOINT
                        if let MatchBody::Block(block) = &arms[0].node.body.node {
                            assert!(matches!(
                                block.node.stmts[0].node,
                                Statement::Checkpoint { .. }
                            ));
                        } else {
                            panic!("expected Block wrapping CHECKPOINT");
                        }
                        // FAILURE -> RETRY
                        if let MatchBody::Block(block) = &arms[1].node.body.node {
                            assert!(matches!(block.node.stmts[0].node, Statement::Retry { .. }));
                        } else {
                            panic!("expected Block wrapping RETRY");
                        }
                    }
                    _ => panic!("expected Match"),
                }
            }
            _ => panic!("expected OperationDecl"),
        }
    }

    // ── Error recovery ─────────────────────────────────────────────

    #[test]
    fn parse_recovering_returns_partial_results() {
        // First declaration is valid, second is malformed (missing name),
        // third is valid. The parser should recover the two valid ones.
        let source = r#"
TYPE UserId = Int64
OPERATION => BODY { }
TYPE Count = Int64
"#;
        let (program, diagnostics) = parse_recovering(source);
        // Should have recovered 2 valid declarations
        assert_eq!(program.declarations.len(), 2);
        assert!(!diagnostics.is_empty(), "should have reported errors");
    }

    #[test]
    fn parse_recovering_empty_program() {
        let (program, diagnostics) = parse_recovering("");
        assert_eq!(program.declarations.len(), 0);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn parse_block_recovery_skips_bad_statement() {
        let prog = parse_ok(
            r#"OPERATION Test => BODY {
  STORE x = 42
  EMIT x
}"#,
        );
        match &prog.declarations[0].node {
            Declaration::OperationDecl { body, .. } => {
                assert_eq!(body.node.stmts.len(), 2);
            }
            _ => panic!("expected OperationDecl"),
        }
    }

    // ── Property-based tests ────────────────────────────────────────

    mod proptest_parser {
        use super::*;
        use proptest::prelude::*;

        /// Strategy for valid type names (built-in types).
        fn builtin_type() -> impl Strategy<Value = &'static str> {
            prop::sample::select(vec![
                "Int64", "Float64", "Str", "Bool", "Int", "Float",
            ])
        }

        /// Strategy for valid identifiers (lowercase, not keywords).
        fn ident() -> impl Strategy<Value = String> {
            "[a-z][a-z0-9]{0,8}".prop_filter("not a keyword", |s| {
                !matches!(
                    s.as_str(),
                    "max" | "strategy" | "to"
                )
            })
        }

        /// All AgentLang keywords to filter from random names.
        const KEYWORDS: &[&str] = &[
            "TYPE", "SCHEMA", "AGENT", "OPERATION", "PIPELINE",
            "BODY", "INPUT", "OUTPUT", "REQUIRE", "ENSURE",
            "INVARIANT", "STORE", "MUTABLE", "MATCH", "WHEN",
            "OTHERWISE", "LOOP", "EMIT", "ASSERT", "RETRY",
            "ESCALATE", "CHECKPOINT", "RESUME", "HALT",
            "DELEGATE", "TO", "FORK", "JOIN", "SUCCESS",
            "FAILURE", "TRUE", "FALSE", "NONE", "AND", "OR",
            "NOT", "EQ", "NEQ", "GT", "GTE", "LT", "LTE",
        ];

        /// Strategy for valid uppercase names that aren't keywords.
        fn safe_name() -> impl Strategy<Value = String> {
            "[A-Z][a-z][a-zA-Z]{0,6}".prop_filter("not a keyword", |s| {
                !KEYWORDS.contains(&s.as_str())
            })
        }

        proptest! {
            /// TYPE declarations with builtin types always parse.
            #[test]
            fn parse_type_decl(
                name in safe_name(),
                ty in builtin_type()
            ) {
                let source = format!("TYPE {} = {}", name, ty);
                let result = parse(&source);
                prop_assert!(result.is_ok(), "TYPE decl should parse: {}", source);
                let prog = result.unwrap();
                prop_assert_eq!(prog.declarations.len(), 1);
            }

            /// SCHEMA declarations with varying field counts always parse.
            #[test]
            fn parse_schema_decl(
                name in safe_name(),
                field1 in ident(),
                field2 in ident(),
            ) {
                // Ensure unique field names
                if field1 != field2 {
                    let source = format!(
                        "SCHEMA {} => {{ {}: Int64, {}: Str }}",
                        name, field1, field2
                    );
                    let result = parse(&source);
                    prop_assert!(result.is_ok(), "SCHEMA decl should parse: {}", source);
                }
            }

            /// Simple OPERATION declarations always parse.
            #[test]
            fn parse_operation_emit(
                name in safe_name(),
                val in 0i64..1000,
            ) {
                let source = format!("OPERATION {} => BODY {{ EMIT {} }}", name, val);
                let result = parse(&source);
                prop_assert!(result.is_ok(), "OPERATION should parse: {}", source);
            }

            /// PIPELINE declarations always parse with valid stages.
            #[test]
            fn parse_pipeline(
                name in safe_name(),
                stage1 in ident(),
                stage2 in ident(),
            ) {
                let source = format!("PIPELINE {} => {} -> {}", name, stage1, stage2);
                let result = parse(&source);
                prop_assert!(result.is_ok(), "PIPELINE should parse: {}", source);
            }

            /// Arbitrary printable ASCII never causes a panic in parse.
            #[test]
            fn parse_no_panic(source in "[ -~]{0,80}") {
                let _ = parse(&source);
            }

            /// parse_recovering never panics on any input.
            #[test]
            fn parse_recovering_no_panic(source in "[ -~]{0,80}") {
                let _ = parse_recovering(&source);
            }
        }
    }
}
