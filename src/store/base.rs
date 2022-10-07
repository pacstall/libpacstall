//! Abstraction over the caching implementation

use std::collections::HashMap;
use std::fmt::Debug;

use serde::{Deserialize, Serialize};

use super::query_builder::{MutationQuery, PacBuildQuery, Query, RepositoryQuery};
use crate::model::{PacBuild, Repository};
use crate::store::errors::StoreError;

/// Alias for store error results
pub type StoreResult<T> = Result<T, StoreError>;

/// Path of the database.
#[cfg(not(test))]
const FSS_PATH: &str = "/etc/pacstall/fss.json";

/// Path of the database.
#[cfg(test)]
const FSS_PATH: &str = "./fss.json";

/// `FileSystem` implementation for the caching system
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Store {
    repositories: Vec<Repository>,
    packages: HashMap<String, Vec<PacBuild>>,
}

impl Store {
    /// # Errors
    pub fn load() -> Result<Self, StoreError> {
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
        let obj: Self = serde_json::from_str(&contents).map_or_else(
            |_| {
                Err(StoreError::Unexpected(
                    "Unable to deserialize database.".to_string(),
                ))
            },
            Ok,
        )?;

        Ok(obj)
    }

    /// # Private
    fn save_to_disk(&self) -> StoreResult<()> {
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

        fs::write(Path::new(FSS_PATH), &json).map_or_else(
            |_| {
                Err(StoreError::Unexpected(
                    "Unable to write database to disk.".to_string(),
                ))
            },
            |_| Ok(()),
        )
    }
}

impl Store {
    pub fn query_packages<F, R>(&self, handler: F) -> R
    where
        F: Fn(Box<dyn Query<PacBuild, PacBuildQuery>>) -> R,
    {
        let query_resolver = Box::new(PacBuildQueryResolver {
            packages: self.packages.clone(),
            repositories: self.repositories.clone(),
        });

        handler(query_resolver)
    }

    pub fn query_repositories<F, R>(&self, handler: F) -> R
    where
        F: Fn(Box<dyn Query<Repository, RepositoryQuery>>) -> R,
    {
        let query_resolver = Box::new(RepositoryQueryResolver {
            packages: self.packages.clone(),
            repositories: self.repositories.clone(),
        });

        handler(query_resolver)
    }

    /// # Errors
    pub fn mutate_packages<F, R>(&mut self, mut handler: F) -> StoreResult<R>
    where
        F: FnMut(&mut dyn MutationQuery<PacBuild, PacBuildQuery>) -> StoreResult<R>,
    {
        let mut query_resolver = PacBuildQueryResolver {
            packages: self.packages.clone(),
            repositories: self.repositories.clone(),
        };

        let res = handler(&mut query_resolver);
        self.packages = query_resolver.packages;
        self.repositories = query_resolver.repositories;
        self.save_to_disk()?;

        res
    }

    /// # Errors
    pub fn mutate_repositories<F, R>(&mut self, mut handler: F) -> StoreResult<R>
    where
        F: FnMut(&mut dyn MutationQuery<Repository, RepositoryQuery>) -> StoreResult<R>,
    {
        let mut query_resolver = RepositoryQueryResolver {
            packages: self.packages.clone(),
            repositories: self.repositories.clone(),
        };

        let res = handler(&mut query_resolver);
        self.packages = query_resolver.packages;
        self.repositories = query_resolver.repositories;
        self.save_to_disk()?;

        res
    }
}

struct PacBuildQueryResolver {
    pub(super) repositories: Vec<Repository>,
    pub(super) packages: HashMap<String, Vec<PacBuild>>,
}

struct RepositoryQueryResolver {
    pub(super) repositories: Vec<Repository>,
    pub(super) packages: HashMap<String, Vec<PacBuild>>,
}

impl Query<Repository, RepositoryQuery> for RepositoryQueryResolver {
    fn single(&self, query: RepositoryQuery) -> Option<Repository> {
        let all = self.find(query);
        all.first().cloned()
    }

    fn find(&self, query: RepositoryQuery) -> Vec<Repository> {
        self.repositories
            .clone()
            .into_iter()
            .filter(|it| query.matches(it))
            .collect()
    }

