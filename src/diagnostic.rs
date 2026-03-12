use crate::lexer::Span;

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
