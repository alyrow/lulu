mod package;
mod commands;
mod utils;

use clap::{Parser, Subcommand};
use yansi::Paint;

use crate::commands::install;

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
        None => {}
    }
}
