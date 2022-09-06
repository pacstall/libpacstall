//! Provides a JSON file-based implementation for the caching system.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::errors::StoreError;
use crate::model::{PacBuild, Repository};
use crate::store::base::{Base, StoreResult};
use crate::store::filters::{InstallState, Kind};

#[cfg(not(test))]
const FSS_PATH: &str = "/etc/pacstall/fss.json";
#[cfg(test)]
const FSS_PATH: &str = "./fss.json";

/// `FileSystem` implementation for the caching system
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FileSystemStore {
    repositories: Vec<Repository>,
    packages: HashMap<String, Vec<PacBuild>>,

    #[serde(skip_serializing)]
    allow_data_save: bool,
}

impl FileSystemStore {
    #[allow(clippy::new_ret_no_self)]
    pub fn new() -> Box<dyn Base> {
        Box::new(FileSystemStore {
            repositories: Vec::new(),
            packages: HashMap::new(),
            allow_data_save: true,
        })
    }

    /// # Private
    fn get_packages_by_repository(
        &mut self,
        repository_url: &str,
    ) -> Result<&mut Vec<PacBuild>, StoreError> {
        self.packages
            .get_mut(&repository_url.to_owned())
            .map_or_else(
                || Err(StoreError::RepositoryNotFound(repository_url.to_owned())),
                Ok,
            )
    }

    /// # Private
    fn save_to_disk(&self) -> StoreResult<()> {
        if self.allow_data_save {
            use std::fs;
            use std::path::Path;

            let json = serde_json::to_vec_pretty(self).map_or_else(
                |_| {
                    Err(StoreError::Unexpected(
                        "Unable to serialize database.".to_string(),
                    ))
                },
                Ok,
            )?;

            return fs::write(Path::new(FSS_PATH), &json).map_or_else(
                |_| {
                    Err(StoreError::Unexpected(
                        "Unable to write database to disk.".to_string(),
                    ))
                },
                |_| Ok(()),
            );
        }

        Ok(())
    }

    /// # Errors
    pub fn load_from_disk() -> Result<Box<dyn Base>, StoreError> {
        use std::fs;
        use std::path::Path;

        let contents = fs::read_to_string(Path::new(FSS_PATH)).map_or_else(
            |_| {
                Err(StoreError::Unexpected(
                    "Unable to read database from disk.".to_string(),
                ))
            },
            Ok,
        )?;
        let mut obj: FileSystemStore = serde_json::from_str(&contents).map_or_else(
            |_| {
                Err(StoreError::Unexpected(
                    "Unable to deserialize database.".to_string(),
                ))
            },
            Ok,
        )?;
        obj.allow_data_save = true;

        Ok(Box::new(obj))
    }
}

impl Base for FileSystemStore {
    fn get_pacbuild_by_name_and_url(&self, name: &str, url: &str) -> Option<&PacBuild> {
        self.packages
            .get(&url.to_string())
            .and_then(|pkgs| pkgs.iter().find(|it| it.name == *name))
    }

    fn get_repository_by_name(&self, name: &str) -> Option<&Repository> {
        self.repositories.iter().find(|repo| repo.name == *name)
    }

    fn get_repository_by_url(&self, url: &str) -> Option<&Repository> {
        self.repositories.iter().find(|repo| repo.url == *url)
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
                .find(|it| it.url == *url)
                .map_or_else(Vec::new, |it| vec![it.url.clone()])
        } else {
            self.repositories.iter().map(|it| it.url.clone()).collect()
        };

