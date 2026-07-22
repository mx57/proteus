use std::fmt;

/// AI-level errors
#[derive(Debug)]
pub enum AiError {
    /// No strategies available
    NoCandidates,
    /// Registry error
    Registry(String),
    /// History store error
    History(String),
    /// Evolution error (not enough parents, etc.)
    Evolution(String),
    /// Serialization error
    Serialization(String),
    /// Orchestrator error
    Orchestrator(String),
}

impl fmt::Display for AiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AiError::NoCandidates => write!(f, "no candidates available"),
            AiError::Registry(msg) => write!(f, "registry error: {msg}"),
            AiError::History(msg) => write!(f, "history error: {msg}"),
            AiError::Evolution(msg) => write!(f, "evolution error: {msg}"),
            AiError::Serialization(msg) => write!(f, "serialization error: {msg}"),
            AiError::Orchestrator(msg) => write!(f, "orchestrator error: {msg}"),
        }
    }
}

impl std::error::Error for AiError {}
