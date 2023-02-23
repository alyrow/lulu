use crate::utils::db::open_and_lock_db;
use crate::{error, tip, title, warning};
use rust_apt::cache::Cache;
use rust_apt::raw::progress::{AptAcquireProgress, AptInstallProgress};
use yansi::{Color, Paint};

pub fn remove(name: String, purge: bool) {
    if sudo::check() != sudo::RunningAs::Root {
        warning!("Lulu must be run as root");
        match sudo::with_env(&["USER", "HOME"]) {
            Ok(_) => {}
            Err(e) => {
                error!("Failed to run as root");
                tip!("Run lulu as root with `sudo lulu remove`");
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

    let mut document = db.clone().collection("installed").doc(name.as_str());

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

    match db.unlock() {
        Ok(_) => {}
        Err(e) => {
            error!("Failed to unlock database");
            panic!("{:?}", e);
        }
    };
}
