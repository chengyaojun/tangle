use crate::ast::*;
use crate::model::{SourceSpan, TangleDiagnostic};
use crate::parser::lexer::{Token, TokenKind};

// ============================================================
// Pratt precedence table
// ============================================================

fn bp_of(kind: TokenKind) -> u8 {
    match kind {
        TokenKind::Or => 1,
        TokenKind::And => 2,
        TokenKind::EqEq | TokenKind::Neq => 3,
        TokenKind::Lt | TokenKind::Gt | TokenKind::Lte | TokenKind::Gte => 4,
        TokenKind::Plus | TokenKind::Minus => 5,
        TokenKind::Star | TokenKind::Slash | TokenKind::Percent => 6,
        TokenKind::PipeOp => 0,
        _ => 0,
    }
}

// ============================================================
// ParserState
// ============================================================

pub struct ParserState<'a> {
    tokens: &'a [Token],
    pos: usize,
    pub diagnostics: Vec<TangleDiagnostic>,
}

impl<'a> ParserState<'a> {
    pub fn new(tokens: &'a [Token]) -> Self {
        ParserState {
            tokens,
            pos: 0,
            diagnostics: vec![],
        }
    }

    fn peek(&self) -> &Token {
        &self.tokens[self.pos]
    }

    fn advance(&mut self) -> Token {
        let t = self.tokens[self.pos].clone();
        if self.pos + 1 < self.tokens.len() {
            self.pos += 1;
        }
        t
    }

    fn merge_span(&self, start: &SourceSpan, end: &SourceSpan) -> SourceSpan {
        SourceSpan {
            file: start.file.clone(),
            start_line: start.start_line,
            start_column: start.start_column,
            end_line: end.end_line,
            end_column: end.end_column,
        }
    }

    fn span_of_expr(&self, expr: &Expr) -> SourceSpan {
        match expr {
            Expr::Literal(e) => e.span.clone(),
            Expr::Identifier(e) => e.span.clone(),
            Expr::MemberAccess(e) => e.span.clone(),
            Expr::Call(e) => e.span.clone(),
            Expr::Binary(e) => e.span.clone(),
            Expr::Unary(e) => e.span.clone(),
            Expr::RecordUpdate(e) => e.span.clone(),
            Expr::Pipe(e) => e.span.clone(),
            Expr::This(e) => e.span.clone(),
            Expr::If(e) => e.span.clone(),
            Expr::Arrow(e) => e.span.clone(),
            Expr::Propagation(e) => e.span.clone(),
            Expr::Match(e) => e.span.clone(),
            Expr::Destructure(e) => e.span.clone(),
            Expr::Panic(e) => e.span.clone(),
        }
    }

    // ============================================================
    // Expression parsing (Pratt)
    // ============================================================

    pub fn parse_expression(&mut self, min_bp: u8) -> Option<Expr> {
        let token = self.advance();
        let mut lhs = self.parse_prefix(token)?;

        loop {
            let op = self.peek().clone();
            let bp = bp_of(op.kind);
            // Use < so that same-precedence operators (like PipeOp at bp=0)
            // still enter the match for infix parsing at the top level.
            if bp < min_bp {
                break;
            }

            match op.kind {
                TokenKind::Question => {
                    self.advance();
                    let span = self.merge_span(&self.span_of_expr(&lhs), &op.span);
                    lhs = Expr::Propagation(PropagationExpr {
                        expr: Box::new(lhs),
                        span,
                    });
                }
                TokenKind::Dot => {
                    self.advance();
                    let member_tok = self.advance();
                    if member_tok.kind != TokenKind::Identifier {
                        self.diagnostics.push(TangleDiagnostic {
                            code: "TANGLE_PARSE_ERROR".into(),
                            message: "Expected identifier after '.'".into(),
                            span: member_tok.span.clone(),
                        });
                        return None;
                    }
                    let span =
                        self.merge_span(&self.span_of_expr(&lhs), &member_tok.span);
                    lhs = Expr::MemberAccess(MemberAccessExpr {
                        object: Box::new(lhs),
                        member: member_tok.lexeme.clone(),
                        span,
                    });
                }
                TokenKind::LParen => {
                    let args = self.parse_arg_list()?;
                    let span = self.merge_span(
                        &self.span_of_expr(&lhs),
                        &self.tokens[self.pos - 1].span,
                    );
                    lhs = Expr::Call(CallExpr {
                        callee: Box::new(lhs),
                        args,
                        span,
                    });
                }
                TokenKind::LBrace => {
                    let fields = self.parse_record_fields()?;
                    let span = self.merge_span(
                        &self.span_of_expr(&lhs),
                        &self.tokens[self.pos - 1].span,
                    );
                    lhs = Expr::RecordUpdate(RecordUpdateExpr {
                        object: Box::new(lhs),
                        fields,
                        span,
                    });
                }
                TokenKind::PipeOp => {
                    self.advance();
                    let rhs = self.parse_expression(1)?;
                    let span = self.merge_span(
                        &self.span_of_expr(&lhs),
                        &self.span_of_expr(&rhs),
                    );
                    lhs = Expr::Pipe(PipeExpr {
                        left: Box::new(lhs),
                        right: Box::new(rhs),
                        span,
                    });
                }
                _ => {
                    // Binary operators
                    self.advance();
                    let rhs = self.parse_expression(bp + 1)?;
                    let op = binary_op_from_token(op.kind);
                    let span = self.merge_span(
                        &self.span_of_expr(&lhs),
                        &self.span_of_expr(&rhs),
                    );
                    lhs = Expr::Binary(BinaryExpr {
                        op,
                        left: Box::new(lhs),
                        right: Box::new(rhs),
                        span,
                    });
                }
            }
        }
        Some(lhs)
    }

