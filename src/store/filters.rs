//! Provides various structs for querying and filtering packages.

/// Used to query packages by installation state
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum InstallState {
    Direct,
    Indirect,
    None,
}

impl From<&crate::model::InstallState> for InstallState {
    fn from(other: &crate::model::InstallState) -> Self {
        InstallState::from_model_install_state(other)
    }
}

impl InstallState {
    pub fn from_model_install_state(other: &crate::model::InstallState) -> InstallState {
        match other {
            crate::model::InstallState::Indirect(..) => InstallState::Indirect,
            crate::model::InstallState::Direct(..) => InstallState::Direct,
            crate::model::InstallState::None => InstallState::None,
        }
    }
}

/// Used to query packages by kind.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Kind {
    AppImage,
    Binary,
    DebFile,
    GitBranch,
    GitRelease,
}

impl From<&crate::model::Kind> for Kind {
    fn from(other: &crate::model::Kind) -> Self { Kind::from_model_kind(other) }
}

impl Kind {
    pub fn from_model_kind(other: &crate::model::Kind) -> Kind {
        match other {
            crate::model::Kind::GitRelease => Kind::GitRelease,
            crate::model::Kind::GitBranch => Kind::GitBranch,
            crate::model::Kind::AppImage(_) => Kind::AppImage,
            crate::model::Kind::Binary(_) => Kind::Binary,
            crate::model::Kind::DebFile(_) => Kind::DebFile,
        }
    }
}
