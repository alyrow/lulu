mod commands;
mod db;
mod model;
mod package;
mod utils;

use clap::{Parser, Subcommand};
use serde::Serialize;
use std::path::Path;
use yansi::Paint;

use crate::commands::{install, remove, setup, update, upgrade};

/// Concept of package manager built on top of apt for handling git repositories
#[derive(Parser)]
#[command(version)]
struct Cli {
    /// Disable color output
    #[arg(long)]
    no_color: bool,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Install packages
    Install {
        /// Package to install
        ///
        /// Can be a package name if lulu is connected to a repository, a git repository with LULU.toml file or can be blank in which case it will fallback
        /// to the current directory (if a valid LULU.toml file exists).
        name: Option<String>,

        /// Do not install built package
        #[arg(short, long)]
        no_install: bool,
    },
    /// Setup lulu db
    ///
    /// Should be executed only once. It will create and init db at /var/log/lulu/db
    Setup {},
    /// Update each repository and eventually inform about possible upgrades
    Update {
        /// Do not check for upgrades
        #[arg(short, long)]
        no_check: bool,
    },
    /// Upgrade installed packages
    Upgrade {},
    /// Remove an installed package
    Remove {
        // Package to uninstall
        name: String,

        /// Purge
        #[arg(short, long)]
        purge: bool,
    },
}

fn main() {
    simple_logger::init_with_level(log::Level::Trace).unwrap();

    let cli = Cli::parse();

    if Paint::enable_windows_ascii() && !cli.no_color {
        Paint::enable()
    } else {
        Paint::disable()
    }

    match &cli.command {
        Some(Commands::Install { name, no_install }) => {
            println!("{:?}", name);
            install(name.to_owned(), no_install.to_owned());
        }
        Some(Commands::Setup { .. }) => {
            setup();
        }
        Some(Commands::Update { no_check }) => {
            update(no_check.to_owned());
        }
        Some(Commands::Upgrade { .. }) => upgrade(),
        Some(Commands::Remove { name, purge }) => {
            remove(name.to_owned(), purge.to_owned());
        }
        None => {
            let fm = db::Db::new(Path::new("/var/lib/lulu/db").to_path_buf()).unwrap();
            fm.collection("test")
                .add(Test {
                    name: "alyrow".to_string(),
                    n: 22,
                })
                .unwrap();
        }
    }
}

#[derive(Serialize)]
struct Test {
    name: String,
    n: u8,
}