    // ============================================================
    // Prefix parsing (NUD)
    // ============================================================

    fn parse_prefix(&mut self, token: Token) -> Option<Expr> {
        match token.kind {
            TokenKind::Number => Some(Expr::Literal(LiteralExpr {
                literal_kind: LiteralKind::Number,
                value: token.lexeme,
                span: token.span,
            })),
            TokenKind::String => Some(Expr::Literal(LiteralExpr {
                literal_kind: LiteralKind::String,
                value: token.lexeme,
                span: token.span,
            })),
            TokenKind::True | TokenKind::False => Some(Expr::Literal(LiteralExpr {
                literal_kind: LiteralKind::Boolean,
                value: token.lexeme,
                span: token.span,
            })),
            TokenKind::Identifier => {
                // Check for `match` and `panic` by lexeme (they're ErrorKw, not Identifier)
                if token.lexeme == "match" {
                    return self.try_parse_match(token);
                }
                if token.lexeme == "panic" {
                    return self.try_parse_panic(token);
                }
                Some(Expr::Identifier(IdentifierExpr {
                    name: token.lexeme,
                    span: token.span,
                }))
            }
            TokenKind::ErrorKw => {
                // ErrorKw covers both "match" and "panic" (soft keywords)
                if token.lexeme == "match" {
                    return self.try_parse_match(token);
                }
                if token.lexeme == "panic" {
                    return self.try_parse_panic(token);
                }
                // Unknown ErrorKw — treat as identifier
                Some(Expr::Identifier(IdentifierExpr {
                    name: token.lexeme,
                    span: token.span,
                }))
            }
            TokenKind::This => Some(Expr::This(ThisExpr {
                span: token.span,
            })),
            TokenKind::Bang | TokenKind::Minus => {
                let operand = self.parse_expression(7)?;
                let op = if token.kind == TokenKind::Bang {
                    UnaryOp::Not
                } else {
                    UnaryOp::Neg
                };
                let span = self.merge_span(&token.span, &self.span_of_expr(&operand));
                Some(Expr::Unary(UnaryExpr {
                    op,
                    operand: Box::new(operand),
                    span,
                }))
            }
            TokenKind::If => self.parse_if_expr(token),
            TokenKind::LParen => self.parse_paren_or_arrow(),
            _ => {
                self.diagnostics.push(TangleDiagnostic {
                    code: "TANGLE_PARSE_ERROR".into(),
                    message: format!("Unexpected token: {:?}", token.kind),
                    span: token.span.clone(),
                });
                None
            }
        }
    }

    // ============================================================
    // Backtracking parsers
    // ============================================================

