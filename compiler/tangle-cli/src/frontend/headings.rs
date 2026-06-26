use crate::model::{HeadingRole, SourceSpan, TangleDiagnostic};
use crate::diagnostic::codes;

pub fn heading_role_for_depth(depth: usize) -> HeadingRole {
    match depth {
        1 => HeadingRole::Program,   2 => HeadingRole::Section,
        3 => HeadingRole::Type,      4 => HeadingRole::Callable,
        5 => HeadingRole::SemanticSection, 6 => HeadingRole::SemanticAtom,
        _ => HeadingRole::Section,
    }
}

#[derive(Debug, Clone)]
pub struct ParsedHeadingText {
    pub title: String,
    pub symbol_name: Option<String>,
    pub diagnostics: Vec<TangleDiagnostic>,
}

pub fn parse_heading_text(text: &str, depth: usize, span: &SourceSpan) -> ParsedHeadingText {
    let trimmed = text.trim();

    // Rule 1: explicit (identifier) in parentheses
    if let Some(open) = trimmed.rfind('(') {
        if let Some(close) = trimmed.rfind(')') {
            if close > open {
                let ident = &trimmed[open + 1..close];
                if is_valid_identifier(ident) {
                    let title = format!("{} {}", trimmed[..open].trim(), trimmed[close + 1..].trim()).trim().to_string();
                    return ParsedHeadingText { title, symbol_name: Some(ident.to_string()), diagnostics: vec![] };
                }
            }
        }
    }

    // Rule 2: pure ASCII identifier
    if is_valid_identifier(trimmed) {
        let mut diagnostics = vec![];
        let first_char = trimmed.chars().next().unwrap();
        match depth {
            1..=3 => {
                if !first_char.is_uppercase() {
                    diagnostics.push(TangleDiagnostic {
                        code: codes::INVALID_HEADING_CASE.into(),
                        message: format!("Heading '{}' at depth {} must be PascalCase", trimmed, depth),
                        span: span.clone(),
                    });
                }
            }
            4..=6 => {
                if !first_char.is_lowercase() {
                    diagnostics.push(TangleDiagnostic {
                        code: codes::INVALID_HEADING_CASE.into(),
                        message: format!("Heading '{}' at depth {} must be camelCase", trimmed, depth),
                        span: span.clone(),
                    });
                }
            }
            _ => {}
        }
        return ParsedHeadingText { title: trimmed.to_string(), symbol_name: Some(trimmed.to_string()), diagnostics };
    }

    // Multi-word ASCII warning
    if trimmed.chars().all(|c| c.is_ascii_graphic() || c == ' ') && trimmed.contains(' ') {
        return ParsedHeadingText {
            title: trimmed.to_string(), symbol_name: None,
            diagnostics: vec![TangleDiagnostic {
                code: codes::HEADING_MULTI_WORD.into(),
                message: format!("Heading '{}' has multiple words — add explicit identifier in parentheses", trimmed),
                span: span.clone(),
            }],
        };
    }

    // Rule 3: Unicode fallback
    ParsedHeadingText { title: trimmed.to_string(), symbol_name: Some(trimmed.to_string()), diagnostics: vec![] }
}

fn is_valid_identifier(s: &str) -> bool {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {},
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}