        self.packages
            .iter()
            .filter(|(repo_url, _)| repos_urls.contains(repo_url))
            .flat_map(|(_, pkgs)| pkgs)
            .filter(|it| {
                if let Some(kind_filter) = &kind {
                    *kind_filter == (&it.kind).into()
                } else {
                    true
                }
            })
            .filter(|it| {
                if let Some(install_state_filter) = &install_state {
                    *install_state_filter == (&it.install_state).into()
                } else {
                    true
                }
            })
            .filter(|it| {
                if let Some(name_like) = name_like {
                    it.name.contains(name_like)
                } else {
                    true
                }
            })
            .collect()
    }

    fn remove_pacbuild(&mut self, name: &str, repository_url: &str) -> Result<(), StoreError> {
        let repo = self
            .packages
            .get_mut(&repository_url.to_owned())
            .ok_or_else(|| StoreError::RepositoryNotFound(repository_url.to_string()))?;

        repo.swap_remove(repo.iter().position(|it| it.name == *name).ok_or(
            StoreError::PacBuildNotFound {
                name: name.to_string(),
                repository: repository_url.to_string(),
            },
        )?);

        self.save_to_disk()?;
        Ok(())
    }

    fn add_pacbuild(&mut self, pacbuild: PacBuild, repository_url: &str) -> StoreResult<()> {
        if self
            .get_pacbuild_by_name_and_url(&pacbuild.name, repository_url)
            .is_some()
        {
            return Err(StoreError::PacBuildConflict {
                name: pacbuild.name,
                repository: repository_url.to_string(),
            });
        }

        self.get_packages_by_repository(repository_url)?
            .push(pacbuild);
        self.save_to_disk()?;

        Ok(())
    }

    fn update_pacbuild(&mut self, pacbuild: PacBuild, repository_url: &str) -> StoreResult<()> {
        if self
            .get_pacbuild_by_name_and_url(&pacbuild.name, repository_url)
            .is_none()
        {
            return Err(StoreError::PacBuildNotFound {
                name: pacbuild.name,
                repository: repository_url.to_string(),
            });
        }

        let pkgs = self.get_packages_by_repository(repository_url)?;
        let idx = pkgs.iter().position(|pb| pb.name == pacbuild.name).unwrap();

        pkgs.swap_remove(idx);
        pkgs.push(pacbuild);
        self.save_to_disk()?;

        Ok(())
    }

    fn remove_all_pacbuilds(
        &mut self,
        names: &[&str],
        repository_url: &str,
    ) -> Result<(), StoreError> {
        let errors = names
            .iter()
            .map(
                |it| match self.get_pacbuild_by_name_and_url(*it, repository_url) {
                    Some(_) => None,
                    None => Some(StoreError::PacBuildNotFound {
                        name: (*it).to_string(),
                        repository: repository_url.to_string(),
                    }),
                },
            )
            .fold(vec![], |acc, it| {
                if let Some(err) = it {
                    let mut acc = acc;
                    acc.push(err);
                    acc
                } else {
                    acc
                }
            });

        if !errors.is_empty() {
            return Err(StoreError::Aggregate(errors));
        }

        let str_names: Vec<String> = names.iter().map(ToString::to_string).collect();

        let new_list: Vec<PacBuild> = self
            .get_packages_by_repository(repository_url)?
            .clone()
            .into_iter()
            .filter(|it| str_names.contains(&it.name))
            .collect();

        self.packages.insert(repository_url.to_owned(), new_list);
        self.save_to_disk()?;

        Ok(())
    }

    fn add_all_pacbuilds(
        &mut self,
        pacbuilds: Vec<PacBuild>,
        repository_url: &str,
    ) -> StoreResult<()> {
        let already_existing_pkgs: Vec<&PacBuild> = pacbuilds
            .iter()
            .filter(|it| {
                self.get_pacbuild_by_name_and_url(it.name.as_str(), repository_url)
                    .is_some()
            })
            .collect();

        if !already_existing_pkgs.is_empty() {
            return Err(StoreError::Aggregate(
                already_existing_pkgs
                    .iter()
                    .map(|it| StoreError::PacBuildConflict {
                        name: it.name.to_string(),
                        repository: it.repository.to_string(),
                    })
                    .collect(),
            ));
        }

        let mut new_list: Vec<PacBuild> = self.get_packages_by_repository(repository_url)?.clone();
        let mut to_add = pacbuilds.clone();
        new_list.append(&mut to_add);
        self.packages.insert(repository_url.to_owned(), new_list);
        self.save_to_disk()?;

        Ok(())
    }

    fn update_all_pacbuilds(
        &mut self,
        pacbuilds: Vec<PacBuild>,
        repository_url: &str,
    ) -> StoreResult<()> {
        for name in pacbuilds.iter().map(|it| &it.name) {
            if self
                .get_pacbuild_by_name_and_url(name, repository_url)
                .is_none()
            {
                return Err(StoreError::PacBuildNotFound {
                    name: name.to_string(),
                    repository: repository_url.to_string(),
                });
            }
        }

        self.allow_data_save = false;
        let errors: Vec<StoreResult<()>> = pacbuilds
            .iter()
            .map(|it| self.update_pacbuild(it.clone(), repository_url))
            .filter(Result::is_err)
            .collect();

        self.allow_data_save = true;

        if errors.is_empty() {
            self.save_to_disk()?;
            Ok(())
        } else {
            Err(StoreError::Aggregate(
                errors.iter().map(|it| it.clone().unwrap_err()).collect(),
            ))
        }
    }

    fn remove_repository(&mut self, repository_url: &str) -> Result<(), StoreError> {
        let repo_idx = self
            .repositories
            .iter()
            .position(|it| it.url.as_str() == repository_url)
            .ok_or_else(|| StoreError::RepositoryNotFound(repository_url.to_string()))?;

        self.repositories.swap_remove(repo_idx);
        self.packages.remove(&repository_url.to_owned());
        self.save_to_disk()?;

        Ok(())
    }

    fn add_repository(&mut self, repository: Repository) -> Result<(), StoreError> {
        let repo_exists = self.repositories.iter().any(|it| it.url == repository.url);

        if repo_exists {
            return Err(StoreError::RepositoryConflict(repository.url));
        }

        self.packages.insert(repository.url.clone(), Vec::new());
        self.repositories.push(repository);
        self.save_to_disk()?;

        Ok(())
    }

    fn update_repository(&mut self, repository: Repository) -> StoreResult<()> {
        let repo_idx = self
            .repositories
            .iter()
            .position(|it| it.url == repository.url)
            .ok_or_else(|| StoreError::RepositoryNotFound(repository.url.to_string()))?;

        self.repositories.swap_remove(repo_idx);
        self.repositories.push(repository);

        self.save_to_disk()?;

        Ok(())
    }

    fn get_all_repositories(&self) -> Vec<&Repository> { self.repositories.iter().collect() }
}

