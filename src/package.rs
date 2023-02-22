use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Lulu {
    pub package: Package,
    pub dependencies: Dependencies,
    pub script: Script,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Package {
    pub name: String,
    pub maintainers: Vec<String>,
    pub description: String,
    pub url: Option<String>,
    pub source: String,
    pub arch: Vec<String>,
    pub license: Vec<String>,
    pub provides: Vec<String>,
    pub preinst: Option<String>,
    pub postinst: Option<String>,
    pub prerm: Option<String>,
    pub postrm: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Dependencies {
    pub runtime: BTreeMap<String, Dependency>,
    pub build: BTreeMap<String, Dependency>,
    pub optional: BTreeMap<String, Dependency>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Script {
    pub prepare: Option<String>,
    pub build: Option<String>,
    pub check: Option<String>,
    pub package: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Dependency {
    pub is: DependencyType,
    pub git: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub enum DependencyType {
    APT,
    GIT,
}
