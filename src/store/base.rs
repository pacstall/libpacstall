//! Abstraction over the caching implementation

use std::collections::HashMap;
use std::fmt::Debug;
use std::fs;
use std::path::Path;

use error_stack::{ensure, report, IntoReport, Result, ResultExt};
use serde::{Deserialize, Serialize};

use super::errors::{
    EntityAlreadyExistsError, EntityMutationError, EntityNotFoundError, IOError, NoQueryMatchError,
    StoreError,
};
use super::query_builder::{Mutable, PacBuildQuery, Queryable, RepositoryQuery};
use crate::model::{PacBuild, Repository};

/// Shorthand alias for [`Result<T, StoreError>`].
pub type StoreResult<T> = Result<T, StoreError>;

/// Path of the database.
#[cfg(not(test))]
const FSS_PATH: &str = "/etc/pacstall/fss.json";

/// Path of the database.
#[cfg(test)]
const FSS_PATH: &str = "./fss.json";

/// Store implementation for metadata caching.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Store {
    repositories: Vec<Repository>,
    packages: HashMap<String, Vec<PacBuild>>,

    #[serde(skip)]
    in_memory: bool,
}

impl Store {
    /// Loads the store from the disk.
    ///
    /// # Errors
    ///
    /// The following errors may occur:
    ///
    /// - [`StoreError`](crate::store::errors::StoreError) - Wrapper for all the
    ///   other [`Store`] errors
    /// - [`IOError`](crate::store::errors::IOError) - When attempting database
    ///   import fails
    pub fn load() -> StoreResult<Self> {
        let contents = fs::read_to_string(Path::new(FSS_PATH))
            .into_report()
            .attach_printable_lazy(|| format!("failed to read file {FSS_PATH:?}"))
            .change_context(IOError)
            .change_context(StoreError)?;

        let obj: Self = serde_json::from_str(&contents)
            .into_report()
            .attach_printable_lazy(|| {
                format!("failed to deserialize database contents: '{contents:?}'")
            })
            .change_context(IOError)
            .change_context(StoreError)?;

        Ok(obj)
    }

    pub fn in_memory() -> Self {
        Store {
            repositories: Vec::new(),
            packages: HashMap::new(),
            in_memory: true,
        }
    }

    /// # Private
    fn save_to_disk(&self) -> StoreResult<()> {
        if self.in_memory {
            return Ok(());
        }

        let json = serde_json::to_vec_pretty(self)
            .into_report()
            .attach_printable_lazy(|| "failed to serialize database".to_string())
            .change_context(IOError)
            .change_context(StoreError)?;

        fs::write(Path::new(FSS_PATH), &json)
            .into_report()
            .attach_printable_lazy(|| {
                format!("failed to write serialized database to {FSS_PATH:?}")
            })
            .change_context(IOError)
            .change_context(StoreError)?;

        Ok(())
    }
}

impl Store {
    /// Searches for [`PacBuild`]s based on the given query.
    pub fn query_pacbuilds<F, R>(&self, handler: F) -> R
    where
        F: Fn(Box<dyn Queryable<PacBuild, PacBuildQuery>>) -> R,
    {
        let query_resolver = Box::new(PacBuildQueryResolver {
            packages: self.packages.clone(),
            repositories: self.repositories.clone(),
        });

        handler(query_resolver)
    }

    /// Searches for [`Repository`]s based on the given query.
    pub fn query_repositories<F, R>(&self, handler: F) -> R
    where
        F: Fn(Box<dyn Queryable<Repository, RepositoryQuery>>) -> R,
    {
        let query_resolver = Box::new(RepositoryQueryResolver {
            packages: self.packages.clone(),
            repositories: self.repositories.clone(),
        });

        handler(query_resolver)
    }

