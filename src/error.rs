use std::fmt;

/// Errors that can occur during mermaid parsing/rendering
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MermaidError {
    /// Empty input provided
    EmptyInput,
    /// Parse error at specific line
    ParseError {
        line: usize,
        message: String,
        suggestion: Option<String>,
    },
    /// Layout error (e.g., cycle detected)
    LayoutError(String),
}

impl fmt::Display for MermaidError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MermaidError::EmptyInput => write!(f, "Empty input"),
            MermaidError::ParseError {
                line,
                message,
                suggestion,
            } => {
                write!(f, "Line {}: {}", line, message)?;
                if let Some(sug) = suggestion {
                    write!(f, " (Suggestion: {})", sug)?;
                }
                Ok(())
            }
            MermaidError::LayoutError(msg) => write!(f, "Layout error: {}", msg),
        }
    }
}

impl std::error::Error for MermaidError {}
