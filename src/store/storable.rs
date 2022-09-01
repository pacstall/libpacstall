use crate::model::{PacBuild, Repository};
use crate::store::filters::{InstallState, Kind};
use crate::store::StoreError;

pub type UnitStoreResult = Result<(), StoreError>;

pub trait Storable {
    fn remove_pacbuild(&mut self, name: &str, repository_url: &str) -> UnitStoreResult;
    fn add_pacbuild(&mut self, pacbuild: PacBuild, repository_url: &str) -> UnitStoreResult;
    fn update_pacbuild(&mut self, pacbuild: PacBuild, repository_url: &str) -> UnitStoreResult;

    fn remove_all_pacbuilds(&mut self, name: Vec<&str>, repository_url: &str) -> UnitStoreResult;
    fn add_all_pacbuilds(
        &mut self,
        pacbuilds: Vec<PacBuild>,
        repository_url: &str,
    ) -> UnitStoreResult;
    fn update_all_pacbuilds(
        &mut self,
        pacbuilds: Vec<PacBuild>,
        repository_url: &str,
    ) -> UnitStoreResult;

    fn remove_repository(&mut self, repository_url: &str) -> UnitStoreResult;
    fn add_repository(&mut self, repository: Repository) -> UnitStoreResult;
    fn update_repository(&mut self, repository: Repository) -> UnitStoreResult;

    fn get_pacbuild_by_name_and_url(&self, name: &str, repository_url: &str) -> Option<&PacBuild>;
    fn get_repository_by_name(&self, name: &str) -> Option<&Repository>;
    fn get_repository_by_url(&self, url: &str) -> Option<&Repository>;

    fn get_all_pacbuilds_by(
        &self,
        name_like: Option<&str>,
        install_state: Option<InstallState>,
        kind: Option<Kind>,
        repository_url: Option<&str>,
    ) -> Vec<&PacBuild>;
}

impl dyn Storable {
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