    fn page(&self, query: RepositoryQuery, page_no: usize, page_size: usize) -> Vec<Repository> {
        let start_idx = page_no * page_size;
        let mut end_idx = start_idx + page_size;

        let found = self.find(query);

        if start_idx > found.len() - 1 {
            return Vec::new();
        }

        if found.len() < end_idx {
            end_idx = found.len();
        }

        found[start_idx..end_idx].to_vec()
    }
}

impl MutationQuery<Repository, RepositoryQuery> for RepositoryQueryResolver {
    fn insert(&mut self, entity: Repository) -> StoreResult<()> {
        let found = self.single(
            RepositoryQuery::select_all()
                .where_name(entity.name.as_str().into())
                .where_url(entity.url.as_str().into()),
        );

        if found.is_some() {
            return Err(StoreError::RepositoryConflict(entity.url));
        }

        self.repositories.push(entity);

        Ok(())
    }

    fn update(&mut self, entity: Repository) -> StoreResult<()> {
        let repo =
            self.single(RepositoryQuery::select_all().where_url(entity.name.as_str().into()));

        if repo.is_none() {
            return Err(StoreError::RepositoryNotFound(entity.url));
        }

        let found = repo.unwrap();
        self.repositories.swap_remove(
            self.repositories
                .iter()
                .position(|it| it.url == found.url)
                .unwrap(),
        );
        self.repositories.push(entity);

        Ok(())
    }

    fn remove(&mut self, query: RepositoryQuery) -> StoreResult<()> {
        let to_remove: _ = self
            .repositories
            .clone()
            .into_iter()
            .filter(|it| query.matches(it))
            .collect::<Vec<Repository>>();

        if to_remove.is_empty() {
            return Err(StoreError::NoQueryMatch);
        }

        let new_repos: Vec<Repository> = self
            .repositories
            .clone()
            .into_iter()
            .filter(|it| !query.matches(it))
            .collect();

        self.repositories = new_repos;

        if let Some(clause) = query.url {
            for repo in to_remove {
                if clause.matches(&repo.url) {
                    self.packages.remove(&repo.url);
                }
            }
        }

        Ok(())
    }
}

impl Query<PacBuild, PacBuildQuery> for PacBuildQueryResolver {
    fn single(&self, query: PacBuildQuery) -> Option<PacBuild> {
        let all = self.find(query);
        all.first().cloned()
    }

    fn find(&self, query: PacBuildQuery) -> Vec<PacBuild> {
        self.packages
            .clone()
            .into_iter()
            .flat_map(|(_, it)| it)
            .filter(|it| query.matches(it))
            .collect()
    }

    fn page(&self, query: PacBuildQuery, page_no: usize, page_size: usize) -> Vec<PacBuild> {
        let start_idx = page_no * page_size;
        let mut end_idx = start_idx + page_size;

        let found = self.find(query);

        if start_idx > found.len() - 1 {
            return Vec::new();
        }

        if found.len() < end_idx {
            end_idx = found.len();
        }

        found[start_idx..end_idx].to_vec()
    }
}

impl MutationQuery<PacBuild, PacBuildQuery> for PacBuildQueryResolver {
    fn insert(&mut self, pacbuild: PacBuild) -> StoreResult<()> {
        if !self
            .repositories
            .iter()
            .any(|it| it.url == pacbuild.repository)
        {
            return Err(StoreError::RepositoryNotFound(pacbuild.repository.clone()));
        }

        let found = self.single(
            PacBuildQuery::select_all()
                .where_name(pacbuild.name.as_str().into())
                .where_repository_url(pacbuild.repository.as_str().into()),
        );

        if found.is_some() {
            return Err(StoreError::PacBuildConflict {
                name: pacbuild.name.clone(),
                repository: pacbuild.repository.clone(),
            });
        }

        if let Some(packages) = self.packages.get_mut(&pacbuild.repository) {
            packages.push(pacbuild);
        } else {
            self.packages
                .insert(pacbuild.repository.clone(), vec![pacbuild]);
        }

        Ok(())
    }

