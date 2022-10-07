use super::base::StoreResult;
use super::filters::{InstallState, Kind};
use crate::model::{PacBuild, Repository};

pub trait Query<T, Q> {
    fn single(&self, query: Q) -> Option<T>;
    fn find(&self, query: Q) -> Vec<T>;
    fn page(&self, query: Q, page_no: usize, page_size: usize) -> Vec<T>;
}

pub trait MutationQuery<T, Q> {
    /// # Errors
    fn remove(&mut self, query: Q) -> StoreResult<()>;
    /// # Errors
    fn insert(&mut self, entity: T) -> StoreResult<()>;
    /// # Errors
    fn update(&mut self, entity: T) -> StoreResult<()>;
}

#[derive(Clone)]
pub enum QueryClause<T> {
    Not(T),
    And(Vec<T>),
    Or(Vec<T>),
}

#[derive(Clone)]
pub enum StringClause {
    Equals(String),
    StartsWith(String),
    EndsWith(String),
    Contains(String),
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

#[derive(Clone)]
pub struct PacBuildQuery {
    pub name: Option<StringClause>,
    pub install_state: Option<InstallState>,
    pub kind: Option<Kind>,
    pub repository_url: Option<StringClause>,
}

impl PacBuildQuery {
    pub fn matches(&self, pacbuild: &PacBuild) -> bool {
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

#[derive(Clone)]
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
    pub fn select_all() -> Self {
        RepositoryQuery {
            name: None,
            url: None,
        }
    }

    pub fn where_name(&self, name: StringClause) -> Self {
        let mut query = self.clone();
        query.name = Some(name);

        query
    }

    pub fn where_url(&self, url: StringClause) -> Self {
        let mut query = self.clone();
        query.url = Some(url);

        query
    }
}

#[allow(clippy::return_self_not_must_use)]
impl PacBuildQuery {
    pub fn select_all() -> Self {
        PacBuildQuery {
            name: None,
            install_state: None,
            kind: None,
            repository_url: None,
        }
    }

    pub fn where_name(&self, name: StringClause) -> Self {
        let mut query = self.clone();
        query.name = Some(name);

        query
    }

    pub fn where_install_state(&self, install_state: InstallState) -> Self {
        let mut query = self.clone();
        query.install_state = Some(install_state);

        query
    }

    pub fn where_kind(&self, kind: Kind) -> Self {
        let mut query = self.clone();
        query.kind = Some(kind);

        query
    }

    pub fn where_repository_url(&self, repository_url: StringClause) -> Self {
        let mut query = self.clone();
        query.repository_url = Some(repository_url);

        query
    }
}