    fn try_parse_match(&mut self, match_token: Token) -> Option<Expr> {
        let saved_pos = self.pos;
        let saved_diags_len = self.diagnostics.len();

        // Try: match expr { arms }
        let expr = self.parse_expression(0)?;
        if self.peek().kind != TokenKind::LBrace {
            // Backtrack: it's just an identifier named "match"
            self.pos = saved_pos;
            self.diagnostics.truncate(saved_diags_len);
            return Some(Expr::Identifier(IdentifierExpr {
                name: "match".into(),
                span: match_token.span,
            }));
        }
        let arms = self.parse_match_arms()?;
        let span = self.merge_span(&match_token.span, &self.tokens[self.pos - 1].span);
        Some(Expr::Match(MatchExpr {
            expr: Box::new(expr),
            arms,
            span,
        }))
    }

    fn try_parse_panic(&mut self, panic_token: Token) -> Option<Expr> {
        let saved_pos = self.pos;
        let saved_diags_len = self.diagnostics.len();

        if self.peek().kind != TokenKind::LParen {
            self.pos = saved_pos;
            self.diagnostics.truncate(saved_diags_len);
            return Some(Expr::Identifier(IdentifierExpr {
                name: "panic".into(),
                span: panic_token.span,
            }));
        }
        self.advance(); // consume (
        let msg = self.parse_expression(0)?;
        if self.peek().kind != TokenKind::RParen {
            self.pos = saved_pos;
            self.diagnostics.truncate(saved_diags_len);
            return Some(Expr::Identifier(IdentifierExpr {
                name: "panic".into(),
                span: panic_token.span,
            }));
        }
        let rparen = self.advance();
        let span = self.merge_span(&panic_token.span, &rparen.span);
        Some(Expr::Panic(PanicExpr {
            message: Box::new(msg),
            span,
        }))
    }

    // ============================================================
    // If expression
    // ============================================================

    fn parse_if_expr(&mut self, if_token: Token) -> Option<Expr> {
        let condition = self.parse_expression(0)?;
        let then_branch = self.parse_block_or_expr()?;
        let else_branch = if self.peek().kind == TokenKind::Else {
            self.advance();
            Some(Box::new(self.parse_block_or_expr()?))
        } else {
            None
        };
        let end_span = match &else_branch {
            Some(e) => self.span_of_expr(e),
            None => self.span_of_expr(&then_branch),
        };
        let span = self.merge_span(&if_token.span, &end_span);
        Some(Expr::If(IfExpr {
            condition: Box::new(condition),
            then_branch: Box::new(then_branch),
            else_branch,
            span,
        }))
    }

    // ============================================================
    // Parenthesized expression / arrow function
    // ============================================================

    fn parse_paren_or_arrow(&mut self) -> Option<Expr> {
        // The '(' was already consumed by advance() in parse_expression.
        // self.pos points to the token after '('.
        let lparen = self.tokens[self.pos - 1].clone();

        if self.peek().kind == TokenKind::RParen {
            self.advance(); // consume )
            self.diagnostics.push(TangleDiagnostic {
                code: "TANGLE_PARSE_ERROR".into(),
                message: "Empty parentheses in expression".into(),
                span: lparen.span.clone(),
            });
            return Some(Expr::Literal(LiteralExpr {
                literal_kind: LiteralKind::Boolean,
                value: "false".into(),
                span: lparen.span,
            }));
        }

        let first = self.parse_expression(0)?;

        if self.peek().kind == TokenKind::Comma {
            // Multi-param: (x, y) — could be arrow params or destructure
            let mut params = vec![ArrowParam {
                name: extract_ident_name(&first)?,
                span: self.span_of_expr(&first),
            }];
            while self.peek().kind == TokenKind::Comma {
                self.advance();
                let next = self.parse_expression(0)?;
                params.push(ArrowParam {
                    name: extract_ident_name(&next)?,
                    span: self.span_of_expr(&next),
                });
            }
            if self.peek().kind != TokenKind::RParen {
                self.diagnostics.push(TangleDiagnostic {
                    code: "TANGLE_PARSE_ERROR".into(),
                    message: "Expected ')'".into(),
                    span: self.peek().span.clone(),
                });
                return None;
            }
            self.advance(); // consume )

            if self.peek().kind == TokenKind::FatArrow || self.peek().kind == TokenKind::Arrow
            {
                self.advance(); // consume -> or =>
                let body = self.parse_expression(0)?;
                let span = self.merge_span(&lparen.span, &self.span_of_expr(&body));
                return Some(Expr::Arrow(ArrowExpr {
                    params,
                    body: Box::new(body),
                    span,
                }));
            }
            // Multi-param without arrow — just return the first expr
            // (future: could be a tuple literal)
            return Some(first);
        }

        if self.peek().kind != TokenKind::RParen {
            self.diagnostics.push(TangleDiagnostic {
                code: "TANGLE_PARSE_ERROR".into(),
                message: "Expected ')'".into(),
                span: self.peek().span.clone(),
            });
            return None;
        }
        self.advance(); // consume )

        if self.peek().kind == TokenKind::FatArrow || self.peek().kind == TokenKind::Arrow {
            // Single-param arrow: (x) -> body
            self.advance(); // consume -> or =>
            let body = self.parse_expression(0)?;
            let span = self.merge_span(&lparen.span, &self.span_of_expr(&body));
            let params = vec![ArrowParam {
                name: extract_ident_name(&first)?,
                span: self.span_of_expr(&first),
            }];
            return Some(Expr::Arrow(ArrowExpr {
                params,
                body: Box::new(body),
                span,
            }));
        }

        // Simple parenthesized expression
        Some(first)
    }

