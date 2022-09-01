use chrono::NaiveDateTime as DateTime;
use serde_derive::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PacBuild {
    pub name: PackageId,
    pub last_updated: DateTime,
    pub repository: URL,
    pub maintainer: String,
    pub package_name: String,
    pub description: String,
    pub homepage: URL,
    pub repology_version: Version,
    pub repology: URL,
    pub install_state: InstallState,
    pub dependencies: Vec<PackageId>,
    pub optional_dependencies: Vec<PackageId>,
    pub license: String,
    pub url: URL,
    pub kind: Kind,
}

pub type Version = String;
pub type PackageId = String;
pub type URL = String;
pub type Hash = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InstallState {
    Direct(DateTime, Version),
    Indirect(DateTime, Version),
    None,
}

impl InstallState {
    pub fn is_installed(&self) -> bool {
        match self {
            Self::None => false,
            _ => true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Kind {
    AppImage(Hash),
    Binary(Hash),
    DebFile(Hash),
    GitBranch,
    GitRelease,
}
