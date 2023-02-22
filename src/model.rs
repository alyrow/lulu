use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct Config {
    pub ignore: Vec<String>,
    pub repositories: Vec<Repository>,
}

#[derive(Deserialize)]
pub struct Repository {
    pub name: String,
    pub source: String,
}

#[derive(Deserialize, Serialize)]
pub struct DbPackage {
    pub repository: String,
    pub path: String,
}