    // ============================================================
    // Argument list: ( arg, arg, ... )
    // ============================================================

    fn parse_arg_list(&mut self) -> Option<Vec<Expr>> {
        self.advance(); // consume '('
        let mut args = vec![];
        if self.peek().kind == TokenKind::RParen {
            self.advance();
            return Some(args);
        }
        loop {
            args.push(self.parse_expression(0)?);
            if self.peek().kind == TokenKind::Comma {
                self.advance();
                continue;
            }
            if self.peek().kind == TokenKind::RParen {
                self.advance();
                break;
            }
            self.diagnostics.push(TangleDiagnostic {
                code: "TANGLE_PARSE_ERROR".into(),
                message: "Expected ',' or ')'".into(),
                span: self.peek().span.clone(),
            });
            return None;
        }
        Some(args)
    }

    // ============================================================
    // Record update fields: { field: val, ... }
    // ============================================================

    fn parse_record_fields(&mut self) -> Option<Vec<RecordField>> {
        self.advance(); // consume '{'
        let mut fields = vec![];
        if self.peek().kind == TokenKind::RBrace {
            self.advance();
            return Some(fields);
        }
        loop {
            let name_tok = self.advance();
            if name_tok.kind != TokenKind::Identifier {
                self.diagnostics.push(TangleDiagnostic {
                    code: "TANGLE_PARSE_ERROR".into(),
                    message: "Expected field name".into(),
                    span: name_tok.span.clone(),
                });
                return None;
            }
            if self.peek().kind != TokenKind::Colon {
                self.diagnostics.push(TangleDiagnostic {
                    code: "TANGLE_PARSE_ERROR".into(),
                    message: "Expected ':' after field name".into(),
                    span: self.peek().span.clone(),
                });
                return None;
            }
            self.advance(); // consume ':'
            let value = self.parse_expression(0)?;
            let span = self.merge_span(&name_tok.span, &self.span_of_expr(&value));
            fields.push(RecordField {
                name: name_tok.lexeme.clone(),
                value,
                span,
            });
            if self.peek().kind == TokenKind::Comma {
                self.advance();
                continue;
            }
            if self.peek().kind == TokenKind::RBrace {
                self.advance();
                break;
            }
            self.diagnostics.push(TangleDiagnostic {
                code: "TANGLE_PARSE_ERROR".into(),
                message: "Expected ',' or '}'".into(),
                span: self.peek().span.clone(),
            });
            return None;
        }
        Some(fields)
    }

    // ============================================================
    // Match arms: { pattern => body, ... }
    // ============================================================

    fn parse_match_arms(&mut self) -> Option<Vec<MatchArm>> {
        self.advance(); // consume '{'
        let mut arms = vec![];
        loop {
            if self.peek().kind == TokenKind::RBrace {
                self.advance();
                break;
            }
            let pattern = self.parse_match_pattern()?;
            if self.peek().kind != TokenKind::FatArrow {
                self.diagnostics.push(TangleDiagnostic {
                    code: "TANGLE_PARSE_ERROR".into(),
                    message: "Expected '=>' in match arm".into(),
                    span: self.peek().span.clone(),
                });
                return None;
            }
            self.advance(); // consume '=>'
            let body = self.parse_expression(0)?;
            let span =
                self.merge_span(&self.tokens[self.pos - 1].span, &self.span_of_expr(&body));
            arms.push(MatchArm {
                pattern,
                body,
                span,
            });
            if self.peek().kind == TokenKind::Comma {
                self.advance();
            }
        }
        Some(arms)
    }

