use std::path::PathBuf;

pub type Result<T, E = Error> = std::result::Result<T, E>;

/// Errors from repository access. The variants match `AppError` in the IPC API
/// (TECH.md §5), so the UI can show the matching empty or error state.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{} is not a git repository", .0.display())]
    NotARepository(PathBuf),
    #[error("ref not found: {0}")]
    RefNotFound(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Internal(String),
}

impl Error {
    pub(crate) fn internal(err: impl std::fmt::Display) -> Self {
        Self::Internal(err.to_string())
    }
}

impl From<gix::Error> for Error {
    fn from(err: gix::Error) -> Self {
        Self::internal(err)
    }
}
