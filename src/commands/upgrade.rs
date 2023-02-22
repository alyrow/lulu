use std::io::Error;
use std::path::Path;
use yansi::{Color, Paint};
use crate::{error, tip, title, warning};
use crate::commands::install;
use crate::db::Db;
use crate::model::Installed;

pub fn upgrade() {
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
    let db = match Db::new(Path::new("/var/lib/lulu/db").to_path_buf()) {
        Ok(db) => db,
        Err(e) => {
            error!("Failed to open database");
            panic!("{:?}", e);
        }
    };
    title!("🧨", "Checking for upgrades");
    db.clone().collection("installed").get().iter().for_each(|p| {
        if !db.clone().collection("packages").doc(p.id.as_str()).exist {
            warning!("Skipping {} as it is not in a repository so we don't know what to do if there are updates available", p.id);
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
}