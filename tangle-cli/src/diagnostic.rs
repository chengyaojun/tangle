use codespan_reporting::diagnostic::{Diagnostic, Label};
use codespan_reporting::files::SimpleFiles;
use codespan_reporting::term;
use crate::model::TangleDiagnostic;

/// 诊断码常量
pub mod codes {
    pub const HEADING_MULTI_WORD: &str = "TANGLE_HEADING_MULTI_WORD";
    pub const INVALID_HEADING_CASE: &str = "TANGLE_INVALID_HEADING_CASE";
    pub const DUPLICATE_SYMBOL: &str = "TANGLE_DUPLICATE_SYMBOL";
    pub const PARSE_ERROR: &str = "TANGLE_PARSE_ERROR";
    pub const TYPE_ERROR: &str = "TANGLE_TYPE_ERROR";
    pub const TYPE_ALL_ERROR: &str = "TANGLE_TYPE_ALL_ERROR";
    pub const PANIC_REACHED: &str = "TANGLE_PANIC_REACHED";
    pub const MATCH_NOT_EXHAUSTIVE: &str = "TANGLE_MATCH_NOT_EXHAUSTIVE";
    pub const IR_VALIDATION_ERROR: &str = "TANGLE_IR_VALIDATION_ERROR";
    pub const UNDECLARED_ERROR: &str = "TANGLE_UNDECLARED_ERROR";
    pub const SYMBOL_NOT_FOUND: &str = "TANGLE_SYMBOL_NOT_FOUND";
}

/// 将诊断列表渲染到 stderr
pub fn render_diagnostics(diagnostics: &[TangleDiagnostic], source: &str, file: &str) {
    let mut files = SimpleFiles::new();
    let file_id = files.add(file, source);

    for diag in diagnostics {
        let diagnostic = Diagnostic::error()
            .with_message(&diag.message)
            .with_code(&diag.code)
            .with_labels(vec![Label::primary(
                file_id,
                diag.span.start_line..diag.span.end_line,
            )]);

        let writer = term::termcolor::StandardStream::stderr(
            term::termcolor::ColorChoice::Auto,
        );
        let config = term::Config::default();
        if let Err(e) = term::emit(&mut writer.lock(), &config, &files, &diagnostic) {
            eprintln!("Failed to render diagnostic: {}", e);
        }
    }
}
