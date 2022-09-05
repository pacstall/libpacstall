//! Provides structs to handle Pacstall's data models.

mod pacbuild;
mod repository;

pub use crate::model::pacbuild::*;
pub use crate::model::repository::{default_repository, Repository};
