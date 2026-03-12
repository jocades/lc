use crate::lexer::Span;
use crate::source::Source;

#[derive(Debug)]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Debug)]
pub struct Label {
    pub span: Span,
    pub message: Option<String>,
}

#[derive(Debug)]
pub struct Diagnostic {
    pub severity: Severity,
    pub message: String,
    pub labels: Vec<Label>,
    pub notes: Vec<String>,
}

impl Diagnostic {
    pub fn error(message: impl Into<String>, span: impl Into<Span>) -> Self {
        Self {
            severity: Severity::Error,
            message: message.into(),
            labels: vec![Label {
                span: span.into(),
                message: None,
            }],
            notes: Vec::new(),
        }
    }

    pub fn warning(message: impl Into<String>, span: impl Into<Span>) -> Self {
        Self {
            severity: Severity::Warning,
            message: message.into(),
            labels: vec![Label {
                span: span.into(),
                message: None,
            }],
            notes: Vec::new(),
        }
    }

    pub fn with_label(mut self, message: impl Into<String>, span: impl Into<Span>) -> Self {
        self.labels.push(Label {
            span: span.into(),
            message: Some(message.into()),
        });
        self
    }

    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.notes.push(note.into());
        self
    }
}

impl Severity {
    fn as_str(&self) -> &'static str {
        match self {
            Severity::Error => "error",
            Severity::Warning => "warning",
        }
    }
}

pub fn render_all(source_text: &str, diagnostics: &[Diagnostic]) -> String {
    let source = Source::new(source_text);
    let mut out = String::new();

    for (i, diagnostic) in diagnostics.iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        render_into(&mut out, &source, diagnostic);
    }

    out
}

fn render_into(out: &mut String, source: &Source<'_>, diagnostic: &Diagnostic) {
    use std::fmt::Write;

    let Some(primary) = diagnostic.labels.first() else {
        let _ = writeln!(
            out,
            "{}: {}",
            diagnostic.severity.as_str(),
            diagnostic.message
        );
        return;
    };

    let (line, col) = source.line_col(primary.span.range());
    let line_index = source.line_of(primary.span.start);
    let line_text = source.line_text(line_index);
    let line_no = line.to_string();
    let gutter_width = line_no.len();
    let line_start = source.line_range(line_index).start;

    let start_col = primary.span.start.saturating_sub(line_start);
    let end_col = primary.span.end.saturating_sub(line_start);
    let underline_width = end_col.saturating_sub(start_col).max(1);
    let caret_pad = " ".repeat(start_col);
    let carets = "^".repeat(underline_width);

    let _ = writeln!(
        out,
        "{}: {}",
        diagnostic.severity.as_str(),
        diagnostic.message
    );
    let _ = writeln!(out, " --> {}:{}", line, col);
    let _ = writeln!(out, "{:>width$} |", "", width = gutter_width);
    let _ = writeln!(out, "{} | {}", line_no, line_text);

    match &primary.message {
        Some(message) => {
            let _ = writeln!(
                out,
                "{:>width$} | {}{} {}",
                "",
                caret_pad,
                carets,
                message,
                width = gutter_width
            );
        }
        None => {
            let _ = writeln!(
                out,
                "{:>width$} | {}{}",
                "",
                caret_pad,
                carets,
                width = gutter_width
            );
        }
    }

    for label in diagnostic.labels.iter().skip(1) {
        let (line, col) = source.line_col(label.span.range());
        let _ = writeln!(out, " = label at {}:{}", line, col);
        if let Some(message) = &label.message {
            let _ = writeln!(out, "   {}", message);
        }
    }

    for note in &diagnostic.notes {
        let _ = writeln!(out, " = note: {}", note);
    }
}