    fn update(&mut self, pacbuild: PacBuild) -> StoreResult<()> {
        if !self
            .repositories
            .iter()
            .any(|it| it.url == pacbuild.repository)
        {
            return Err(StoreError::RepositoryNotFound(pacbuild.repository));
        }

        let found = self.single(
            PacBuildQuery::select_all()
                .where_name(pacbuild.name.as_str().into())
                .where_repository_url(pacbuild.repository.as_str().into()),
        );

        if found.is_none() {
            return Err(StoreError::PacBuildNotFound {
                name: pacbuild.name,
                repository: pacbuild.repository,
            });
        }

        let pkg = found.unwrap();
        let repo = self.packages.get_mut(&pkg.repository).unwrap();
        let pos = repo.iter().position(|it| it.name == pkg.name).unwrap();
        repo.remove(pos);
        repo.push(pacbuild);

        Ok(())
    }

    fn remove(&mut self, query: PacBuildQuery) -> StoreResult<()> {
        let mut did_remove = false;
        for packages in &mut self.packages.values_mut() {
            let pkgs: _ = packages
                .iter()
                .cloned()
                .filter(|it| !query.matches(it))
                .collect::<Vec<PacBuild>>();

            if packages.len() != pkgs.len() {
                did_remove = true;
            }

            *packages = pkgs;
        }

        if did_remove {
            Ok(())
        } else {
            Err(StoreError::NoQueryMatch)
        }
    }
}

#[cfg(test)]
mod test {
    use super::Store;
    use crate::model::Repository;
    use crate::store::filters::{InstallState, Kind};
    use crate::store::query_builder::{PacBuildQuery, RepositoryQuery, StringClause};

    mod util {
        use chrono::NaiveDateTime;

        use crate::model::{InstallState, Kind, PacBuild, Repository};
        use crate::store::base::Store;

        pub fn create_store_with_sample_data() -> (Store, Repository, PacBuild) {
            let mut fss = Store::default();
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

            fss.mutate_repositories(|store| store.insert(repo.clone()))
                .unwrap();
            fss.mutate_packages(|store| store.insert(pacbuild_to_add.clone()))
                .unwrap();

            (fss, repo, pacbuild_to_add)
        }
    }

    #[test]
    fn new_creates_empty_fs_store() {
        let fss = Store::default();
        let pacbuilds = fss.query_packages(|store| store.find(PacBuildQuery::select_all()));
        let repos = fss.query_repositories(|store| store.find(RepositoryQuery::select_all()));

        assert_eq!(pacbuilds.len(), 0);
        assert_eq!(repos.len(), 0);
    }

    #[test]
    fn add_repository_works() {
        let mut fss = Store::default();

        fss.mutate_repositories(|store| store.insert(Repository::default()))
            .unwrap();
        let repos = fss.query_repositories(|store| store.find(RepositoryQuery::select_all()));

        assert_eq!(repos.len(), 1);
    }

    #[test]
    fn get_repository_by_name_works() {
        let mut fss = Store::default();
        let repo = Repository::default();

        fss.mutate_repositories(|store| store.insert(repo.clone()))
            .unwrap();
        let found_repo = fss
            .query_repositories(|store| {
                store.single(RepositoryQuery::select_all().where_name(repo.name.as_str().into()))
            })
            .unwrap();

        assert_eq!(repo, found_repo);
    }

    #[test]
    fn get_repository_by_url_works() {
        let mut fss = Store::default();
        let repo = Repository::default();

        fss.mutate_repositories(|store| store.insert(repo.clone()))
            .unwrap();
        let found_repo = fss
            .query_repositories(|store| {
                store.single(RepositoryQuery::select_all().where_url(repo.url.as_str().into()))
            })
            .unwrap();

        assert_eq!(repo, found_repo);
    }

    #[test]
    fn add_pacbuild_works() {
        let (fss, ..) = util::create_store_with_sample_data();
        let pbs = fss.query_packages(|store| store.find(PacBuildQuery::select_all()));

        println!("{:#?}", pbs);

        assert_eq!(pbs.len(), 1);
    }

    #[test]
    fn get_pacbuild_by_name_and_url_works() {
        let (fss, _, pacbuild) = util::create_store_with_sample_data();
        let found = fss
            .query_packages(|store| {
                store.single(
                    PacBuildQuery::select_all()
                        .where_name(pacbuild.name.as_str().into())
                        .where_repository_url(pacbuild.repository.as_str().into()),
                )
            })
            .unwrap();

        assert_eq!(found, pacbuild);
    }

    #[test]
    fn get_all_pacbuilds_works() {
        let (fss, ..) = util::create_store_with_sample_data();
        let found = fss.query_packages(|store| store.find(PacBuildQuery::select_all()));

        assert_eq!(found.len(), 1);
    }

