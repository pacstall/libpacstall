//! Abstraction over the caching implementation

use std::fmt::Debug;

use crate::model::{PacBuild, Repository};
use crate::store::errors::StoreError;
use crate::store::filters::{InstallState, Kind};

/// Alias for store error results
pub type StoreResult<T> = Result<T, StoreError>;

/// Abstraction over the caching implementation
pub trait Base: Debug {
    /// Removes `PacBuild` by name that belongs to the given repository.
    ///
    /// # Errors
    /// * `StoreError::RepositoryNotFound`
    /// * `StoreError::PacBuildNotFound`
    fn remove_pacbuild(&mut self, name: &str, repository_url: &str) -> StoreResult<()>;

    /// Adds `PacBuild` to the given repository.
    ///
    /// # Errors
    /// * `StoreError::RepositoryConflict`
    /// * `StoreError::PacBuildConflict`
    fn add_pacbuild(&mut self, pacbuild: PacBuild, repository_url: &str) -> StoreResult<()>;

    /// Updates `PacBuild` that belongs to the given repository.
    ///
    /// # Errors
    /// * `StoreError::RepositoryNotFound`
    /// * `StoreError::PacBuildNotFound`
    fn update_pacbuild(&mut self, pacbuild: PacBuild, repository_url: &str) -> StoreResult<()>;

    /// Removes all `PacBuild` by name that belongs to the given repository.
    ///
    /// # Errors
    /// * `StoreError::Aggregate`
    fn remove_all_pacbuilds(&mut self, name: &[&str], repository_url: &str) -> StoreResult<()>;

    /// Adds all `PacBuild` to the given repository.
    ///
    /// # Errors
    /// * `StoreError::Aggregate`
    fn add_all_pacbuilds(
        &mut self,
        pacbuilds: Vec<PacBuild>,
        repository_url: &str,
    ) -> StoreResult<()>;

    /// Updates all `PacBuild` that belongs to the given repository.
    ///
    /// # Errors
    /// * `StoreError::Aggregate`
    fn update_all_pacbuilds(
        &mut self,
        pacbuilds: Vec<PacBuild>,
        repository_url: &str,
    ) -> StoreResult<()>;

    /// Removes [Repository] by url.
    ///
    /// # Errors
    /// * `StoreError::RepositoryNotFound`
    fn remove_repository(&mut self, repository_url: &str) -> StoreResult<()>;

    /// Adds `Repository`.
    ///
    /// # Errors
    /// * `StoreError::RepositoryConflict`
    fn add_repository(&mut self, repository: Repository) -> StoreResult<()>;

    /// Updates [Repository].
    ///
    /// # Errors
    /// * `StoreError::RepositoryConflict`
    fn update_repository(&mut self, repository: Repository) -> StoreResult<()>;

    /// Find first by name in the given repository
    fn get_pacbuild_by_name_and_url(&self, name: &str, repository_url: &str) -> Option<&PacBuild>;

    /// Find repository by name
    fn get_repository_by_name(&self, name: &str) -> Option<&Repository>;
    /// Find repository by url
    fn get_repository_by_url(&self, url: &str) -> Option<&Repository>;

    /// Find all repositories
    fn get_all_repositories(&self) -> Vec<&Repository>;

    /// Find all packages that match all the given params. `None` params are
    /// skipped.
    fn get_all_pacbuilds_by(
        &self,
        name_like: Option<&str>,
        install_state: Option<InstallState>,
        kind: Option<Kind>,
        repository_url: Option<&str>,
    ) -> Vec<&PacBuild>;
}

impl dyn Base {
    /// Find all pacbuilds from all repositories
    pub fn get_all_pacbuilds(&self) -> Vec<&PacBuild> {
        self.get_all_pacbuilds_by(None, None, None, None)
    }

    pub fn get_all_pacbuilds_by_name_like(&self, name_like: &str) -> Vec<&PacBuild> {
        self.get_all_pacbuilds_by(Some(name_like), None, None, None)
    }

    pub fn get_all_pacbuilds_by_name_like_and_kind(
        &self,
        name_like: &str,
        kind: Kind,
    ) -> Vec<&PacBuild> {
        self.get_all_pacbuilds_by(Some(name_like), None, Some(kind), None)
    }

    pub fn get_all_pacbuilds_by_name_like_and_install_state(
        &self,
        name_like: &str,
        install_state: InstallState,
    ) -> Vec<&PacBuild> {
        self.get_all_pacbuilds_by(Some(name_like), Some(install_state), None, None)
    }

    pub fn get_all_pacbuilds_by_name_like_and_repository_url(
        &self,
        name_like: &str,
        url: &str,
    ) -> Vec<&PacBuild> {
        self.get_all_pacbuilds_by(Some(name_like), None, None, Some(url))
    }

    pub fn get_all_pacbuilds_by_name_like_and_install_state_and_kind(
        &self,
        name_like: &str,
        install_state: InstallState,
        kind: Kind,
    ) -> Vec<&PacBuild> {
        self.get_all_pacbuilds_by(Some(name_like), Some(install_state), Some(kind), None)
    }

    pub fn get_all_pacbuilds_by_name_like_and_install_state_and_repository_url(
        &self,
        name_like: &str,
        install_state: InstallState,
        url: &str,
    ) -> Vec<&PacBuild> {
        self.get_all_pacbuilds_by(Some(name_like), Some(install_state), None, Some(url))
    }

    pub fn get_all_pacbuilds_by_name_like_and_install_state_and_kind_and_repository_url(
        &self,
        name_like: &str,
        install_state: InstallState,
        kind: Kind,
        url: &str,
    ) -> Vec<&PacBuild> {
        self.get_all_pacbuilds_by(Some(name_like), Some(install_state), Some(kind), Some(url))
    }

    pub fn get_all_pacbuilds_by_kind(&self, kind: Kind) -> Vec<&PacBuild> {
        self.get_all_pacbuilds_by(None, None, Some(kind), None)
    }

    pub fn get_all_pacbuilds_by_kind_and_install_state(
        &self,
        kind: Kind,
        install_state: InstallState,
    ) -> Vec<&PacBuild> {
        self.get_all_pacbuilds_by(None, Some(install_state), Some(kind), None)
    }

    pub fn get_all_pacbuilds_by_kind_and_repository_url(
        &self,
        kind: Kind,
        url: &str,
    ) -> Vec<&PacBuild> {
        self.get_all_pacbuilds_by(None, None, Some(kind), Some(url))
    }

    pub fn get_all_pacbuilds_by_kind_and_install_state_and_repository_url(
        &self,
        kind: Kind,
        install_state: InstallState,
        url: &str,
    ) -> Vec<&PacBuild> {
        self.get_all_pacbuilds_by(None, Some(install_state), Some(kind), Some(url))
    }

    pub fn get_all_pacbuilds_by_install_state(
        &self,
        install_state: InstallState,
    ) -> Vec<&PacBuild> {
        self.get_all_pacbuilds_by(None, Some(install_state), None, None)
    }

    pub fn get_all_pacbuilds_by_install_state_and_repository_url(
        &self,
        install_state: InstallState,
        url: &str,
    ) -> Vec<&PacBuild> {
        self.get_all_pacbuilds_by(None, Some(install_state), None, Some(url))
    }

    pub fn get_all_pacbuilds_by_repository_url(&self, url: &str) -> Vec<&PacBuild> {
        self.get_all_pacbuilds_by(None, None, None, Some(url))
    }
}
