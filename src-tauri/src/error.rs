//! The error every command returns (TECH.md §5).

use serde::Serialize;
use turbodiff_repo::Error;

/// Tagged by `kind`, so the UI can show the matching empty or error state
/// (PRD §7).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type, thiserror::Error)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum AppError {
    #[error("{path} is not a git repository")]
    NotARepository { path: String },
    #[error("ref not found: {name}")]
    RefNotFound { name: String },
    /// The window has no repository open.
    #[error("no repository is open")]
    NoRepository,
    /// The path isn't in the current list of changed files, usually because
    /// the list was rebuilt since the request was made.
    #[error("{path} is not in the list of changed files")]
    FileNotFound { path: String },
    /// The comparison changed before the result was ready. The response is
    /// no longer needed and can be ignored.
    #[error("cancelled")]
    Cancelled,
    #[error("{message}")]
    Io { message: String },
    #[error("{message}")]
    Internal { message: String },
}

impl AppError {
    pub fn internal(err: impl std::fmt::Display) -> Self {
        Self::Internal {
            message: err.to_string(),
        }
    }
}

impl From<Error> for AppError {
    fn from(err: Error) -> Self {
        match err {
            Error::NotARepository(path) => Self::NotARepository {
                path: path.display().to_string(),
            },
            Error::RefNotFound(name) => Self::RefNotFound { name },
            Error::Io(err) => Self::Io {
                message: err.to_string(),
            },
            Error::Internal(message) => Self::Internal { message },
            Error::Cancelled => Self::Cancelled,
        }
    }
}

impl From<tauri::Error> for AppError {
    fn from(err: tauri::Error) -> Self {
        Self::internal(err)
    }
}

pub type AppResult<T> = Result<T, AppError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repository_errors_keep_their_kind() {
        let err = AppError::from(Error::RefNotFound("main".into()));
        assert_eq!(
            err,
            AppError::RefNotFound {
                name: "main".into()
            }
        );
        assert_eq!(AppError::from(Error::Cancelled), AppError::Cancelled);
    }

    #[test]
    fn serialises_tagged_by_kind() {
        let json = serde_json::to_value(AppError::NotARepository {
            path: "/tmp/x".into(),
        })
        .unwrap();
        assert_eq!(
            json,
            serde_json::json!({ "kind": "notARepository", "path": "/tmp/x" })
        );
        let json = serde_json::to_value(AppError::Cancelled).unwrap();
        assert_eq!(json, serde_json::json!({ "kind": "cancelled" }));
    }
}
