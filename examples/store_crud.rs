use chrono::NaiveDateTime;
use error_stack::Result;
use libpacstall::model::{InstallState, Kind, PacBuild, Repository};
use libpacstall::store::base::Store;
use libpacstall::store::errors::StoreError;
use libpacstall::store::filters;
use libpacstall::store::query_builder::{PacBuildQuery, RepositoryQuery, StringClause};

fn main() {
    let mut store = Store::in_memory();

    example_entity_insertion(&mut store).unwrap();
    example_entity_query(&store);
    example_entity_update(&mut store).unwrap();
    example_entity_deletion(&mut store).unwrap();
}

fn example_entity_query(store: &Store) {
    println!("## Running [example_entity_query]");

    println!("\n\tSearching for all pacbuilds that contain the word 'discord' in their name.");

    let pacbuilds = store.query_pacbuilds(|store| {
        store.find(
            PacBuildQuery::select().where_name(StringClause::Contains(String::from("discord"))),
        )
    });

    println!(
        "\n\tWe are expecting to find 1 result. Found: {}.",
        pacbuilds.len()
    );
    assert_eq!(pacbuilds.len(), 1);
    println!("\tDone!");

    let pacbuild = pacbuilds.first().unwrap();

    println!(
        "\n\tWe are expecting to find 'discord-deb' from 'https://awesome-repository.local' \
         repository."
    );
    println!(
        "\t\tFound '{}' from repository '{}'",
        &pacbuild.name, &pacbuild.repository
    );
    assert_eq!(pacbuild.name, String::from("discord-deb"));
    assert_eq!(
        pacbuild.repository,
        String::from("https://awesome-repository.local")
    );
    println!("\tDone!\n");
}

#[allow(clippy::redundant_pattern_matching)]
fn example_entity_insertion(store: &mut Store) -> Result<(), StoreError> {
    println!("\n## Running [example_entity_insertion]\n");

    // Create dummy data
    let repository = create_repository(
        String::from("My Awesome Repository"),
        String::from("https://awesome-repository.local"),
    );

    let pacbuild = create_pacbuild(
        String::from("discord-deb"),
        InstallState::None,
        Kind::DebFile(String::from("some hash")),
        repository.url.clone(),
    );

    // Insert repository first, because the pacbuild depends on it.
    println!("\n\tAttempting to insert the new repository into the store.");
    store.mutate_repositories(|store| store.insert(repository.clone()))?;
    println!("\tDone!\n");

    // Repository exists so it is safe to add the pacbuild.
    println!("\tAttempting to insert the new pacbuild into the store.");
    store.mutate_pacbuilds(|store| store.insert(pacbuild.clone()))?;
    println!("\tDone!\n");

    // PacBuild is already inserted, so trying to insert it again would result in a
    // conflict error.
    println!("\tAttempting to insert the same pacbuild into the store.");
    let result = store.mutate_pacbuilds(|store| store.insert(pacbuild.clone()));
    if let Err(_) = &result {
        println!("\t\tInserting the same pacbuild failed as expected.");

        // Uncomment the next line to see how the stacktrace looks :)
        // result.unwrap();
    } else {
        panic!("\t\tThis will never be printed.")
    }
    println!("\tDone!\n");

    Ok(())
}

fn example_entity_update(store: &mut Store) -> Result<(), StoreError> {
    println!("## Running [example_entity_update]\n");

    // Search for the discord package.
    println!("\tSearching for a single package called 'discord-deb'.");
    let mut pacbuild = store
        .query_pacbuilds(|store| {
            store.single(
        PacBuildQuery::select().where_name("discord-deb".into()) // Same as StringClause::Equals(String::from("discord-deb"))
    )
        })
        .unwrap();

    println!("\tFound: {:#?}\n", pacbuild);
    assert_eq!(pacbuild.install_state, InstallState::None);

    // Assume we installed it
    println!("\tWe update it so it looks like it is installed.");
    pacbuild.install_state = InstallState::Direct(current_time(), String::from("1.0.0"));
    store.mutate_pacbuilds(|store| store.update(pacbuild.clone()))?;
    println!("\tUpdated pacbuild: {:#?}\n", pacbuild);

    // Search again
    println!("\tWe search for the same package again.");
    let same_pacbuild = store
        .query_pacbuilds(|store| {
            store.single(PacBuildQuery::select().where_install_state(filters::InstallState::Direct))
        })
        .unwrap();
    println!(
        "\tValue after re-querying the store: {:#?}\n",
        same_pacbuild
    );

    println!("\tAsserting that the change propagated.");
    assert_eq!(pacbuild, same_pacbuild);
    println!("\tDone!");

    Ok(())
}

fn example_entity_deletion(store: &mut Store) -> Result<(), StoreError> {
    println!("## Running [example_entity_deletion]\n");

    // Select the first repository
    println!("\tFetching a repository.");
    let repository = store
        .query_repositories(|store| store.single(RepositoryQuery::select()))
        .unwrap();
    println!("\tFound: {:?}\n", repository);

    let pacbuilds = store.query_pacbuilds(|store| {
        store.find(PacBuildQuery::select().where_repository_url(repository.url.as_str().into()))
    });
    println!(
        "\tThis repository has a total of **{}** pacbuilds.\n",
        pacbuilds.len()
    );

    // We attempt to delete it
    println!("\tAttempting to delete it.");
    store.mutate_repositories(|store| {
        store.remove(RepositoryQuery::select().where_url(repository.url.as_str().into()))
    })?;
    println!("\tDone!\n");

    // Selecting the same repository again
    println!("\tSelecting the same repository again.");
    let found = store.query_repositories(|store| {
        store.single(RepositoryQuery::select().where_url(repository.url.as_str().into()))
    });
    assert!(found.is_none());
    println!("\tFound no match.\n");

    // Find any pacbuild from that repository.
    println!("\tAttempting to find any pacbuild from that repository.");
    let pacbuilds = store.query_pacbuilds(|store| {
        store.find(PacBuildQuery::select().where_repository_url(repository.url.as_str().into()))
    });

    println!("\tWe expect to find none. Found: **{}**\n", pacbuilds.len());
    assert_eq!(pacbuilds.len(), 0);

    Ok(())
}

fn create_pacbuild(
    name: String,
    install_state: InstallState,
    kind: Kind,
    repository_url: String,
) -> PacBuild {
    println!(
        "\tCreating dummy PacBuild[name = '{}', install_state = '{:?}', kind = '{:?}', repository \
         = '{}']",
        name, install_state, kind, repository_url
    );

    PacBuild {
        name,
        last_updated: current_time(),
        repository: repository_url,
        maintainer: String::from(""),
        package_name: String::from(""),
        description: String::from(""),
        homepage: String::from(""),
        repology_version: String::from(""),
        repology: String::from(""),
        install_state,
        dependencies: Vec::new(),
        optional_dependencies: Vec::new(),
        license: String::from("MIT"),
        url: String::from("https://pacbuild.pac"),
        kind,
    }
}

fn create_repository(name: String, url: String) -> Repository {
    println!(
        "\tCreating dummy Repository[name = '{}', url = '{}']",
        name, url
    );
    Repository {
        name,
        url,
        preference: 0,
    }
}

fn current_time() -> NaiveDateTime {
    NaiveDateTime::from_timestamp(chrono::Utc::now().timestamp(), 0)
}
