use crate::model::Config;
use crate::{error, tip, title, warning};
use std::io::Read;
use std::path::Path;
use yansi::{Color, Paint};

pub fn update(_no_check: bool) {
    if sudo::check() != sudo::RunningAs::Root {
        warning!("Lulu must be run as root");
        match sudo::escalate_if_needed() {
            Ok(_) => {}
            Err(e) => {
                error!("Failed to run as root");
                tip!("Run lulu as root with `sudo lulu update`");
                panic!("{:?}", e);
            }
        }
    }
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
                error!("Failed to connect to repository")
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

        let repo = match git2::Repository::open(path.clone()) {
            Ok(r) => r,
            Err(_) => match std::fs::create_dir_all(path.as_path()) {
                Ok(_) => match git2::Repository::clone(repo.source.as_str(), path) {
                    Ok(r) => r,
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

        let local_oid = match repo.head() {
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
            match crate::utils::git::pull(repo, "origin", "master") {
                // TODO get remote from config
                Ok(_) => {
                    // TODO Update db
                }
                Err(_) => {
                    error!("Failed to update repository");
                }
            }
        }
    });
}
