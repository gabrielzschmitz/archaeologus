use thiserror::Error;

#[derive(Debug, Error)]
pub enum GitError {
    #[error("libgit2 error: {0}")]
    Git2(#[from] git2::Error),

    #[error("repository not found at {0}")]
    RepoNotFound(String),

    #[error("commit not found: {0}")]
    CommitNotFound(String),

    #[error("file not found in repository: {0}")]
    FileNotFound(String),

    #[error("invalid UTF-8 in {context}: {source}")]
    Utf8 {
        context: String,
        source: std::str::Utf8Error,
    },

    #[error("authentication failed for {0}")]
    AuthFailed(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, GitError>;
