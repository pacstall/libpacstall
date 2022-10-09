//! Provides various structs for querying and filtering
//! [`PacBuild`](crate::model::PacBuild)s.

/// Used to query [`PacBuild`](crate::model::PacBuild)s by installation state.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum InstallState {
    /// [`PacBuild`](crate::model::PacBuild) is installed directly.
    Direct,

    /// [`PacBuild`](crate::model::PacBuild) is installed, but as a dependency
    /// of another.
    Indirect,

    /// [`PacBuild`](crate::model::PacBuild) is not installed.
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

/// Used to query [`PacBuild`](crate::model::PacBuild)s by kind.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Kind {
    /// [`PacBuild`](crate::model::PacBuild) is a prebuilt AppImage.
    AppImage,

    /// [`PacBuild`](crate::model::PacBuild) is a prebuilt binary. Usually a
    /// compressed in a tar or zip file.
    Binary,

    /// [`PacBuild`](crate::model::PacBuild) is a prebuilt `.deb` file.
    DebFile,

    /// [`PacBuild`](crate::model::PacBuild) will be built from a git branch.
    GitBranch,

    /// [`PacBuild`](crate::model::PacBuild) will be built from a fixed git
    /// release.
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
