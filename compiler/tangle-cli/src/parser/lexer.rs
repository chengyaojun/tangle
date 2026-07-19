use crate::model::{SourceSpan, TangleDiagnostic};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
    Number, String, True, False, Identifier,
    Return, Let, Const, If, Else, This, Is,
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
            "is" => TokenKind::Is,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenize_integer() {
        let (tokens, _diags) = tokenize("42", "test.md");
        assert_eq!(tokens.len(), 2); // number + eof
        assert_eq!(tokens[0].kind, TokenKind::Number);
        assert_eq!(tokens[0].lexeme, "42");
    }

    #[test]
    fn test_tokenize_decimal() {
        let (tokens, _diags) = tokenize("3.14", "test.md");
        assert_eq!(tokens[0].kind, TokenKind::Number);
        assert_eq!(tokens[0].lexeme, "3.14");
    }

    #[test]
    fn test_tokenize_string() {
        let (tokens, _diags) = tokenize("\"hello\"", "test.md");
        assert_eq!(tokens[0].kind, TokenKind::String);
        assert_eq!(tokens[0].lexeme, "hello");
    }

    #[test]
    fn test_tokenize_unterminated_string() {
        let (tokens, diags) = tokenize("\"unclosed", "test.md");
        assert_eq!(tokens[0].kind, TokenKind::String);
        assert_eq!(tokens[0].lexeme, "unclosed");
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].code, "TANGLE_UNTERMINATED_STRING");
        assert_eq!(diags[0].message, "Unterminated string literal");
    }

    #[test]
    fn test_tokenize_keywords() {
        let (tokens, _diags) = tokenize("return let const if else this true false", "test.md");
        assert_eq!(tokens[0].kind, TokenKind::Return);
        assert_eq!(tokens[1].kind, TokenKind::Let);
        assert_eq!(tokens[2].kind, TokenKind::Const);
        assert_eq!(tokens[3].kind, TokenKind::If);
        assert_eq!(tokens[4].kind, TokenKind::Else);
        assert_eq!(tokens[5].kind, TokenKind::This);
        assert_eq!(tokens[6].kind, TokenKind::True);
        assert_eq!(tokens[7].kind, TokenKind::False);
    }

    #[test]
    fn test_tokenize_identifiers() {
        let (tokens, _diags) = tokenize("foo bar123 _private", "test.md");
        assert_eq!(tokens[0].kind, TokenKind::Identifier);
        assert_eq!(tokens[0].lexeme, "foo");
        assert_eq!(tokens[1].kind, TokenKind::Identifier);
        assert_eq!(tokens[1].lexeme, "bar123");
        assert_eq!(tokens[2].kind, TokenKind::Identifier);
        assert_eq!(tokens[2].lexeme, "_private");
    }

    #[test]
    fn test_tokenize_multi_char_operators() {
        let (tokens, _diags) = tokenize("== != <= >= && || |> => ->", "test.md");
        assert_eq!(tokens[0].kind, TokenKind::EqEq);
        assert_eq!(tokens[0].lexeme, "==");
        assert_eq!(tokens[1].kind, TokenKind::Neq);
        assert_eq!(tokens[1].lexeme, "!=");
        assert_eq!(tokens[2].kind, TokenKind::Lte);
        assert_eq!(tokens[2].lexeme, "<=");
        assert_eq!(tokens[3].kind, TokenKind::Gte);
        assert_eq!(tokens[3].lexeme, ">=");
        assert_eq!(tokens[4].kind, TokenKind::And);
        assert_eq!(tokens[4].lexeme, "&&");
        assert_eq!(tokens[5].kind, TokenKind::Or);
        assert_eq!(tokens[5].lexeme, "||");
        assert_eq!(tokens[6].kind, TokenKind::PipeOp);
        assert_eq!(tokens[6].lexeme, "|>");
        assert_eq!(tokens[7].kind, TokenKind::FatArrow);
        assert_eq!(tokens[7].lexeme, "=>");
        assert_eq!(tokens[8].kind, TokenKind::Arrow);
        assert_eq!(tokens[8].lexeme, "->");
    }

    #[test]
    fn test_tokenize_single_char_operators_and_delimiters() {
        let (tokens, _diags) = tokenize("+ - * / . , : ; ( ) { } [ ]", "test.md");
        assert_eq!(tokens[0].kind, TokenKind::Plus);
        assert_eq!(tokens[0].lexeme, "+");
        assert_eq!(tokens[1].kind, TokenKind::Minus);
        assert_eq!(tokens[1].lexeme, "-");
        assert_eq!(tokens[2].kind, TokenKind::Star);
        assert_eq!(tokens[2].lexeme, "*");
        assert_eq!(tokens[3].kind, TokenKind::Slash);
        assert_eq!(tokens[3].lexeme, "/");
        assert_eq!(tokens[4].kind, TokenKind::Dot);
        assert_eq!(tokens[4].lexeme, ".");
        assert_eq!(tokens[5].kind, TokenKind::Comma);
        assert_eq!(tokens[5].lexeme, ",");
        assert_eq!(tokens[6].kind, TokenKind::Colon);
        assert_eq!(tokens[6].lexeme, ":");
        assert_eq!(tokens[7].kind, TokenKind::Semicolon);
        assert_eq!(tokens[7].lexeme, ";");
        assert_eq!(tokens[8].kind, TokenKind::LParen);
        assert_eq!(tokens[8].lexeme, "(");
        assert_eq!(tokens[9].kind, TokenKind::RParen);
        assert_eq!(tokens[9].lexeme, ")");
        assert_eq!(tokens[10].kind, TokenKind::LBrace);
        assert_eq!(tokens[10].lexeme, "{");
        assert_eq!(tokens[11].kind, TokenKind::RBrace);
        assert_eq!(tokens[11].lexeme, "}");
        assert_eq!(tokens[12].kind, TokenKind::LBracket);
        assert_eq!(tokens[12].lexeme, "[");
        assert_eq!(tokens[13].kind, TokenKind::RBracket);
        assert_eq!(tokens[13].lexeme, "]");
    }

    #[test]
    fn test_tokenize_question_mark() {
        let (tokens, _diags) = tokenize("?", "test.md");
        assert_eq!(tokens[0].kind, TokenKind::Question);
        assert_eq!(tokens[0].lexeme, "?");
    }

    #[test]
    fn test_tokenize_pipe() {
        let (tokens, _diags) = tokenize("|", "test.md");
        assert_eq!(tokens[0].kind, TokenKind::Pipe);
        assert_eq!(tokens[0].lexeme, "|");
    }

    #[test]
    fn test_tokenize_complex_expression() {
        let (tokens, _diags) = tokenize("return this { is_active: true }", "test.md");
        let kinds: Vec<TokenKind> = tokens.iter().map(|t| t.kind).collect();
        assert_eq!(kinds, vec![
            TokenKind::Return, TokenKind::This, TokenKind::LBrace,
            TokenKind::Identifier, TokenKind::Colon, TokenKind::True,
            TokenKind::RBrace, TokenKind::Eof,
        ]);
    }

    #[test]
    fn test_token_spans() {
        let (tokens, _diags) = tokenize("  x\n  42", "test.md");
        assert_eq!(tokens[0].kind, TokenKind::Identifier);
        assert_eq!(tokens[0].lexeme, "x");
        assert_eq!(tokens[0].span.start_line, 1);
        assert_eq!(tokens[0].span.start_column, 3);
        assert_eq!(tokens[1].kind, TokenKind::Number);
        assert_eq!(tokens[1].lexeme, "42");
        assert_eq!(tokens[1].span.start_line, 2);
    }

    #[test]
    fn lex_is_keyword() {
        let (tokens, _diags) = tokenize("if x is Some", "test.md");
        assert!(
            tokens.iter().any(|t| matches!(t.kind, TokenKind::Is)),
            "expected TokenKind::Is in: {:?}",
            tokens
        );
    }

    #[test]
    fn lex_is_alone_is_keyword() {
        // "is" 单独出现仍是关键字（保留字），不可作变量名
        let (tokens, _diags) = tokenize("is", "test.md");
        assert!(tokens.iter().any(|t| matches!(t.kind, TokenKind::Is)));
    }
}
