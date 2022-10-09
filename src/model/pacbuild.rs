//!

use chrono::NaiveDateTime as DateTime;
use serde_derive::{Deserialize, Serialize};

/// Representation of the PACBUILD file.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PacBuild {
    /// PacBuild unique name per [`Repository`](crate::model::Repository).
    pub name: PackageId,

    /// Last time it was changed.
    pub last_updated: DateTime,

    /// [`Repository`](crate::model::Repository) url.
    pub repository: URL,

    /// Maintainer name and possibly email.
    ///
    /// # Example
    /// `Paul Cosma <paul.cosma97@gmail.com>`
    pub maintainer: String,

    /// Canonical package name. Usually the `name` without the type extension.
    ///
    /// # Example
    /// - `PacBuild { name: "discord-deb", package_name: "discord" }`
    pub package_name: String,

    /// Short package description.
    pub description: String,

    /// Official homepage [URL].
    pub homepage: URL,

    /// Latest version fetched from Repology.
    pub repology_version: Version,

    /// Repology filter.
    ///
    /// # Example
    /// **TBA**
    pub repology: String,

    /// Installation state.
    pub install_state: InstallState,

    /// Runtime dependencies.
    pub dependencies: Vec<PackageId>,

    /// Optional dependencies.
    pub optional_dependencies: Vec<PackageId>,

    /// SPDX License.
    pub license: String,

    /// Download [URL].
    pub url: URL,

    /// [`PacBuild`] type, deduced from the name suffix.
    pub kind: Kind,
}

/// Represents a `SemVer` version.
/// # Examples
///
/// ```
/// use libpacstall::model::Version;
///
/// let ver: Version = "1.0.0".into();
/// ```
pub type Version = String;

/// Represents a [`PacBuild`] or Apt package name.
/// # Examples
///
/// ```
/// use libpacstall::model::PackageId;
///
/// let identifier: PackageId = "discord-deb".into();
/// ```
pub type PackageId = String;
/// Represents an URL
/// # Examples
///
/// ```
/// use libpacstall::model::URL;
///
/// let url: URL = "https://example.com".into();
/// ```
pub type URL = String;
/// Represents a file checksum
/// # Examples
///
/// ```
/// use libpacstall::model::Hash;
///
/// let hash: Hash = "b5c9710f33204498efb64cf8257cd9b19e9d3e6b".into();
/// ```
pub type Hash = String;

/// Represents the install state of a package.
/// # Examples
///
/// ```
/// use chrono::NaiveDate;
/// use libpacstall::model::InstallState;
///
/// let installed_directly = InstallState::Direct(
///     NaiveDate::from_ymd(2016, 7, 8).and_hms(9, 10, 11),
///     "0.9.2".into(),
/// );
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum InstallState {
    /// Package is installed directly, meaning the user wanted it.
    Direct(DateTime, Version),

    /// Package is installed as a dependency.
    Indirect(DateTime, Version),

    /// Package is not installed.
    None,
}

impl InstallState {
    /// Returns `true` if the package is installed otherwise `false`.
    pub fn is_installed(&self) -> bool { !matches!(self, Self::None) }
}

/// Represents the type of the package. Usually deduced by the [PacBuild#name]
/// suffix.
///
/// # Examples
///
/// ```
/// use libpacstall::model::Kind;
///
/// let git_release = Kind::GitRelease;
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Kind {
    /// [`PacBuild`] will install an `AppImage`.
    AppImage(Hash),

    /// [`PacBuild`] will install a prebuilt, usually `tar.gz`, package.
    Binary(Hash),

    /// [`PacBuild`] will install an existing `.deb` file.
    DebFile(Hash),

    /// [`PacBuild`] will install the source of a given Git branch.
    GitBranch,

    /// [`PacBuild`] will install the source of a given Git release.
    GitRelease,
}
