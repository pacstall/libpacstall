//!

use std::collections::HashMap;

use chrono::NaiveDateTime as DateTime;
use serde_derive::{Deserialize, Serialize};

use crate::store::errors::InvalidVersionError;

/// Representation of the PACBUILD file.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PacBuild {
    /// PacBuild unique name per [`Repository`](crate::model::Repository).
    pub name: PackageId,

    /// Last time it was changed.
    pub last_updated: DateTime,

    /// [`Repository`](crate::model::Repository) url.
    pub repository: URL,

    /// List of maintainers.
    ///
    /// # Example
    /// `Paul Cosma <paul.cosma97@gmail.com>`
    pub maintainers: Vec<String>,

    /// Canonical package name. Usually the `name` without the type extension.
    ///
    /// # Example
    /// - `PacBuild { name: "discord-deb", package_name: vec!["discord"] }`
    pub package_names: Vec<PackageId>,

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

    /// When building a split package, this variable can be used to explicitly
    /// specify the name to be used to refer to the group of packages in the
    /// output and in the naming of source-only tarballs.
    pub package_base: Option<PackageId>,

    /// Installation state.
    pub install_state: InstallState,

    /// An array of packages that must be installed for the software to build
    /// and run.
    pub dependencies: Vec<VersionConstrainedPackageId>,

    /// Used to force the package to be seen as newer than any previous version
    /// with a lower epoch. This value is required to be a non-negative
    /// integer; the default is 0. It is used when the version numbering
    /// scheme of a package changes (or is alphanumeric), breaking normal
    /// version comparison logic.
    pub epoch: i32,

    /// An array of additional packages that the software provides the features
    /// of (or a virtual package such as cron or sh). Packages providing the
    /// same item can be installed side-by-side, unless at least one of them
    /// uses a conflicts array
    pub provides: Vec<PackageId>,

    /// An array of packages that conflict with, or cause problems with the
    /// package, if installed. All these packages and packages providing
    /// this item will need to be removed
    pub conflicts: Vec<PackageId>,

    /// An array of obsolete packages that are replaced by the package, e.g.
    /// `wireshark-qt` uses `replaces=('wireshark')`
    pub replaces: Vec<PackageId>,

    /// An array of PPAs that provide the package.
    pub ppas: Vec<String>,

    /// The group the package belongs in. For instance, when installing
    /// `plasma`, it installs all packages belonging in that group.
    pub groups: Vec<GroupId>,

    /// Optional dependencies. Each Key:Pair is meant to describe the package
    /// identifier and the reason for installing.
    pub optional_dependencies: HashMap<VersionConstrainedPackageId, String>,

    /// An array of packages that are only required to build the software.
    pub make_dependencies: Vec<VersionConstrainedPackageId>,

    /// The license under which the software is distributed.
    pub licenses: Vec<String>,

    /// File required to build the package.
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
/// let ver: Version = Version::semver(1, 0, 0);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Version {
    pub major: i32,
    pub minor: i32,
    pub patch: i32,
    pub suffix: Option<String>,
}

impl Version {
    pub fn single(major: i32) -> Self {
        Version {
            major,
            minor: 0,
            patch: 0,
            suffix: None,
        }
    }

    pub fn double(major: i32, minor: i32) -> Self {
        Version {
            major,
            minor,
            patch: 0,
            suffix: None,
        }
    }

    pub fn semver(major: i32, minor: i32, patch: i32) -> Self {
        Version {
            major,
            minor,
            patch,
            suffix: None,
        }
    }

    pub fn semver_extended(major: i32, minor: i32, patch: i32, suffix: &str) -> Self {
        Version {
            major,
            minor,
            patch,
            suffix: Some(suffix.to_string()),
        }
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match self.major.partial_cmp(&other.major) {
            Some(core::cmp::Ordering::Equal) => {},
            ord => return ord,
        }

        match self.minor.partial_cmp(&other.minor) {
            Some(core::cmp::Ordering::Equal) => {},
            ord => return ord,
        }

        match self.patch.partial_cmp(&other.patch) {
            Some(core::cmp::Ordering::Equal) => {},
            ord => return ord,
        }

        self.suffix.partial_cmp(&other.suffix)
    }
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match self.partial_cmp(other) {
            Some(ord) => ord,
            None => panic!("unreachable"),
        }
    }
}

impl TryFrom<String> for Version {
    type Error = InvalidVersionError;

    fn try_from(value: String) -> Result<Self, InvalidVersionError> {
        let parts: Vec<&str> = value.split('.').collect();

        if parts.is_empty() {
            return Err(InvalidVersionError {});
        }

        let major: i32 = parts[0].parse().map_err(|_| InvalidVersionError {})?;

        let minor: i32 = if parts.len() >= 2 {
            parts[1].parse().map_err(|_| InvalidVersionError {})?
        } else {
            0
        };

        let patch: i32 = if parts.len() >= 3 {
            parts[2].parse().map_err(|_| InvalidVersionError {})?
        } else {
            0
        };

        let suffix = if parts.len() >= 4 {
            Some(parts[3..].join("."))
        } else {
            None
        };

        Ok(Version {
            major,
            minor,
            patch,
            suffix,
        })
    }
}

/// Represents a [`PacBuild`] or Apt package name.
/// # Examples
///
/// ```
/// use libpacstall::model::PackageId;
///
/// let identifier: PackageId = "discord-deb".into();
/// ```
pub type PackageId = String;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum VersionConstrainedPackageId {
    Any(PackageId),
    GreaterThan(Version, PackageId),
    GreaterThanEquals(Version, PackageId),
    LessThan(Version, PackageId),
    LessThanEquals(Version, PackageId),
    Between(Version, Version, PackageId),
    BetweenInclusive(Version, Version, PackageId),
}

#[allow(clippy::derive_hash_xor_eq)]
impl std::hash::Hash for VersionConstrainedPackageId {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match &self {
            Self::Any(p_id)
            | Self::GreaterThan(_, p_id)
            | Self::Between(_, _, p_id)
            | Self::BetweenInclusive(_, _, p_id)
            | Self::GreaterThanEquals(_, p_id)
            | Self::LessThanEquals(_, p_id)
            | Self::LessThan(_, p_id) => p_id.hash(state),
        };
    }
}

/// The group the package belongs in. For instance, when installing `plasma`, it
/// installs all packages belonging in that group.
pub type GroupId = String;
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
/// use libpacstall::model::{InstallState, Version};
///
/// let installed_directly = InstallState::Direct(
///     NaiveDate::from_ymd(2016, 7, 8).and_hms(9, 10, 11),
///     Version::semver(0, 9, 2),
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