    fn parse_match_pattern(&mut self) -> Option<MatchPattern> {
        let tok = self.peek();
        // Wildcard pattern: _ (lexer emits Identifier for "_")
        if tok.kind == TokenKind::Identifier && tok.lexeme == "_" {
            self.advance();
            return Some(MatchPattern::Wildcard);
        }
        let name_tok = self.advance();
        if name_tok.kind != TokenKind::Identifier {
            self.diagnostics.push(TangleDiagnostic {
                code: "TANGLE_PARSE_ERROR".into(),
                message: "Expected pattern (variant name or wildcard '_')".into(),
                span: name_tok.span.clone(),
            });
            return None;
        }
        let binding = if self.peek().kind == TokenKind::LParen {
            self.advance(); // consume '('
            let bind = if self.peek().kind == TokenKind::Identifier {
                Some(self.advance().lexeme.clone())
            } else {
                None
            };
            if self.peek().kind == TokenKind::RParen {
                self.advance(); // consume ')'
            }
            bind
        } else {
            None
        };
        Some(MatchPattern::Variant {
            name: name_tok.lexeme.clone(),
            binding,
        })
    }

    // ============================================================
    // Block or expression (for if branches)
    // ============================================================

    fn parse_block_or_expr(&mut self) -> Option<Expr> {
        if self.peek().kind == TokenKind::LBrace {
            self.advance(); // consume '{'
            let mut stmts = vec![];
            loop {
                if self.peek().kind == TokenKind::RBrace {
                    self.advance();
                    break;
                }
                if let Some(stmt) = self.parse_statement() {
                    stmts.push(stmt);
                } else {
                    // Error recovery: skip to next semicolon or RBrace
                    while self.peek().kind != TokenKind::Semicolon
                        && self.peek().kind != TokenKind::RBrace
                        && self.peek().kind != TokenKind::Eof
                    {
                        self.advance();
                    }
                    if self.peek().kind == TokenKind::Semicolon {
                        self.advance();
                    }
                }
            }
            // Return last expression or a unit literal
            if let Some(Stmt::Expression(es)) = stmts.last() {
                return Some(es.expr.clone());
            }
            return Some(Expr::Literal(LiteralExpr {
                literal_kind: LiteralKind::Boolean,
                value: "true".into(),
                span: SourceSpan {
                    file: String::new(),
                    start_line: 0,
                    start_column: 0,
                    end_line: 0,
                    end_column: 0,
                },
            }));
        }
        self.parse_expression(0)
    }

    // ============================================================
    // Statement parsing
    // ============================================================

    pub fn parse_statement(&mut self) -> Option<Stmt> {
        match self.peek().kind {
            TokenKind::Return => {
                let ret_tok = self.advance();
                let value = if self.peek().kind == TokenKind::Semicolon
                    || self.peek().kind == TokenKind::Eof
                    || self.peek().kind == TokenKind::RBrace
                {
                    None
                } else {
                    self.parse_expression(0)
                };
                let span = match &value {
                    Some(v) => self.merge_span(&ret_tok.span, &self.span_of_expr(v)),
                    None => ret_tok.span.clone(),
                };
                self.skip_semicolon();
                Some(Stmt::Return(ReturnStmt { value, span }))
            }
            TokenKind::Let | TokenKind::Const => {
                let kw_tok = self.advance();
                let name_tok = self.advance();
                if name_tok.kind != TokenKind::Identifier {
                    self.diagnostics.push(TangleDiagnostic {
                        code: "TANGLE_PARSE_ERROR".into(),
                        message: "Expected identifier after let/const".into(),
                        span: name_tok.span.clone(),
                    });
                    return None;
                }
                let type_annotation = if self.peek().kind == TokenKind::Colon {
                    self.advance(); // consume ':'
                                  // Simple inline type parsing: expect an identifier
                    let ty_tok = self.advance();
                    if ty_tok.kind != TokenKind::Identifier {
                        self.diagnostics.push(TangleDiagnostic {
                            code: "TANGLE_PARSE_ERROR".into(),
                            message: "Expected type name after ':'".into(),
                            span: ty_tok.span.clone(),
                        });
                        return None;
                    }
                    Some(TypeExpr::Named(NamedTypeExpr {
                        name: ty_tok.lexeme.clone(),
                        span: ty_tok.span.clone(),
                    }))
                } else {
                    None
                };
                if self.peek().kind != TokenKind::Eq {
                    self.diagnostics.push(TangleDiagnostic {
                        code: "TANGLE_PARSE_ERROR".into(),
                        message: "Expected '=' after let/const binding".into(),
                        span: self.peek().span.clone(),
                    });
                    return None;
                }
                self.advance(); // consume '='
                let value = self.parse_expression(0)?;
                let span = self.merge_span(&kw_tok.span, &self.span_of_expr(&value));
                self.skip_semicolon();
                if kw_tok.kind == TokenKind::Let {
                    Some(Stmt::Let(LetStmt {
                        name: name_tok.lexeme.clone(),
                        type_annotation,
                        value,
                        span,
                    }))
                } else {
                    Some(Stmt::Const(ConstStmt {
                        name: name_tok.lexeme.clone(),
                        type_annotation,
                        value,
                        span,
                    }))
                }
            }
            _ => {
                let expr = self.parse_expression(0)?;
                let span = self.span_of_expr(&expr);
                self.skip_semicolon();
                Some(Stmt::Expression(ExpressionStmt { expr, span }))
            }
        }
    }

