use crate::error;
use crate::utils::db::open_db;
use std::path::Path;

use crate::model::{DbPackage, Installed};
use crate::package::Lulu;
use crate::utils::lulu::lulu_file;
use yansi::{Color, Paint};

fn display(id: String, installed: Option<Installed>, lulu: Option<Lulu>) {
    println!(
        "{}\t{}",
        Paint::cyan(id).bold(),
        if installed.is_some() {
            Paint::green(format!("Installed ({})", installed.unwrap().version))
        } else {
            Paint::default("Not installed".to_string()).dimmed()
        }
    );
    if lulu.is_some() {
        println!("  {}", lulu.unwrap().package.description);
    }
    println!();
}

pub fn list(installed: bool) {
    let db = match open_db() {
        Ok(db) => db,
        Err(e) => {
            error!("Failed to open database");
            panic!("{:?}", e);
        }
    };

    if installed {
        db.clone()
            .collection("installed")
            .get()
            .iter()
            .for_each(|doc| {
                let installed = match doc.doc.clone().get::<Installed>() {
                    Ok(opt) => match opt {
                        None => {
                            return;
                        }
                        Some(data) => data,
                    },
                    Err(_) => {
                        error!("Failed get document");
                        return;
                    }
                };
                let lulu = match db
                    .clone()
                    .collection("packages")
                    .doc(doc.id.as_str())
                    .get::<DbPackage>()
                {
                    Ok(opt) => match opt {
                        None => None,
                        Some(data) => match lulu_file(Path::new(&data.path).join("LULU.toml")) {
                            Ok(res) => match res {
                                Ok(lulu) => Some(lulu),
                                Err(_) => None,
                            },
                            Err(_) => None,
                        },
                    },
                    Err(_) => {
                        error!("Failed get document");
                        return;
                    }
                };
                display(doc.clone().id, Some(installed), lulu);
            });
    } else {
        db.clone()
            .collection("packages")
            .get()
            .iter()
            .for_each(|doc| {
                let installed = match db
                    .clone()
                    .collection("installed")
                    .doc(&doc.id)
                    .get::<Installed>()
                {
                    Ok(opt) => opt,
                    Err(_) => {
                        error!("Failed get document");
                        return;
                    }
                };
                let lulu = match doc.doc.clone().get::<DbPackage>() {
                    Ok(opt) => match opt {
                        None => None,
                        Some(data) => match lulu_file(Path::new(&data.path).join("LULU.toml")) {
                            Ok(res) => match res {
                                Ok(lulu) => Some(lulu),
                                Err(_) => None,
                            },
                            Err(_) => None,
                        },
                    },
                    Err(_) => {
                        error!("Failed get document");
                        return;
                    }
                };
                display(doc.clone().id, installed, lulu);
            });
    }
}
