use crate::model::{SourceSpan, TangleDiagnostic};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
    Number, String, True, False, Identifier,
    Return, Let, Const, If, Else, This,
    PipeOp, Dot, Comma, Colon, Semicolon,
    LParen, RParen, LBrace, RBrace, LBracket, RBracket,
    Plus, Minus, Star, Slash, Percent,
    Eq, EqEq, Neq, Lt, Gt, Lte, Gte,
    And, Or, Bang, Pipe, Arrow, FatArrow,
    Question, ErrorKw, Eof,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub lexeme: String,
    pub span: SourceSpan,
}

pub struct Lexer<'a> {
    #[allow(dead_code)]
    source: &'a str,
    file: &'a str,
    chars: Vec<char>,
    pos: usize,
    line: usize,
    col: usize,
    diagnostics: Vec<TangleDiagnostic>,
}

impl<'a> Lexer<'a> {
    pub fn new(source: &'a str, file: &'a str) -> Self {
        Lexer { source, file, chars: source.chars().collect(), pos: 0, line: 1, col: 1, diagnostics: vec![] }
    }

    pub fn diagnostics(&self) -> &[TangleDiagnostic] { &self.diagnostics }

    fn current(&self) -> Option<char> { self.chars.get(self.pos).copied() }
    fn peek(&self, offset: usize) -> Option<char> { self.chars.get(self.pos + offset).copied() }

    fn advance(&mut self) -> Option<char> {
        let c = self.chars.get(self.pos).copied();
        if let Some(ch) = c {
            self.pos += 1;
            if ch == '\n' { self.line += 1; self.col = 1; }
            else { self.col += 1; }
        }
        c
    }

    fn make_span(&self, sl: usize, sc: usize, el: usize, ec: usize) -> SourceSpan {
        SourceSpan { file: self.file.to_string(), start_line: sl, start_column: sc, end_line: el, end_column: ec }
    }

    pub fn tokenize(&mut self) -> Vec<Token> {
        let mut tokens = vec![];
        loop {
            self.skip_whitespace();
            let sl = self.line; let sc = self.col;
            let c = match self.current() { Some(c) => c, None => break };
            let token = match c {
                '0'..='9' => self.read_number(sl, sc),
                '"' => self.read_string(sl, sc),
                '|' if self.peek(1) == Some('>') => { self.advance(); self.advance(); self.make_tok(TokenKind::PipeOp, "|>", sl, sc) }
                '|' if self.peek(1) == Some('|') => { self.advance(); self.advance(); self.make_tok(TokenKind::Or, "||", sl, sc) }
                '|' => { self.advance(); self.make_tok(TokenKind::Pipe, "|", sl, sc) }
                '=' if self.peek(1) == Some('=') => { self.advance(); self.advance(); self.make_tok(TokenKind::EqEq, "==", sl, sc) }
                '=' if self.peek(1) == Some('>') => { self.advance(); self.advance(); self.make_tok(TokenKind::FatArrow, "=>", sl, sc) }
                '=' => { self.advance(); self.make_tok(TokenKind::Eq, "=", sl, sc) }
                '!' if self.peek(1) == Some('=') => { self.advance(); self.advance(); self.make_tok(TokenKind::Neq, "!=", sl, sc) }
                '!' => { self.advance(); self.make_tok(TokenKind::Bang, "!", sl, sc) }
                '<' if self.peek(1) == Some('=') => { self.advance(); self.advance(); self.make_tok(TokenKind::Lte, "<=", sl, sc) }
                '<' => { self.advance(); self.make_tok(TokenKind::Lt, "<", sl, sc) }
                '>' if self.peek(1) == Some('=') => { self.advance(); self.advance(); self.make_tok(TokenKind::Gte, ">=", sl, sc) }
                '>' => { self.advance(); self.make_tok(TokenKind::Gt, ">", sl, sc) }
                '&' if self.peek(1) == Some('&') => { self.advance(); self.advance(); self.make_tok(TokenKind::And, "&&", sl, sc) }
                '-' if self.peek(1) == Some('>') => { self.advance(); self.advance(); self.make_tok(TokenKind::Arrow, "->", sl, sc) }
                '-' => { self.advance(); self.make_tok(TokenKind::Minus, "-", sl, sc) }
                '.' => { self.advance(); self.make_tok(TokenKind::Dot, ".", sl, sc) }
                ',' => { self.advance(); self.make_tok(TokenKind::Comma, ",", sl, sc) }
                ':' => { self.advance(); self.make_tok(TokenKind::Colon, ":", sl, sc) }
                ';' => { self.advance(); self.make_tok(TokenKind::Semicolon, ";", sl, sc) }
                '(' => { self.advance(); self.make_tok(TokenKind::LParen, "(", sl, sc) }
                ')' => { self.advance(); self.make_tok(TokenKind::RParen, ")", sl, sc) }
                '{' => { self.advance(); self.make_tok(TokenKind::LBrace, "{", sl, sc) }
                '}' => { self.advance(); self.make_tok(TokenKind::RBrace, "}", sl, sc) }
                '[' => { self.advance(); self.make_tok(TokenKind::LBracket, "[", sl, sc) }
                ']' => { self.advance(); self.make_tok(TokenKind::RBracket, "]", sl, sc) }
                '+' => { self.advance(); self.make_tok(TokenKind::Plus, "+", sl, sc) }
                '*' => { self.advance(); self.make_tok(TokenKind::Star, "*", sl, sc) }
                '/' => { self.advance(); self.make_tok(TokenKind::Slash, "/", sl, sc) }
                '%' => { self.advance(); self.make_tok(TokenKind::Percent, "%", sl, sc) }
                '?' => { self.advance(); self.make_tok(TokenKind::Question, "?", sl, sc) }
                c if c.is_alphabetic() || c == '_' => self.read_identifier_or_keyword(sl, sc),
                _ => {
                    self.advance();
                    self.diagnostics.push(TangleDiagnostic { code: "TANGLE_LEXER_ERROR".into(), message: format!("Unknown character: '{}'", c), span: self.make_span(sl, sc, self.line, self.col) });
                    Token { kind: TokenKind::Eof, lexeme: c.to_string(), span: self.make_span(sl, sc, self.line, self.col) }
                }
            };
            tokens.push(token);
        }
        tokens.push(Token { kind: TokenKind::Eof, lexeme: String::new(), span: self.make_span(self.line, self.col, self.line, self.col) });
        tokens
    }

