use std::path::Path;

#[derive(Debug, thiserror::Error)]
pub enum LedgerError {
    #[error("{0}")]
    NotFound(String),
    #[error("{message}: {}", candidates.join(", "))]
    Ambiguous {
        message: String,
        candidates: Vec<String>,
    },
    #[error("{0}")]
    Invalid(String),
    #[error("{message}; allowed: {}", if allowed.is_empty() { "none".to_string() } else { allowed.join(", ") })]
    IllegalTransition {
        message: String,
        allowed: Vec<String>,
    },
    #[error("{0}")]
    Conflict(String),
    #[error("{context}: {source}")]
    Io {
        context: String,
        #[source]
        source: std::io::Error,
    },
    #[error("{0}")]
    Internal(String),
}

pub type Result<T> = std::result::Result<T, LedgerError>;

impl LedgerError {
    pub fn io(path: &Path, source: std::io::Error) -> Self {
        LedgerError::Io {
            context: path.display().to_string(),
            source,
        }
    }

    pub fn exit_code(&self) -> u8 {
        match self {
            LedgerError::NotFound(_) | LedgerError::Ambiguous { .. } => 3,
            LedgerError::Invalid(_) | LedgerError::IllegalTransition { .. } => 4,
            LedgerError::Conflict(_) => 5,
            LedgerError::Io { .. } | LedgerError::Internal(_) => 1,
        }
    }

    pub fn json_code(&self) -> &'static str {
        match self {
            LedgerError::NotFound(_) => "not_found",
            LedgerError::Ambiguous { .. } => "ambiguous",
            LedgerError::Invalid(_) => "invalid",
            LedgerError::IllegalTransition { .. } => "illegal_transition",
            LedgerError::Conflict(_) => "conflict",
            LedgerError::Io { .. } | LedgerError::Internal(_) => "internal",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn all() -> Vec<LedgerError> {
        vec![
            LedgerError::NotFound("x".into()),
            LedgerError::Ambiguous {
                message: "x".into(),
                candidates: vec!["a/1".into(), "b/1".into()],
            },
            LedgerError::Invalid("x".into()),
            LedgerError::IllegalTransition {
                message: "x".into(),
                allowed: vec![],
            },
            LedgerError::Conflict("x".into()),
            LedgerError::io(Path::new("p"), std::io::Error::other("boom")),
            LedgerError::Internal("x".into()),
        ]
    }

    #[test]
    fn exit_and_json_codes() {
        let codes: Vec<(u8, &str)> = all()
            .iter()
            .map(|e| (e.exit_code(), e.json_code()))
            .collect();
        assert_eq!(
            codes,
            vec![
                (3, "not_found"),
                (3, "ambiguous"),
                (4, "invalid"),
                (4, "illegal_transition"),
                (5, "conflict"),
                (1, "internal"),
                (1, "internal"),
            ]
        );
    }

    #[test]
    fn ambiguous_message_lists_candidates() {
        assert_eq!(all()[1].to_string(), "x: a/1, b/1");
    }
}