    /// Mutates [`PacBuild`]s based on the given query.
    ///
    /// # Errors
    ///
    /// The following errors may occur:
    ///
    /// - [`StoreError`](crate::store::errors::StoreError) - Wrapper for all the
    ///   other [`Store`] errors
    /// - [`EntityNotFoundError`](crate::store::errors::EntityNotFoundError) -
    ///   When attempting to query a [`PacBuild`] or related entity that does
    ///   not exist
    /// - [`EntityAlreadyExistsError`](crate::store::errors::EntityAlreadyExistsError) - When attempting insert a [`PacBuild`] or related entity that already exists
    /// - [`NoQueryMatchError`](crate::store::errors::NoQueryMatchError) - When
    ///   attempting to remove a [`PacBuild`] that does not exist
    /// - [`IOError`](crate::store::errors::IOError) - When attempting database
    ///   export fails
    pub fn mutate_pacbuilds<F, R>(&mut self, mut handler: F) -> StoreResult<R>
    where
        F: FnMut(&mut dyn Mutable<PacBuild, PacBuildQuery>) -> StoreResult<R>,
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

    /// Mutates [`Repository`]s based on the given query.
    ///
    /// # Errors
    ///
    /// The following errors may occur:
    ///
    /// - [`StoreError`](crate::store::errors::StoreError) - Wrapper for all the
    ///   other [`Store`] errors
    /// - [`EntityNotFoundError`](crate::store::errors::EntityNotFoundError) -
    ///   When attempting to query a [`Repository`] or related entity that does
    ///   not exist
    /// - [`EntityAlreadyExistsError`](crate::store::errors::EntityAlreadyExistsError) - When attempting insert a [`Repository`] or related entity that already exists
    /// - [`NoQueryMatchError`](crate::store::errors::NoQueryMatchError) - When
    ///   attempting to remove a [`Repository`] that does not exist
    /// - [`IOError`](crate::store::errors::IOError) - When attempting database
    ///   export fails
    pub fn mutate_repositories<F, R>(&mut self, mut handler: F) -> StoreResult<R>
    where
        F: FnMut(&mut dyn Mutable<Repository, RepositoryQuery>) -> StoreResult<R>,
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

impl Queryable<Repository, RepositoryQuery> for RepositoryQueryResolver {
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

impl Mutable<Repository, RepositoryQuery> for RepositoryQueryResolver {
    fn insert(&mut self, entity: Repository) -> StoreResult<()> {
        let found = self.single(
            RepositoryQuery::select()
                .where_name(entity.name.as_str().into())
                .where_url(entity.url.as_str().into()),
        );

        ensure!(
            found.is_none(),
            report!(EntityAlreadyExistsError)
                .attach_printable(format!("repository '{entity:?}' already exists"))
                .change_context(EntityMutationError)
                .change_context(StoreError)
        );

        self.repositories.push(entity);

        Ok(())
    }