    fn skip_whitespace(&mut self) {
        while let Some(c) = self.current() {
            if c == ' ' || c == '\t' || c == '\r' || c == '\n' { self.advance(); } else { break; }
        }
    }

    fn read_number(&mut self, sl: usize, sc: usize) -> Token {
        let mut lexeme = String::new();
        while let Some(c) = self.current() {
            if c.is_ascii_digit() || c == '.' { lexeme.push(c); self.advance(); } else { break; }
        }
        Token { kind: TokenKind::Number, lexeme, span: self.make_span(sl, sc, self.line, self.col) }
    }

    fn read_string(&mut self, sl: usize, sc: usize) -> Token {
        self.advance();
        let mut lexeme = String::new();
        while let Some(c) = self.current() {
            if c == '"' { self.advance(); return Token { kind: TokenKind::String, lexeme, span: self.make_span(sl, sc, self.line, self.col) }; }
            lexeme.push(c); self.advance();
        }
        self.diagnostics.push(TangleDiagnostic { code: "TANGLE_UNTERMINATED_STRING".into(), message: "Unterminated string literal".into(), span: self.make_span(sl, sc, self.line, self.col) });
        Token { kind: TokenKind::String, lexeme, span: self.make_span(sl, sc, self.line, self.col) }
    }

    fn read_identifier_or_keyword(&mut self, sl: usize, sc: usize) -> Token {
        let mut lexeme = String::new();
        while let Some(c) = self.current() {
            if c.is_alphanumeric() || c == '_' { lexeme.push(c); self.advance(); } else { break; }
        }
        let kind = match lexeme.as_str() {
            "return" => TokenKind::Return, "let" => TokenKind::Let, "const" => TokenKind::Const,
            "if" => TokenKind::If, "else" => TokenKind::Else, "this" => TokenKind::This,
            "true" => TokenKind::True, "false" => TokenKind::False,
            "match" => TokenKind::ErrorKw, "panic" => TokenKind::ErrorKw,
            _ => TokenKind::Identifier,
        };
        Token { kind, lexeme, span: self.make_span(sl, sc, self.line, self.col) }
    }

    fn make_tok(&self, kind: TokenKind, lexeme: &str, sl: usize, sc: usize) -> Token {
        Token { kind, lexeme: lexeme.to_string(), span: self.make_span(sl, sc, self.line, self.col) }
    }
}

pub fn tokenize(source: &str, file: &str) -> (Vec<Token>, Vec<TangleDiagnostic>) {
    let mut lexer = Lexer::new(source, file);
    let tokens = lexer.tokenize();
    let diagnostics = lexer.diagnostics;
    (tokens, diagnostics.to_vec())
}
