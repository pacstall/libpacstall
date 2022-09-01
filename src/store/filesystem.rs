use std::collections::HashMap;

use super::StoreError;
use crate::model::{PacBuild, Repository};
use crate::store::filters::{InstallState, Kind};
use crate::store::storable::{Storable, UnitStoreResult};
pub struct FileSystemStore {
    repositories: Vec<Repository>,
    packages: HashMap<String, Vec<PacBuild>>,

    allow_data_save: bool,
}

impl FileSystemStore {
    pub fn new() -> Box<dyn Storable> {
        Box::new(FileSystemStore {
            repositories: vec![],
            packages: HashMap::new(),
            allow_data_save: true,
        })
    }

    fn get_packages_by_repository(
        &self,
        repository_url: &str,
    ) -> Result<&Vec<PacBuild>, StoreError> {
        self.packages.get(&repository_url.to_owned()).map_or_else(
            || {
                Err(StoreError::new(
                    format!("Repository \"{}\" does not exist.", repository_url).as_str(),
                ))
            },
            |it| Ok(it),
        )
    }

    fn save_to_disk(&self) {
        if self.allow_data_save {
            todo!()
        }
    }
}

impl Storable for FileSystemStore {
    fn get_pacbuild_by_name_and_url(&self, name: &str, url: &str) -> Option<&PacBuild> {
        self.packages
            .iter()
            .filter(|(repo_url, _)| (*repo_url).to_owned() == url.to_owned())
            .flat_map(|(_, pkgs)| pkgs)
            .find(|p| p.name == name.to_owned())
    }

    fn get_repository_by_name(&self, name: &str) -> Option<&Repository> {
        self.repositories
            .iter()
            .find(|repo| repo.name == name.to_owned())
    }

    fn get_repository_by_url(&self, url: &str) -> Option<&Repository> {
        self.repositories
            .iter()
            .find(|repo| repo.url == url.to_owned())
    }

    fn get_all_pacbuilds_by(
        &self,
        name_like: Option<&str>,
        install_state: Option<InstallState>,
        kind: Option<Kind>,
        repository_url: Option<&str>,
    ) -> Vec<&PacBuild> {
        let repos_urls = if let Some(url) = repository_url {
            self.repositories
                .iter()
                .find(|it| it.url == url.to_string())
                .map_or_else(|| vec![], |it| vec![it.url.to_owned()])
        } else {
            self.repositories
                .iter()
                .map(|it| it.url.to_owned())
                .collect()
        };

        self.packages
            .iter()
            .filter(|(repo_url, _)| repos_urls.contains(repo_url))
            .flat_map(|(_, pkgs)| pkgs)
            .filter(|it| {
                if let Some(kind_filter) = &kind {
                    kind_filter.to_owned() == Kind::from_model_kind(it.kind.clone())
                } else {
                    false
                }
            })
            .filter(|it| {
                if let Some(install_state_filter) = &install_state {
                    install_state_filter.to_owned()
                        == InstallState::from_model_install_state(it.install_state.clone())
                } else {
                    false
                }
            })
            .filter(|it| {
                if let Some(name_like) = name_like {
                    it.name.contains(name_like)
                } else {
                    false
                }
            })
            .collect()
    }

    fn remove_pacbuild(&mut self, name: &str, repository_url: &str) -> Result<(), StoreError> {
        let new_list = self
            .get_packages_by_repository(repository_url)?
            .iter()
            .filter(|it| it.name != name.to_owned())
            .map(|it| it.clone())
            .collect::<Vec<PacBuild>>();

        self.packages.insert(repository_url.to_owned(), new_list);

        self.save_to_disk();
        Ok(())
    }

    fn add_pacbuild(&mut self, pacbuild: PacBuild, repository_url: &str) -> UnitStoreResult {
        let mut new_list = self.get_packages_by_repository(repository_url)?.to_owned();

        new_list.push(pacbuild.clone());
        self.packages.insert(repository_url.to_owned(), new_list);

        self.save_to_disk();

        Ok(())
    }