    #[test]
    fn get_all_pacbuilds_by_name_like_works() {
        let (fss, _, pb) = util::create_store_with_sample_data();
        let found = fss.query_packages(|store| {
            store.find(
                PacBuildQuery::select_all().where_name(StringClause::Contains(pb.name.clone())),
            )
        });

        assert_eq!(found.len(), 1);
    }

    #[test]
    fn get_all_pacbuilds_by_name_like_works_when_no_results() {
        let (fss, ..) = util::create_store_with_sample_data();
        let found = fss.query_packages(|store| {
            store.find(
                PacBuildQuery::select_all().where_name(StringClause::Contains("blablabla".into())),
            )
        });

        assert_eq!(found.len(), 0);
    }

    #[test]
    fn get_all_pacbuilds_by_install_state_works() {
        let (fss, ..) = util::create_store_with_sample_data();
        let found = fss.query_packages(|store| {
            store.find(PacBuildQuery::select_all().where_install_state(InstallState::Direct))
        });

        assert_eq!(found.len(), 1);
    }

    #[test]
    fn get_all_pacbuilds_by_install_state_works_when_no_results() {
        let (fss, ..) = util::create_store_with_sample_data();
        let found = fss.query_packages(|store| {
            store.find(PacBuildQuery::select_all().where_install_state(InstallState::Indirect))
        });

        assert_eq!(found.len(), 0);
    }

    #[test]
    fn get_all_pacbuilds_by_kind_works() {
        let (fss, ..) = util::create_store_with_sample_data();
        let found = fss.query_packages(|store| {
            store.find(PacBuildQuery::select_all().where_kind(Kind::DebFile))
        });

        assert_eq!(found.len(), 1);
    }

    #[test]
    fn get_all_pacbuilds_by_kind_works_when_no_results() {
        let (fss, ..) = util::create_store_with_sample_data();
        let found = fss.query_packages(|store| {
            store.find(PacBuildQuery::select_all().where_kind(Kind::Binary))
        });

        assert_eq!(found.len(), 0);
    }

    #[test]
    fn get_all_pacbuilds_by_repository_url_works() {
        let (fss, repo, _) = util::create_store_with_sample_data();
        let found = fss.query_packages(|store| {
            store.find(PacBuildQuery::select_all().where_repository_url(repo.url.as_str().into()))
        });

        assert_eq!(found.len(), 1);
    }

    #[test]
    fn get_all_pacbuilds_by_repository_url_works_when_no_results() {
        let (fss, ..) = util::create_store_with_sample_data();
        let found = fss.query_packages(|store| {
            store.find(PacBuildQuery::select_all().where_repository_url("does not exist".into()))
        });

        assert_eq!(found.len(), 0);
    }

    #[test]
    fn update_pacbuild_works() {
        let (mut fss, _, mut pb) = util::create_store_with_sample_data();
        pb.description = "something else".into();

        fss.mutate_packages(|query| query.update(pb.clone()))
            .unwrap();

        let results = fss.query_packages(|query| {
            query.find(
                PacBuildQuery::select_all()
                    .where_name(pb.name.as_str().into())
                    .where_repository_url(pb.repository.as_str().into()),
            )
        });
        let found = results.first().unwrap();

        assert_eq!(pb, *found);
    }

    #[test]
    #[should_panic]
    fn update_pacbuild_panics_when_pacbuild_not_found() {
        let (mut fss, _, mut pb) = util::create_store_with_sample_data();
        pb.name = "lala".into();
        pb.description = "something else".into();

        fss.mutate_packages(|query| query.update(pb.clone()))
            .unwrap();
    }

    #[test]
    #[should_panic]
    fn remove_pacbuild_panics_when_pacbuild_not_found() {
        let (mut fss, ..) = util::create_store_with_sample_data();

        fss.mutate_packages(|query| {
            query.remove(PacBuildQuery::select_all().where_name("asd".into()))
        })
        .unwrap();
    }

    #[test]
    #[should_panic]
    fn add_pacbuild_panics_when_pacbuild_already_exists() {
        let (mut fss, _, pb) = util::create_store_with_sample_data();
        fss.mutate_packages(|query| query.insert(pb.clone()))
            .unwrap();
    }
}
