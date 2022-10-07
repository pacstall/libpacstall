//! Errors used by the caching system

use thiserror::Error;

/// Errors used by Base
#[derive(Debug, Clone, Error)]
pub enum StoreError {
    #[error("the provided query yielded no results")]
    NoQueryMatch,

    #[error("repository '{0}' could not be found")]
    RepositoryNotFound(String),

    #[error("pacbuild '{name:?}' could not be found in repository {repository:?}")]
    PacBuildNotFound { name: String, repository: String },

    #[error("repository '{0}' already exists")]
    RepositoryConflict(String),

    #[error("pacbuild '{name:?}' already exists in repository {repository:?}")]
    PacBuildConflict { name: String, repository: String },

    #[error("unexpected error: {0}")]
    Unexpected(String),

    #[error("multiple errors: {0:?}")]
    Aggregate(Vec<StoreError>),
}
