use crate::db::Condition;
use crate::model::{Config, DbPackage};
use crate::utils::db::open_and_lock_db;
use crate::utils::lulu::lulu_file;
use crate::{error, success, tip, title, warning};
use serde_json::Value;
use std::io::Read;
use std::path::Path;
use yansi::{Color, Paint};

pub fn update(_no_check: bool) {
    if sudo::check() != sudo::RunningAs::Root {
        warning!("Lulu must be run as root");
        match sudo::with_env(&["USER", "HOME"]) {
            Ok(_) => {}
            Err(e) => {
                error!("Failed to run as root");
                tip!("Run lulu as root with `sudo lulu update`");
                panic!("{:?}", e);
            }
        }
    }
    let mut db = match open_and_lock_db() {
        Ok(db) => db,
        Err(e) => {
            panic!("{:?}", e);
        }
    };

    title!("📁", "Getting repositories from config");
    let file = match std::fs::File::open(Path::new("/etc/lulu.conf")) {
        Ok(f) => f,
        Err(e) => {
            error!("Error while opening /etc/lulu.conf");
            panic!("{:?}", e);
        }
    };
    let mut buf_reader = std::io::BufReader::new(file);
    let mut contents = String::new();
    match buf_reader.read_to_string(&mut contents) {
        Ok(_) => {}
        Err(e) => {
            error!("Failed to read /etc/lulu.conf");
            panic!("{:?}", e);
        }
    }
    let config: Config = toml::from_str(&contents).unwrap();
    config.repositories.iter().for_each(|repo| {
        title!("🔎", "Updating {}", repo.name);
        let path = Path::new("/var/lib/lulu/repositories").join(repo.name.clone());
        let mut remote = match git2::Remote::create_detached(repo.source.clone()) {
            Ok(r) => r,
            Err(_) => {
                error!("Failed to create update");
                return;
            }
        };
        match remote.connect(git2::Direction::Fetch) {
            Ok(_) => {}
            Err(_) => {
                error!("Failed to connect to repository");
                return;
            }
        }
        let remote_oid = match remote.list() {
            Ok(list) => match list.first() {
                None => {
                    error!("Remote repository seems empty");
                    return;
                }
                Some(head) => head.oid(),
            },
            Err(_) => {
                error!("Failed to get list from remote repository");
                return;
            }
        };

        let mut need_update = false;

        let git_repo = match git2::Repository::open(path.clone()) {
            Ok(r) => r,
            Err(_) => match std::fs::create_dir_all(path.as_path()) {
                Ok(_) => match git2::Repository::clone(repo.source.as_str(), path.clone()) {
                    Ok(r) => {
                        need_update = true;
                        r
                    }
                    Err(_) => {
                        error!("Can't clone repository");
                        return;
                    }
                },
                Err(_) => {
                    error!("Can't create repository");
                    return;
                }
            },
        };

        let local_oid = match git_repo.head() {
            Ok(head) => match head.target() {
                None => {
                    error!("The commit should point to a ref");
                    return;
                }
                Some(oid) => oid,
            },
            Err(_) => {
                error!("There should be at least one commit");
                return;
            }
        };

        if remote_oid != local_oid {
            match crate::utils::git::pull(git_repo, "origin", "master") {
                // TODO get remote from config
                Ok(_) => {
                    need_update = true;
                }
                Err(_) => {
                    error!("Failed to update repository");
                    return;
                }
            }
        }

        if need_update {
            match db.clone().collection("packages").wherr(
                "repository".to_string(),
                Condition::Equal,
                Value::from(repo.name.clone()),
            ) {
                Ok(w) => w,
                Err(_) => {
                    error!("Failed to update repository");
                    return;
                }
            }
            .get()
            .iter()
            .for_each(|doc| {
                let _ = doc.doc.clone().delete();
            });
            let rd = match std::fs::read_dir(path) {
                Ok(rd) => rd,
                Err(_) => {
                    error!("Failed to update repository");
                    return;
                }
            };
            rd.for_each(|dir| {
                if dir.is_ok() {
                    let dir = dir.unwrap();
                    if dir.path().is_dir() && dir.path().join("LULU.toml").is_file() {
                        let lulu = match lulu_file(dir.path().join("LULU.toml")) {
                            Ok(f) => {
                                if f.is_ok() {
                                    f.unwrap()
                                } else {
                                    error!("LULU.toml is not deserializable");
                                    return;
                                }
                            }
                            Err(_) => {
                                error!("LULU.toml is not readable");
                                return;
                            }
                        };
                        match db
                            .clone()
                            .collection("packages")
                            .doc(lulu.package.name.as_str())
                            .set(DbPackage {
                                repository: repo.name.clone(),
                                path: dir.path().display().to_string(),
                            }) {
                            Ok(_) => {}
                            Err(e) => {
                                warning!("Failed to add package {}", lulu.package.name);
                                eprintln!("{:?}", e);
                            }
                        };
                    }
                }
            })
        }

        success!("Up to date");
    });

    match db.unlock() {
        Ok(_) => {}
        Err(e) => {
            error!("Failed to unlock database");
            panic!("{:?}", e);
        }
    };
}
