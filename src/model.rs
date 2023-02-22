use serde::Deserialize;

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