#[cfg(test)]
mod test {
    use super::FileSystemStore;
    use crate::model::Repository;

    mod util {
        use chrono::NaiveDateTime;

        use crate::model::{InstallState, Kind, PacBuild, Repository};
        use crate::store::base::Base;
        use crate::store::fs::FileSystemStore;

        pub fn create_store_with_sample_data() -> (Box<dyn Base>, Repository, PacBuild) {
            let mut fss = FileSystemStore::new();
            let repo = Repository::default();
            let pacbuild_to_add = PacBuild {
                name: "dummy-pacbuild-deb".into(),
                package_name: "dummy-pacbuild".into(),
                description: "blah".into(),
                dependencies: Vec::new(),
                homepage: "https://example.com".into(),
                install_state: InstallState::Direct(
                    NaiveDateTime::from_timestamp(chrono::Utc::now().timestamp(), 0),
                    "1.0.0".into(),
                ),
                kind: Kind::DebFile("hashash".into()),
                last_updated: NaiveDateTime::from_timestamp(chrono::Utc::now().timestamp(), 0),
                license: "BSD".into(),
                maintainer: "saenai255".into(),
                optional_dependencies: Vec::new(),
                repology: "filter".into(),
                repology_version: "1.0.1".into(),
                repository: repo.url.clone(),
                url: "https://example.com/dummy-pacbuild-1.0.0.deb".into(),
            };

            fss.add_repository(repo.clone()).unwrap();
            fss.add_pacbuild(pacbuild_to_add.clone(), &repo.url)
                .unwrap();

            (fss, repo, pacbuild_to_add)
        }
    }

    #[test]
    fn new_creates_empty_fs_store() {
        let fss = FileSystemStore::new();
        let pacbuilds = fss.get_all_pacbuilds();
        let repos = fss.get_all_repositories();

        assert_eq!(pacbuilds.len(), 0);
        assert_eq!(repos.len(), 0);
    }

    #[test]
    fn add_repository_works() {
        let mut fss = FileSystemStore::new();

        fss.add_repository(Repository::default()).unwrap();
        let repos = fss.get_all_repositories();

        assert_eq!(repos.len(), 1);
    }

    #[test]
    fn get_repository_by_name_works() {
        let mut fss = FileSystemStore::new();
        let repo = Repository::default();

        fss.add_repository(repo.clone()).unwrap();
        let found_repo = fss.get_repository_by_name(&repo.name).unwrap();

        assert_eq!(repo, found_repo.clone());
    }

    #[test]
    fn get_repository_by_url_works() {
        let mut fss = FileSystemStore::new();
        let repo = Repository::default();

        fss.add_repository(repo.clone()).unwrap();
        let found_repo = fss.get_repository_by_url(&repo.url).unwrap();

        assert_eq!(repo, found_repo.clone());
    }

    #[test]
    fn add_pacbuild_works() {
        let (fss, ..) = util::create_store_with_sample_data();
        let pbs = fss.get_all_pacbuilds();

        println!("{:#?}", pbs);

        assert_eq!(pbs.len(), 1);
    }

    #[test]
    fn get_pacbuild_by_name_and_url_works() {
        let (fss, _, pacbuild) = util::create_store_with_sample_data();
        let found = fss
            .get_pacbuild_by_name_and_url(&pacbuild.name, &pacbuild.repository)
            .unwrap();

        assert_eq!(found.clone(), pacbuild);
    }

    #[test]
    fn get_all_pacbuilds_works() {
        let (fss, ..) = util::create_store_with_sample_data();
        let found = fss.get_all_pacbuilds();

        assert_eq!(found.len(), 1);
    }

    #[test]
    fn get_all_pacbuilds_by_name_like_works() {
        let (fss, _, pb) = util::create_store_with_sample_data();
        let found = fss.get_all_pacbuilds_by_name_like(&pb.name);

        assert_eq!(found.len(), 1);
    }