    fn skip_semicolon(&mut self) {
        if self.peek().kind == TokenKind::Semicolon {
            self.advance();
        }
    }

    // ============================================================
    // Code body parsing
    // ============================================================

    pub fn parse_code_body(&mut self) -> CodeBody {
        let start = self.peek().span.clone();
        let mut statements = vec![];
        while self.peek().kind != TokenKind::Eof {
            if let Some(stmt) = self.parse_statement() {
                statements.push(stmt);
            } else {
                // Error recovery: skip to next semicolon or EOF
                while self.peek().kind != TokenKind::Semicolon
                    && self.peek().kind != TokenKind::Eof
                {
                    self.advance();
                }
                if self.peek().kind == TokenKind::Semicolon {
                    self.advance();
                }
            }
        }
        let end = self
            .tokens
            .last()
            .map(|t| t.span.clone())
            .unwrap_or_else(|| start.clone());
        CodeBody {
            statements,
            span: self.merge_span(&start, &end),
        }
    }
}

// ============================================================
// Helpers
// ============================================================

fn binary_op_from_token(kind: TokenKind) -> BinaryOp {
    match kind {
        TokenKind::Plus => BinaryOp::Add,
        TokenKind::Minus => BinaryOp::Sub,
        TokenKind::Star => BinaryOp::Mul,
        TokenKind::Slash => BinaryOp::Div,
        TokenKind::Percent => BinaryOp::Mod,
        TokenKind::EqEq => BinaryOp::Eq,
        TokenKind::Neq => BinaryOp::Neq,
        TokenKind::Lt => BinaryOp::Lt,
        TokenKind::Gt => BinaryOp::Gt,
        TokenKind::Lte => BinaryOp::Lte,
        TokenKind::Gte => BinaryOp::Gte,
        TokenKind::And => BinaryOp::And,
        TokenKind::Or => BinaryOp::Or,
        _ => BinaryOp::Add, // fallback (should not happen)
    }
}

fn extract_ident_name(expr: &Expr) -> Option<String> {
    match expr {
        Expr::Identifier(id) => Some(id.name.clone()),
        _ => None,
    }
}

// ============================================================
// Convenience functions
// ============================================================

pub fn parse_expression(tokens: &[Token]) -> (Option<Expr>, Vec<TangleDiagnostic>) {
    let mut parser = ParserState::new(tokens);
    let result = parser.parse_expression(0);
    (result, parser.diagnostics)
}

pub fn parse_statement(tokens: &[Token]) -> (Option<Stmt>, Vec<TangleDiagnostic>) {
    let mut parser = ParserState::new(tokens);
    let result = parser.parse_statement();
    (result, parser.diagnostics)
}

pub fn parse_code_body(tokens: &[Token]) -> (CodeBody, Vec<TangleDiagnostic>) {
    let mut parser = ParserState::new(tokens);
    let body = parser.parse_code_body();
    (body, parser.diagnostics)
}