    fn update_pacbuild(&mut self, pacbuild: PacBuild, repository_url: &str) -> UnitStoreResult {
        let new_list = self
            .get_packages_by_repository(repository_url)?
            .iter()
            .map(|it| {
                if it.name == pacbuild.name.to_owned() {
                    pacbuild.clone()
                } else {
                    it.clone()
                }
            })
            .collect();

        self.packages.insert(repository_url.to_owned(), new_list);
        self.save_to_disk();

        Ok(())
    }

    fn remove_all_pacbuilds(
        &mut self,
        names: Vec<&str>,
        repository_url: &str,
    ) -> Result<(), StoreError> {
        let str_names: Vec<String> = names.iter().map(|name| name.to_string()).collect();

        let new_list: Vec<PacBuild> = self
            .get_packages_by_repository(repository_url)?
            .to_owned()
            .into_iter()
            .filter(|it| str_names.contains(&it.name))
            .collect();

        self.packages.insert(repository_url.to_owned(), new_list);
        self.save_to_disk();

        Ok(())
    }

    fn add_all_pacbuilds(
        &mut self,
        pacbuilds: Vec<PacBuild>,
        repository_url: &str,
    ) -> UnitStoreResult {
        let mut new_list: Vec<PacBuild> =
            self.get_packages_by_repository(repository_url)?.to_owned();

        let already_existing_pkgs: Vec<&PacBuild> = pacbuilds
            .iter()
            .filter(|it| {
                self.get_pacbuild_by_name_and_url(it.name.as_str(), &repository_url)
                    .is_some()
            })
            .collect();

        if !already_existing_pkgs.is_empty() {
            return Err(StoreError::new(
                format!(
                    "The following PACBUILDs already exist: {:#?}",
                    already_existing_pkgs
                )
                .as_str(),
            ));
        }

        let mut to_add = pacbuilds.to_owned();
        new_list.append(&mut to_add);
        self.packages.insert(repository_url.to_owned(), new_list);
        self.save_to_disk();

        Ok(())
    }

    fn update_all_pacbuilds(
        &mut self,
        pacbuilds: Vec<PacBuild>,
        repository_url: &str,
    ) -> UnitStoreResult {
        self.allow_data_save = false;
        let errors: Vec<UnitStoreResult> = pacbuilds
            .iter()
            .map(|it| self.update_pacbuild(it.to_owned(), repository_url))
            .filter(|it| it.is_err())
            .collect();

        self.allow_data_save = true;

        if errors.is_empty() {
            self.save_to_disk();
            Ok(())
        } else {
            let e = errors.first().unwrap().clone().expect_err("unreachable");
            Err(StoreError::new(e.message.as_str()))
        }
    }

    fn remove_repository(&mut self, repository_url: &str) -> Result<(), StoreError> {
        let repo_exists = self
            .repositories
            .iter()
            .any(|it| it.url.as_str() == repository_url);

        if !repo_exists {
            return Err(StoreError::new(
                format!("Repository {} does not exist.", repository_url).as_str(),
            ));
        }

        self.repositories = self
            .repositories
            .iter()
            .filter(|repo| repo.url != repository_url)
            .map(|it| it.to_owned())
            .collect();

        self.packages.remove(&repository_url.to_owned());
        self.save_to_disk();

        Ok(())
    }

    fn add_repository(&mut self, repository: Repository) -> Result<(), StoreError> {
        let repo_exists = self.repositories.iter().any(|it| it.url == repository.url);

        if repo_exists {
            return Err(StoreError::new(
                format!("Repository {} already exists.", repository.url).as_str(),
            ));
        }

        self.packages.insert(repository.url.clone(), Vec::new());
        self.repositories.push(repository);
        self.save_to_disk();

        Ok(())
    }

    fn update_repository(&mut self, repository: Repository) -> UnitStoreResult {
        let repo_exists = self
            .repositories
            .iter()
            .any(|it| it.url == repository.url.to_owned());

        if !repo_exists {
            return Err(StoreError::new(
                format!("Repository {} does not exist.", repository.url).as_str(),
            ));
        }

        self.repositories = self
            .repositories
            .iter()
            .map(|it| {
                if it.url == repository.url.to_owned() {
                    repository.to_owned()
                } else {
                    it.to_owned()
                }
            })
            .collect();

        self.save_to_disk();

        Ok(())
    }
}
