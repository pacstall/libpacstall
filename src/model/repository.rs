use serde_derive::{Deserialize, Serialize};

/// Representation of a Pacstall repository.
///
/// Defaults to the official repository.
#[derive(Deserialize, Debug, Eq, PartialEq, Serialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Repository {
    /// The name of the repository.
    pub name: String,
    /// URL of the repository.
    ///
    /// Note that the URL **isn't verified** during extraction!
    pub url: String,
    /// Preference of the repository.
    ///
    /// Specifies which repository to look into first during certain operations
    /// like installing a package. If the package isn't present in the first
    /// preferred repository, then the second preferred repository is looked
    /// into.
    pub preference: u32,
}

#[allow(clippy::module_name_repetitions)]
pub fn default_repository() -> Vec<Repository> { vec![Repository::default()] }

impl Default for Repository {
    fn default() -> Self {
        Self {
            name: "official".into(),
            url: "https://github.com/pacstall/pacstall-programs".into(),
            preference: 1,
        }
    }
}
