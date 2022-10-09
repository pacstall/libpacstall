//! Provides query utilities for the cache store

use super::base::StoreResult;
use super::filters::{InstallState, Kind};
use crate::model::{PacBuild, Repository};

/// Defines the common methods for querying entities.
pub trait Queryable<T, Q> {
    /// Finds a single entity that matches the given query.
    fn single(&self, query: Q) -> Option<T>;

    /// Finds all entities that match the given query.
    fn find(&self, query: Q) -> Vec<T>;

    /// Finds a selection of entities that match the given query.
    fn page(&self, query: Q, page_no: usize, page_size: usize) -> Vec<T>;
}

/// Defines the common methods for mutating entities
pub trait Mutable<T, Q> {
    /// Removes all entities that match the given
    ///
    /// # Errors
    ///
    /// The following errors may occur:
    ///
    /// - [`StoreError`](crate::store::errors::StoreError) - Wrapper for all the
    ///   other [`Store`](crate::store::base::Store) errors
    /// - [`NoQueryMatchError`](crate::store::errors::NoQueryMatchError) - When
    ///   attempting to remove an entity that does not exist
    /// - [`IOError`](crate::store::errors::IOError) - When attempting database
    ///   export fails
    fn remove(&mut self, query: Q) -> StoreResult<()>;

    /// Inserts a single entity
    ///
    /// # Errors
    ///
    /// The following errors may occur:
    ///
    /// - [`StoreError`](crate::store::errors::StoreError) - Wrapper for all the
    ///   other [`Store`](crate::store::base::Store) errors
    /// - [`EntityNotFoundError`](crate::store::errors::EntityNotFoundError) -
    ///   When attempting to query an entity or related entity that does not
    ///   exist
    /// - [`EntityAlreadyExistsError`](crate::store::errors::EntityAlreadyExistsError) - When attempting insert an entity or related entity that already exists
    /// - [`IOError`](crate::store::errors::IOError) - When attempting database
    ///   export fails
    fn insert(&mut self, entity: T) -> StoreResult<()>;

    /// Removes all entities that match the given
    ///
    /// # Errors
    ///
    /// The following errors may occur:
    ///
    /// - [`StoreError`](crate::store::errors::StoreError) - Wrapper for all the
    ///   other [`Store`](crate::store::base::Store) errors
    /// - [`EntityNotFoundError`](crate::store::errors::EntityNotFoundError) -
    ///   When attempting to query a [`Repository`] or related entity that does
    ///   not exist
    /// - [`EntityAlreadyExistsError`](crate::store::errors::EntityAlreadyExistsError) - When attempting insert a [`Repository`] or related entity that already exists
    /// - [`IOError`](crate::store::errors::IOError) - When attempting database
    ///   export fails
    fn update(&mut self, entity: T) -> StoreResult<()>;
}

/// Represents a query utility for common verbs.
#[derive(Debug, Clone)]
pub enum QueryClause<T> {
    /// Represents logical `NOT`.
    Not(T),

    /// Represents logical `AND`.
    And(Vec<T>),

    /// Represents logical `OR`.
    Or(Vec<T>),
}

/// Represents a string query utility.
#[derive(Debug, Clone)]
pub enum StringClause {
    /// Equivalent of `==`.
    Equals(String),

    /// Matches all strings starting with the wrapped string.
    StartsWith(String),

    /// Matches all strings ending with the wrapped string.
    EndsWith(String),

    /// Matches all strings containing the wrapped string.
    Contains(String),

    /// Represents a list of query conditionals.
    Composite(Box<QueryClause<StringClause>>),
}

impl StringClause {
    pub fn matches(&self, value: &str) -> bool {
        match self {
            Self::Equals(it) => it == value,
            Self::Contains(it) => value.contains(it),
            Self::StartsWith(it) => value.starts_with(it),
            Self::EndsWith(it) => value.ends_with(it),
            Self::Composite(query) => match &**query {
                QueryClause::Not(str_clause) => !str_clause.matches(value),
                QueryClause::And(str_clauses) => str_clauses.iter().all(|it| it.matches(value)),
                QueryClause::Or(str_clauses) => str_clauses.iter().any(|it| it.matches(value)),
            },
        }
    }
}

impl From<String> for StringClause {
    fn from(it: String) -> Self { StringClause::Equals(it) }
}

impl From<&str> for StringClause {
    fn from(it: &str) -> Self { StringClause::Equals(String::from(it)) }
}

impl From<&String> for StringClause {
    fn from(it: &String) -> Self { StringClause::Equals(it.clone()) }
}

/// Query representation for [`PacBuild`]s.
#[derive(Debug, Clone)]
pub struct PacBuildQuery {
    pub name: Option<StringClause>,
    pub install_state: Option<InstallState>,
    pub kind: Option<Kind>,
    pub repository_url: Option<StringClause>,
}

impl PacBuildQuery {
    pub(super) fn matches(&self, pacbuild: &PacBuild) -> bool {
        if let Some(clause) = &self.name {
            if !clause.matches(&pacbuild.name) {
                return false;
            }
        }

        if let Some(clause) = &self.repository_url {
            if !clause.matches(&pacbuild.repository) {
                return false;
            }
        }

        if let Some(kind) = &self.kind {
            if kind != &Kind::from_model_kind(&pacbuild.kind.clone()) {
                return false;
            }
        }

        if let Some(install_state) = &self.install_state {
            if install_state
                != &InstallState::from_model_install_state(&pacbuild.install_state.clone())
            {
                return false;
            }
        }

        true
    }
}

/// Query representation for [`Repository`]s.
#[derive(Debug, Clone)]
pub struct RepositoryQuery {
    pub name: Option<StringClause>,
    pub url: Option<StringClause>,
}

impl RepositoryQuery {
    pub(super) fn matches(&self, repository: &Repository) -> bool {
        if let Some(clause) = &self.name {
            if !clause.matches(&repository.name) {
                return false;
            }
        }

        if let Some(clause) = &self.url {
            if !clause.matches(&repository.url) {
                return false;
            }
        }

        true
    }
}

#[allow(clippy::return_self_not_must_use)]
impl RepositoryQuery {
    /// Initializes the query.
    pub fn select() -> Self {
        RepositoryQuery {
            name: None,
            url: None,
        }
    }

    /// Adds a name clause.
    pub fn where_name(&self, name: StringClause) -> Self {
        let mut query = self.clone();
        query.name = Some(name);

        query
    }

    /// Adds a repository url clause.
    pub fn where_url(&self, url: StringClause) -> Self {
        let mut query = self.clone();
        query.url = Some(url);

        query
    }
}

#[allow(clippy::return_self_not_must_use)]
impl PacBuildQuery {
    /// Initializes the query.
    pub fn select() -> Self {
        PacBuildQuery {
            name: None,
            install_state: None,
            kind: None,
            repository_url: None,
        }
    }

    /// Adds a name clause.
    pub fn where_name(&self, name: StringClause) -> Self {
        let mut query = self.clone();
        query.name = Some(name);

        query
    }

    /// Adds an [`InstallState`] clause.
    pub fn where_install_state(&self, install_state: InstallState) -> Self {
        let mut query = self.clone();
        query.install_state = Some(install_state);

        query
    }

    /// Adds a [`Kind`] clause.
    pub fn where_kind(&self, kind: Kind) -> Self {
        let mut query = self.clone();
        query.kind = Some(kind);

        query
    }

    /// Adds a repository url clause.
    pub fn where_repository_url(&self, repository_url: StringClause) -> Self {
        let mut query = self.clone();
        query.repository_url = Some(repository_url);

        query
    }
}