    #[test]
    fn get_all_pacbuilds_by_name_like_works_when_no_results() {
        let (fss, ..) = util::create_store_with_sample_data();
        let found = fss.get_all_pacbuilds_by_name_like("blalblala");

        assert_eq!(found.len(), 0);
    }

    #[test]
    fn get_all_pacbuilds_by_install_state_works() {
        let (fss, ..) = util::create_store_with_sample_data();
        let found =
            fss.get_all_pacbuilds_by_install_state(crate::store::filters::InstallState::Direct);

        assert_eq!(found.len(), 1);
    }

    #[test]
    fn get_all_pacbuilds_by_install_state_works_when_no_results() {
        let (fss, ..) = util::create_store_with_sample_data();
        let found =
            fss.get_all_pacbuilds_by_install_state(crate::store::filters::InstallState::Indirect);

        assert_eq!(found.len(), 0);
    }

    #[test]
    fn get_all_pacbuilds_by_kind_works() {
        let (fss, ..) = util::create_store_with_sample_data();
        let found = fss.get_all_pacbuilds_by_kind(crate::store::filters::Kind::DebFile);

        assert_eq!(found.len(), 1);
    }

    #[test]
    fn get_all_pacbuilds_by_kind_works_when_no_results() {
        let (fss, ..) = util::create_store_with_sample_data();
        let found = fss.get_all_pacbuilds_by_kind(crate::store::filters::Kind::Binary);

        assert_eq!(found.len(), 0);
    }

    #[test]
    fn get_all_pacbuilds_by_repository_url_works() {
        let (fss, repo, _) = util::create_store_with_sample_data();
        let found = fss.get_all_pacbuilds_by_repository_url(&repo.url);

        assert_eq!(found.len(), 1);
    }

    #[test]
    fn get_all_pacbuilds_by_repository_url_works_when_no_results() {
        let (fss, ..) = util::create_store_with_sample_data();
        let found = fss.get_all_pacbuilds_by_repository_url("this repo url does not exist");

        assert_eq!(found.len(), 0);
    }

    #[test]
    fn update_pacbuild_works() {
        let (mut fss, _, mut pb) = util::create_store_with_sample_data();
        pb.description = "something else".into();

        fss.update_pacbuild(pb.clone(), &pb.repository).unwrap();
        let found = fss
            .get_pacbuild_by_name_and_url(&pb.name, &pb.repository)
            .unwrap();

        assert_eq!(pb, *found);
    }

    #[test]
    fn update_all_pacbuilds_works() {
        let (mut fss, _, mut pb) = util::create_store_with_sample_data();
        pb.description = "something else".into();

        fss.update_all_pacbuilds(vec![pb.clone()], &pb.repository)
            .unwrap();
        let found = fss
            .get_pacbuild_by_name_and_url(&pb.name, &pb.repository)
            .unwrap();

        assert_eq!(pb, *found);
    }

    #[test]
    #[should_panic]
    fn update_pacbuild_panics_when_pacbuild_not_found() {
        let (mut fss, _, mut pb) = util::create_store_with_sample_data();
        pb.name = "lala".into();
        pb.description = "something else".into();

        fss.update_pacbuild(pb.clone(), &pb.repository).unwrap();
    }

    #[test]
    #[should_panic]
    fn update_all_pacbuilds_panics_when_pacbuild_not_found() {
        let (mut fss, _, mut pb) = util::create_store_with_sample_data();
        pb.name = "lala".into();
        pb.description = "something else".into();

        fss.update_all_pacbuilds(vec![pb.clone()], &pb.repository)
            .unwrap();
    }

    #[test]
    #[should_panic]
    fn remove_pacbuild_panics_when_pacbuild_not_found() {
        let (mut fss, _, pb) = util::create_store_with_sample_data();

        fss.remove_pacbuild("does-not-exist", &pb.repository)
            .unwrap();
    }

    #[test]
    #[should_panic]
    fn remove_all_pacbuilds_panics_when_pacbuild_not_found() {
        let (mut fss, _, pb) = util::create_store_with_sample_data();

        fss.remove_all_pacbuilds(&(&vec!["does-not-exist"])[..], &pb.repository)
            .unwrap();
    }

    #[test]
    #[should_panic]
    fn add_pacbuild_panics_when_pacbuild_already_exists() {
        let (mut fss, _, pb) = util::create_store_with_sample_data();
        fss.add_pacbuild(pb.clone(), &pb.repository).unwrap();
    }

    #[test]
    #[should_panic]
    fn update_all_pacbuilds_panics_when_pacbuild_already_exists() {
        let (mut fss, _, pb) = util::create_store_with_sample_data();
        fss.add_all_pacbuilds(vec![pb.clone()], &pb.repository)
            .unwrap();
    }
}
