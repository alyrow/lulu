use crate::db::Db;
use crate::{error, tip, title, warning};
use rust_apt::cache::Cache;
use rust_apt::raw::progress::{AptAcquireProgress, AptInstallProgress};
use std::path::Path;
use yansi::{Color, Paint};

pub fn remove(name: String, purge: bool) {
    if sudo::check() != sudo::RunningAs::Root {
        warning!("Lulu must be run as root");
        match sudo::escalate_if_needed() {
            Ok(_) => {}
            Err(e) => {
                error!("Failed to run as root");
                tip!("Run lulu as root with `sudo lulu remove`");
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

    let mut document = db.collection("installed").doc(name.as_str());

    if !document.exist {
        error!("Package {} not installed with lulu", name);
        return;
    }

    title!("📦", "Uninstalling {}", name);
    let cache = match Cache::new::<bool>(&[]) {
        Ok(c) => c,
        Err(_) => todo!(),
    };

    let to_uninstall = cache.get(&name).expect("Should be in the cache");
    to_uninstall.mark_delete(purge);
    to_uninstall.protect();

    let mut acquire_progress = AptAcquireProgress::new_box();
    let mut install_progress = AptInstallProgress::new_box();

    match cache.commit(&mut acquire_progress, &mut install_progress) {
        Ok(_) => match document.delete() {
            Ok(_) => {}
            Err(_) => {
                error!(
                    "Failed to delete {} from database, db is now in broken state",
                    name
                );
            }
        },
        Err(_) => {
            error!("Failed to uninstall {}", name);
        }
    };
}
