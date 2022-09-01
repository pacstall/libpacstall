use chrono::NaiveDateTime as DateTime;
use serde_derive::{Deserialize, Serialize};

use crate::model::pacbuild::PacBuild;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Repository {
    pub name: String,
    pub last_updated: DateTime,
    pub url: String,
    pub pacbuilds: Vec<PacBuild>,
    pub priority: u8,
}
