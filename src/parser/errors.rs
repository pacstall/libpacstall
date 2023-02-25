use miette::{Diagnostic, Report, SourceSpan};
use thiserror::Error;

#[derive(Debug, Diagnostic, Error)]
#[error("Parser error")]
pub struct ParseError {
    /// Source code.
    #[source_code]
    pub input: String,

    #[related]
    pub related: Vec<Report>,
}

#[derive(Debug, Diagnostic, Clone, Eq, PartialEq, Error)]
#[error("Invalid field")]
pub struct FieldError {
    /// States the issues with the field.
    pub field_label: String,

    /// Span of the field which has the error.
    #[label("{field_label}")]
    pub field_span: SourceSpan,

    /// Span of the erroneous source code.
    #[label("here")]
    pub error_span: SourceSpan,

    /// Suggestion for fixing the parser error.
    #[help]
    pub help: String,
}

#[derive(Debug, Diagnostic, Clone, Eq, PartialEq, Error)]
#[error("Missing field")]
pub struct MissingField {
    pub label: &'static str,
}

#[derive(Debug, Diagnostic, Clone, Eq, PartialEq, Error)]
#[error("Bad syntax")]
pub struct BadSyntax;
