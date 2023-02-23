use crate::commands::install;
use crate::model::{Config, Installed};
use crate::utils::db::open_and_lock_db;
use crate::{error, tip, title, warning};
use std::io::Read;
use std::path::Path;
use yansi::{Color, Paint};

pub fn upgrade() {
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

    title!("🧨", "Checking for upgrades");
    db.clone().collection("installed").get().iter().for_each(|p| {
        if !db.clone().collection("packages").doc(p.id.as_str()).exist {
            warning!("Skipping {} as it is not in a repository so we don't know what to do if there are updates available", p.id);
            return;
        }

        if config.ignore.contains(&p.id) {
            warning!("Skipping {} as it is in ignore section", p.id);
            return;
        }

        let package = match p.doc.clone().get::<Installed>() {
            Ok(pkg) => match pkg {
                None => {
                    error!("Failed to check update for {}", p.id);
                    return;
                }
                Some(p) => p
            }
            Err(_) => {
                error!("Failed to check update for {}", p.id);
                return;
            }
        };

        let mut remote = match git2::Remote::create_detached(package.source) {
            Ok(r) => r,
            Err(_) => {
                error!("Failed to check update for {}", p.id);
                return;
            }
        };
        match remote.connect(git2::Direction::Fetch) {
            Ok(_) => {}
            Err(_) => {
                error!("Failed to connect to source for {}", p.id);
                return;
            }
        }
        let remote_oid = match remote.list() {
            Ok(list) => match list.first() {
                None => {
                    error!("Remote repository seems empty for {}", p.id);
                    return;
                }
                Some(head) => head.oid(),
            },
            Err(_) => {
                error!("Failed to get list from remote repository for {}", p.id);
                return;
            }
        };

        if remote_oid.to_string() == package.hash { return; }

        title!("⚙" ,"Upgrading {}", p.id);
        install(Some(p.clone().id), false);
    });

    match db.unlock() {
        Ok(_) => {}
        Err(e) => {
            error!("Failed to unlock database");
            panic!("{:?}", e);
        }
    };
}