    fn update(&mut self, entity: Repository) -> StoreResult<()> {
        let repo = self.single(RepositoryQuery::select().where_url(entity.name.as_str().into()));

        ensure!(
            repo.is_some(),
            report!(EntityNotFoundError)
                .attach_printable(format!("repository '{entity:?}' does not exist"))
                .change_context(EntityMutationError)
                .change_context(StoreError)
        );

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
        let to_remove: Vec<Repository> = self
            .repositories
            .clone()
            .into_iter()
            .filter(|it| query.matches(it))
            .collect();

        ensure!(
            !to_remove.is_empty(),
            report!(NoQueryMatchError)
                .attach_printable(format!("query '{query:?}' found no results"))
                .change_context(EntityMutationError)
                .change_context(StoreError)
        );

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

impl Queryable<PacBuild, PacBuildQuery> for PacBuildQueryResolver {
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

impl Mutable<PacBuild, PacBuildQuery> for PacBuildQueryResolver {
    fn insert(&mut self, pacbuild: PacBuild) -> StoreResult<()> {
        ensure!(
            self.repositories
                .iter()
                .any(|it| it.url == pacbuild.repository),
            report!(EntityNotFoundError)
                .attach_printable(format!(
                    "repository of pacbuild {pacbuild:?} does not exist"
                ))
                .change_context(EntityMutationError)
                .change_context(StoreError)
        );

        let found = self.single(
            PacBuildQuery::select()
                .where_name(pacbuild.name.as_str().into())
                .where_repository_url(pacbuild.repository.as_str().into()),
        );

        ensure!(
            found.is_none(),
            report!(EntityAlreadyExistsError)
                .attach_printable(format!("pacbuild {found:?} already exists"))
                .change_context(EntityMutationError)
                .change_context(StoreError)
        );

        if let Some(packages) = self.packages.get_mut(&pacbuild.repository) {
            packages.push(pacbuild);
        } else {
            self.packages
                .insert(pacbuild.repository.clone(), vec![pacbuild]);
        }

        Ok(())
    }

    fn update(&mut self, pacbuild: PacBuild) -> StoreResult<()> {
        ensure!(
            self.repositories
                .iter()
                .any(|it| it.url == pacbuild.repository),
            report!(EntityNotFoundError)
                .attach_printable(format!(
                    "repository of pacbuild {pacbuild:?} does not exist"
                ))
                .change_context(EntityMutationError)
                .change_context(StoreError)
        );

        let found = self.single(
            PacBuildQuery::select()
                .where_name(pacbuild.name.as_str().into())
                .where_repository_url(pacbuild.repository.as_str().into()),
        );

        ensure!(
            found.is_some(),
            report!(EntityNotFoundError)
                .attach_printable(format!(
                    "repository of pacbuild {pacbuild:?} does not exist"
                ))
                .change_context(EntityMutationError)
                .change_context(StoreError)
        );

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
            let pkgs: Vec<PacBuild> = packages
                .iter()
                .cloned()
                .filter(|it| !query.matches(it))
                .collect();

            if packages.len() != pkgs.len() {
                did_remove = true;
            }

            *packages = pkgs;
        }

        ensure!(
            did_remove,
            report!(NoQueryMatchError)
                .attach_printable(format!("query {query:?} found no results"))
                .change_context(EntityMutationError)
                .change_context(StoreError)
        );

        Ok(())
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
            let mut fss = Store::in_memory();
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
            fss.mutate_pacbuilds(|store| store.insert(pacbuild_to_add.clone()))
                .unwrap();

            (fss, repo, pacbuild_to_add)
        }
    }

    #[test]
    fn new_creates_empty_fs_store() {
        let fss = Store::in_memory();
        let pacbuilds = fss.query_pacbuilds(|store| store.find(PacBuildQuery::select()));
        let repos = fss.query_repositories(|store| store.find(RepositoryQuery::select()));

        assert_eq!(pacbuilds.len(), 0);
        assert_eq!(repos.len(), 0);
    }

    #[test]
    fn add_repository_works() {
        let mut fss = Store::in_memory();

        fss.mutate_repositories(|store| store.insert(Repository::default()))
            .unwrap();
        let repos = fss.query_repositories(|store| store.find(RepositoryQuery::select()));

        assert_eq!(repos.len(), 1);
    }

    #[test]
    fn get_repository_by_name_works() {
        let mut fss = Store::in_memory();
        let repo = Repository::default();

        fss.mutate_repositories(|store| store.insert(repo.clone()))
            .unwrap();
        let found_repo = fss
            .query_repositories(|store| {
                store.single(RepositoryQuery::select().where_name(repo.name.as_str().into()))
            })
            .unwrap();

        assert_eq!(repo, found_repo);
    }

    #[test]
    fn get_repository_by_url_works() {
        let mut fss = Store::in_memory();
        let repo = Repository::default();

        fss.mutate_repositories(|store| store.insert(repo.clone()))
            .unwrap();
        let found_repo = fss
            .query_repositories(|store| {
                store.single(RepositoryQuery::select().where_url(repo.url.as_str().into()))
            })
            .unwrap();

        assert_eq!(repo, found_repo);
    }

    #[test]
    fn add_pacbuild_works() {
        let (fss, ..) = util::create_store_with_sample_data();
        let pbs = fss.query_pacbuilds(|store| store.find(PacBuildQuery::select()));

        println!("{:#?}", pbs);

        assert_eq!(pbs.len(), 1);
    }

    #[test]
    fn get_pacbuild_by_name_and_url_works() {
        let (fss, _, pacbuild) = util::create_store_with_sample_data();
        let found = fss
            .query_pacbuilds(|store| {
                store.single(
                    PacBuildQuery::select()
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
        let found = fss.query_pacbuilds(|store| store.find(PacBuildQuery::select()));

        assert_eq!(found.len(), 1);
    }

    #[test]
    fn get_all_pacbuilds_by_name_like_works() {
        let (fss, _, pb) = util::create_store_with_sample_data();
        let found = fss.query_pacbuilds(|store| {
            store.find(PacBuildQuery::select().where_name(StringClause::Contains(pb.name.clone())))
        });

        assert_eq!(found.len(), 1);
    }

    #[test]
    fn get_all_pacbuilds_by_name_like_works_when_no_results() {
        let (fss, ..) = util::create_store_with_sample_data();
        let found = fss.query_pacbuilds(|store| {
            store.find(
                PacBuildQuery::select().where_name(StringClause::Contains("blablabla".into())),
            )
        });

        assert_eq!(found.len(), 0);
    }

    #[test]
    fn get_all_pacbuilds_by_install_state_works() {
        let (fss, ..) = util::create_store_with_sample_data();
        let found = fss.query_pacbuilds(|store| {
            store.find(PacBuildQuery::select().where_install_state(InstallState::Direct))
        });

        assert_eq!(found.len(), 1);
    }

    #[test]
    fn get_all_pacbuilds_by_install_state_works_when_no_results() {
        let (fss, ..) = util::create_store_with_sample_data();
        let found = fss.query_pacbuilds(|store| {
            store.find(PacBuildQuery::select().where_install_state(InstallState::Indirect))
        });

        assert_eq!(found.len(), 0);
    }

    #[test]
    fn get_all_pacbuilds_by_kind_works() {
        let (fss, ..) = util::create_store_with_sample_data();
        let found = fss
            .query_pacbuilds(|store| store.find(PacBuildQuery::select().where_kind(Kind::DebFile)));

        assert_eq!(found.len(), 1);
    }

    #[test]
    fn get_all_pacbuilds_by_kind_works_when_no_results() {
        let (fss, ..) = util::create_store_with_sample_data();
        let found = fss
            .query_pacbuilds(|store| store.find(PacBuildQuery::select().where_kind(Kind::Binary)));

        assert_eq!(found.len(), 0);
    }

    #[test]
    fn get_all_pacbuilds_by_repository_url_works() {
        let (fss, repo, _) = util::create_store_with_sample_data();
        let found = fss.query_pacbuilds(|store| {
            store.find(PacBuildQuery::select().where_repository_url(repo.url.as_str().into()))
        });

        assert_eq!(found.len(), 1);
    }

    #[test]
    fn get_all_pacbuilds_by_repository_url_works_when_no_results() {
        let (fss, ..) = util::create_store_with_sample_data();
        let found = fss.query_pacbuilds(|store| {
            store.find(PacBuildQuery::select().where_repository_url("does not exist".into()))
        });

        assert_eq!(found.len(), 0);
    }

    #[test]
    fn update_pacbuild_works() {
        let (mut fss, _, mut pb) = util::create_store_with_sample_data();
        pb.description = "something else".into();

        fss.mutate_pacbuilds(|query| query.update(pb.clone()))
            .unwrap();

        let results = fss.query_pacbuilds(|query| {
            query.find(
                PacBuildQuery::select()
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

        fss.mutate_pacbuilds(|query| query.update(pb.clone()))
            .unwrap();
    }

    #[test]
    #[should_panic]
    fn remove_pacbuild_panics_when_pacbuild_not_found() {
        let (mut fss, ..) = util::create_store_with_sample_data();

        fss.mutate_pacbuilds(|query| {
            query.remove(PacBuildQuery::select().where_name("asd".into()))
        })
        .unwrap();
    }

    #[test]
    #[should_panic]
    fn add_pacbuild_panics_when_pacbuild_already_exists() {
        let (mut fss, _, pb) = util::create_store_with_sample_data();
        fss.mutate_pacbuilds(|query| query.insert(pb.clone()))
            .unwrap();
    }
}
