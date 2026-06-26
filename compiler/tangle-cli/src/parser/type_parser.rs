use crate::ast::*;
use crate::model::{SourceSpan, TangleDiagnostic};
use crate::parser::lexer::{tokenize, Token, TokenKind};

// ============================================================
// Public entry point
// ============================================================

pub fn parse_type_expr(source: &str, file: &str) -> (Option<TypeExpr>, Vec<TangleDiagnostic>) {
    let (tokens, lexer_diags) = tokenize(source, file);
    let mut parser = TypeParser::new(&tokens);
    let result = parser.parse_sum_type();
    let mut diags = lexer_diags;
    diags.extend(parser.diagnostics);
    (result, diags)
}

// ============================================================
// TypeParser
// ============================================================

struct TypeParser<'a> {
    tokens: &'a [Token],
    pos: usize,
    diagnostics: Vec<TangleDiagnostic>,
}

impl<'a> TypeParser<'a> {
    fn new(tokens: &'a [Token]) -> Self {
        TypeParser {
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

    // ============================================================
    // Sum type: T1 | T2 | T3
    // ============================================================

    fn parse_sum_type(&mut self) -> Option<TypeExpr> {
        let mut left = self.parse_function_type()?;
        while self.peek().kind == TokenKind::Pipe {
            self.advance(); // consume '|'
            let right = self.parse_function_type()?;
            left = match left {
                TypeExpr::Sum(mut s) => {
                    let right_span = span_of_type(&right);
                    s.variants.push(right);
                    s.span = merge_spans(&s.span, &right_span);
                    TypeExpr::Sum(s)
                }
                other => {
                    let right_span = span_of_type(&right);
                    let span = merge_spans(&span_of_type(&other), &right_span);
                    TypeExpr::Sum(SumTypeExpr {
                        variants: vec![other, right],
                        span,
                    })
                }
            };
        }
        Some(left)
    }

    // ============================================================
    // Function type: Param -> Return
    // ============================================================

    fn parse_function_type(&mut self) -> Option<TypeExpr> {
        let left = self.parse_generic_or_primary()?;
        if self.peek().kind == TokenKind::Arrow {
            self.advance(); // consume '->'
            let returns = self.parse_sum_type()?;
            let params = match &left {
                TypeExpr::Primitive(_) | TypeExpr::Named(_) | TypeExpr::Generic(_) => {
                    vec![left.clone()]
                }
                TypeExpr::Sum(inner) => {
                    // For sum types in param position, each variant is treated as a parameter
                    // e.g., String | Int -> Bool  =>  (String | Int) -> Bool
                    vec![TypeExpr::Sum(inner.clone())]
                }
                _ => vec![left.clone()],
            };
            let span = merge_spans(&span_of_type(&params[0]), &span_of_type(&returns));
            return Some(TypeExpr::Function(FunctionTypeExpr {
                params,
                returns: Box::new(returns),
                span,
            }));
        }
        Some(left)
    }

    // ============================================================
    // Generic type: Result<String, Error>  or  Array<Int>
    // ============================================================

    fn parse_generic_or_primary(&mut self) -> Option<TypeExpr> {
        let name_tok = self.advance();
        if name_tok.kind != TokenKind::Identifier {
            self.diagnostics.push(TangleDiagnostic {
                code: "TANGLE_TYPE_PARSE_ERROR".into(),
                message: format!(
                    "Expected type name, got: {:?}",
                    name_tok.kind
                ),
                span: name_tok.span.clone(),
            });
            return None;
        }
        let name = name_tok.lexeme.clone();
        let span = name_tok.span.clone();

        // Check for generic type args: Name<A, B, ...>
        if self.peek().kind == TokenKind::Lt {
            self.advance(); // consume '<'
            let mut args = vec![];
            loop {
                args.push(self.parse_sum_type()?);
                if self.peek().kind == TokenKind::Comma {
                    self.advance(); // consume ','
                    continue;
                }
                if self.peek().kind == TokenKind::Gt {
                    self.advance(); // consume '>'
                    break;
                }
                self.diagnostics.push(TangleDiagnostic {
                    code: "TANGLE_TYPE_PARSE_ERROR".into(),
                    message: "Expected ',' or '>' in type arguments".into(),
                    span: self.peek().span.clone(),
                });
                return None;
            }
            let end_span = self.tokens[self.pos - 1].span.clone();
            return Some(TypeExpr::Generic(GenericTypeExpr {
                base: name,
                type_args: args,
                span: merge_spans(&span, &end_span),
            }));
        }

        // Distinguish primitive types from named (user-defined) types
        match name.as_str() {
            "String" | "Int" | "Float" | "Bool" => {
                Some(TypeExpr::Primitive(PrimitiveTypeExpr { name, span }))
            }
            _ => Some(TypeExpr::Named(NamedTypeExpr { name, span })),
        }
    }
}

// ============================================================
// Helpers
// ============================================================

fn span_of_type(ty: &TypeExpr) -> SourceSpan {
    match ty {
        TypeExpr::Primitive(t) => t.span.clone(),
        TypeExpr::Sum(t) => t.span.clone(),
        TypeExpr::Generic(t) => t.span.clone(),
        TypeExpr::Function(t) => t.span.clone(),
        TypeExpr::Named(t) => t.span.clone(),
    }
}

fn merge_spans(start: &SourceSpan, end: &SourceSpan) -> SourceSpan {
    SourceSpan {
        file: start.file.clone(),
        start_line: start.start_line,
        start_column: start.start_column,
        end_line: end.end_line,
        end_column: end.end_column,
    }
}
