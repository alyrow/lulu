use crate::db::Db;
use crate::{error, success, tip, title, warning};
use std::io::ErrorKind;
use std::path::Path;
use yansi::{Color, Paint};

pub fn setup() {
    title!("⚙", "Setting up lulu database");
    match Db::new(Path::new("/var/lib/lulu/db").to_path_buf()) {
        Ok(_) => {
            success!("Done");
        }
        Err(e) => match e.kind() {
            ErrorKind::PermissionDenied => {
                if sudo::check() != sudo::RunningAs::Root {
                    warning!("Lulu must be run as root");
                    match sudo::escalate_if_needed() {
                        Ok(_) => {}
                        Err(e) => {
                            error!("Failed to run as root");
                            tip!("Run lulu as root with `sudo lulu setup`");
                            panic!("{:?}", e);
                        }
                    }
                } else {
                    error!("Unrecoverable error while setting up lulu database (root does not have any rights?)");
                    panic!("{:?}", e);
                }
            }
            _ => {
                error!("Unrecoverable error while setting up lulu database");
                panic!("{:?}", e);
            }
        },
    }
}
