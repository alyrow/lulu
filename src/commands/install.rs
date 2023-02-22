use std::{
    env,
    fs::{DirBuilder, File},
    path::{Path, PathBuf},
    process::Command,
};

use deb_rust::{binary::DebPackage, DebArchitecture};
use fork::{fork, Fork};
use git2::{DescribeOptions, Repository};
use log::trace;
use rust_apt::{
    cache::Cache,
    package::Package as AptPackage,
    raw::progress::{AptAcquireProgress, AptInstallProgress},
};
use yansi::{Color, Paint};

use crate::db::Db;
use crate::model::{DbPackage, Installed};
use crate::utils::lulu::lulu_file;
use crate::{
    error,
    package::{DependencyType, Lulu},
    success, tip, title, warning,
};

fn install_local(ctx: Context) {
    let deserialized = match lulu_file("LULU.toml") {
        Ok(f) => f.unwrap(),
        Err(e) => {
            error!("LULU.toml is not readable");
            panic!("{:?}", e)
        }
    };

    install_with_ctx(env::current_dir().unwrap(), deserialized, ctx);
}

fn install_git(url: String, ctx: Context) {
    let path = env::temp_dir().join(format!("lulu_{}", url.replace(":", "_").replace("/", "_")));
    let mut builder = DirBuilder::new();
    builder.recursive(true);
    builder.create(path.clone().into_os_string()).unwrap();

    title!(
        "🔎",
        "Cloning repository into {}",
        Paint::cyan(path.clone().display()).underline()
    );

    let _repo = match Repository::clone(&url, path.clone()) {
        Ok(repo) => repo,
        Err(e) => {
            error!("Failed to clone repository");
            panic!("{:?}", e)
        }
    };
    env::set_current_dir(path.display().to_string()).unwrap();
    install_local(ctx);
}

fn install_db(name: String, ctx: Context) {
    let document = ctx.clone().db.collection("packages").doc(name.as_str());
    if !document.exist {
        error!("Package {} not found", name);
        panic!("Package {} not found", name);
    }

    let package = match document.get::<DbPackage>() {
        Ok(p) => match p {
            None => {
                error!("Document seems to be empty");
                panic!("Document seems to be empty");
            }
            Some(p) => p,
        },
        Err(e) => {
            error!("Failed to read document");
            panic!("{:?}", e);
        }
    };

    let path = env::temp_dir().join(format!("lulu_{}", name));
    let mut builder = DirBuilder::new();
    builder.recursive(true);
    builder.create(path.clone().into_os_string()).unwrap();

    match std::fs::copy(
        Path::new(&package.path).join("LULU.toml"),
        path.join("LULU.toml"),
    ) {
        Ok(_) => {}
        Err(e) => {
            error!("Failed to copy LULU.toml");
            panic!("{:?}", e);
        }
    };

    env::set_current_dir(path.display().to_string()).unwrap();
    install_local(ctx);
}

fn install_with_ctx(path: PathBuf, lulu: Lulu, ctx: Context) {
    let repo = match Repository::open(path.clone()) {
        Ok(repo) => repo,
        Err(_) => {
            let path2 = path.join("SRC");
            match Repository::open(path2.clone()) {
                Ok(repo) => repo,
                Err(_) => {
                    let mut builder = DirBuilder::new();
                    builder.recursive(true);
                    builder.create(path2.clone().into_os_string()).unwrap();
                    title!(
                        "🔎",
                        "Cloning source repository into {}",
                        Paint::cyan(path2.clone().display()).underline()
                    );
                    match Repository::clone(&lulu.package.source, path2) {
                        Ok(repo) => repo,
                        Err(e) => {
                            error!("Failed to clone repository");
                            panic!("{:?}", e)
                        }
                    }
                }
            }
        }
    };

    // TODO: Go to a particular commit
    let version = match repo.describe(&DescribeOptions::default()) {
        Ok(d) => match d.format(None) {
            Ok(s) => s.replace("-", ".").replace("v", ""),
            Err(e) => {
                error!("Failed to get version");
                panic!("{:?}", e)
            }
        },
        Err(_) => repo
            .head()
            .expect("There should be at least one commit")
            .target()
            .expect("The commit should point to a ref")
            .to_string(),
    };
    trace!("Version is {}", Paint::cyan(version.clone()));

    if sudo::check() != sudo::RunningAs::Root {
        sudo::with_env(&["USER", "HOME"]).expect("lulu need root access to install packages");
    }

    title!("📦", "Installing build dependencies");
    let apt_dependencies: Vec<String> = lulu
        .dependencies
        .build
        .iter()
        .filter(|(_, e)| e.is == DependencyType::APT)
        .map(|(k, _)| k.to_string())
        .collect();

    let cache = match Cache::new::<bool>(&[]) {
        Ok(c) => c,
        Err(_) => todo!(),
    };

    let mut ok = true;
    for pkg in apt_dependencies.clone() {
        if cache.get(&pkg).is_none() {
            warning!("Failed to find package: {}", Paint::yellow(pkg).italic());
            ok = false;
        }
    }

    if !ok {
        error!("Failed to retreive all packages");
        tip!("Try to run `apt update`");
        panic!("Some packages don't exist in cache")
    }

    let apt_dependencies: Vec<AptPackage> = apt_dependencies
        .into_iter()
        .map(|pkg| cache.get(&pkg).expect("Should be in the cache"))
        .collect();

    let mut to_uninstall = Vec::<String>::new();

    for pkg in apt_dependencies {
        success!(
            "Found package: {}  \t{}",
            Paint::cyan(pkg.name()).italic(),
            if pkg.installed().is_some() {
                Paint::green(format!(
                    "Installed ({})",
                    pkg.installed().unwrap().version()
                ))
            } else {
                Paint::red(format!("To be installed"))
            }
        );
        if pkg.installed().is_none() {
            if !pkg.mark_install(true, false) {
                error!("Can't mark {} for install", Paint::red(pkg.name()).italic());
            }
            pkg.protect();
            to_uninstall.push(pkg.name().to_string())
        }
    }

    cache.resolve(true).unwrap();

    let mut acquire_progress = AptAcquireProgress::new_box();
    let mut install_progress = AptInstallProgress::new_box();

    match cache.get_archives(&mut acquire_progress) {
        Ok(_) => match cache.do_install(&mut install_progress) {
            Ok(_) => (),
            Err(e) => panic!("{:?}", e),
        },
        Err(e) => panic!("{:?}", e),
    }

    // BUILD

    let mut status: i32 = 0;
    match fork() {
        Ok(Fork::Parent(child)) => {
            trace!(
                "Continuing execution in parent process, new child has pid: {}",
                child
            );
            unsafe { libc::waitpid(child, &mut status, 0) };
            trace!("Status is {}", status);
        }
        Ok(Fork::Child) => {
            let srcdir = repo.path().parent().unwrap().to_path_buf();

            let sudo = env::var("SUDO_USER");
            if sudo.is_ok() && sudo.unwrap() != "" {
                let uid: u32 = env::var("SUDO_UID").unwrap().parse().unwrap();
                let gid: u32 = env::var("SUDO_GID").unwrap().parse().unwrap();
                unsafe { libc::setuid(uid) };
                unsafe { libc::setgid(gid) };
            }

            let pkgdir = path.join("LULU");
            let mut builder = DirBuilder::new();
            builder.recursive(true);
            builder.create(pkgdir.clone().into_os_string()).unwrap();

            generate(lulu.clone(), path, srcdir, pkgdir.clone());

            let mut package = DebPackage::new(&lulu.package.name);
            let provides: Vec<&str> = lulu.package.provides.iter().map(String::as_str).collect();
            let dependencies_runtime: Vec<&str> = lulu
                .dependencies
                .runtime
                .iter()
                .map(|d| d.0.as_str())
                .collect();
            let dependencies_optional: Vec<&str> = lulu
                .dependencies
                .optional
                .iter()
                .map(|d| d.0.as_str())
                .collect();

            package = package
                .set_version(&version)
                .set_description(&lulu.package.description)
                .set_architecture(DebArchitecture::Amd64)
                .set_maintainer(
                    lulu.package
                        .maintainers
                        .first()
                        .expect("There should be at least one maintener"),
                )
                .with_provides(provides)
                .with_depends(dependencies_runtime)
                .with_recommends(dependencies_optional);

            if lulu.package.preinst.is_some() {
                package = package.preinst_from_str(&lulu.package.preinst.unwrap());
            }

            if lulu.package.postinst.is_some() {
                package = package.postinst_from_str(&lulu.package.postinst.unwrap());
            }

            if lulu.package.prerm.is_some() {
                package = package.prerm_from_str(&lulu.package.prerm.unwrap());
            }

            if lulu.package.postrm.is_some() {
                package = package.postrm_from_str(&lulu.package.postrm.unwrap());
            }

            package = package
                .with_dir(pkgdir, std::path::Path::new("").to_path_buf())
                .unwrap();

            package
                .build()
                .unwrap()
                .write(File::create(format!("{}-{}.deb", lulu.package.name, version)).unwrap())
                .unwrap();

            std::process::exit(0);
        }
        Err(_) => error!("Fork failed"),
    }

    // Uninstalling
    title!("📦", "Uninstalling build dependencies");
    let cache = match Cache::new::<bool>(&[]) {
        Ok(c) => c,
        Err(_) => todo!(),
    };
    let to_uninstall: Vec<AptPackage> = to_uninstall
        .into_iter()
        .map(|pkg| cache.get(&pkg).expect("Should be in the cache"))
        .collect();
    for pkg in to_uninstall {
        pkg.mark_delete(true);
        pkg.protect();
    }
    let mut acquire_progress = AptAcquireProgress::new_box();
    let mut install_progress = AptInstallProgress::new_box();
    match cache.commit(&mut acquire_progress, &mut install_progress) {
        Ok(_) => {}
        Err(_) => {
            error!("Failed to uninstall build packages");
            panic!("Failed to uninstall build packages");
        }
    };

    // Verifying if status is ok
    if status != 0 {
        error!("Something went wrong");
        panic!("Something went wrong");
    }

    // Installing built package
    if !ctx.no_install {
        title!(
            "📦",
            "Installing {}",
            Paint::cyan(lulu.package.name.clone()).italic()
        );
        let cache = match Cache::new::<&str>(&[Path::new(&format!(
            "{}-{}.deb",
            lulu.package.name, version
        ))
        .to_str()
        .expect("Path should exist")])
        {
            Ok(c) => c,
            Err(_) => todo!(),
        };
        let package = match cache.get(&lulu.package.name) {
            Some(p) => p,
            None => {
                error!("Package not found");
                panic!("Package not found");
            }
        };

        println!(
            "{}",
            package
                .installed()
                .map_or("Not installed".to_string(), |v| v.version().to_string())
        );
        package.mark_install(true, true);
        package.protect();

        cache.resolve(true).unwrap();

        let mut acquire_progress = AptAcquireProgress::new_box();
        let mut install_progress = AptInstallProgress::new_box();

        match cache.get_archives(&mut acquire_progress) {
            Ok(_) => match cache.do_install(&mut install_progress) {
                Ok(_) => (),
                Err(e) => panic!("{:?}", e),
            },
            Err(e) => panic!("{:?}", e),
        }

        match ctx
            .db
            .collection("installed")
            .doc(lulu.package.name.as_str())
            .set(Installed {
                version,
                hash: repo
                    .head()
                    .expect("There should be at least one commit")
                    .target()
                    .expect("The commit should point to a ref")
                    .to_string(),
                source: lulu.package.source,
            }) {
            Ok(_) => {}
            Err(e) => {
                panic!("{:?}", e);
            }
        };
    }

    success!("Done");
}

fn generate(lulu: Lulu, basedir: PathBuf, srcdir: PathBuf, pkgdir: PathBuf) {
    // Prepare
    title!("🔧", "Preparing");
    if lulu.script.prepare.is_some() {
        if !Command::new("bash")
            .env("basedir", basedir.display().to_string())
            .env("srcdir", srcdir.display().to_string())
            .env("pkgdir", pkgdir.display().to_string())
            .arg("-ec")
            .arg(lulu.script.prepare.unwrap())
            .spawn()
            .expect("Failed to execute command")
            .wait()
            .unwrap()
            .success()
        {
            error!("Prepare failed");
            std::process::exit(1);
        }
    }

    // Build
    title!("🔨", "Building");
    env::set_current_dir(srcdir.display().to_string()).unwrap();
    if lulu.script.build.is_some() {
        if !Command::new("bash")
            .env("basedir", basedir.display().to_string())
            .env("srcdir", srcdir.display().to_string())
            .env("pkgdir", pkgdir.display().to_string())
            .arg("-ec")
            .arg(lulu.script.build.unwrap())
            .spawn()
            .expect("Failed to execute command")
            .wait()
            .unwrap()
            .success()
        {
            error!("Build failed");
            std::process::exit(1);
        }
    }

    // Test
    title!("🪃", "Testing");
    if lulu.script.check.is_some() {
        if !Command::new("bash")
            .env("basedir", basedir.display().to_string())
            .env("srcdir", srcdir.display().to_string())
            .env("pkgdir", pkgdir.display().to_string())
            .arg("-ec")
            .arg(lulu.script.check.unwrap())
            .spawn()
            .expect("Failed to execute command")
            .wait()
            .unwrap()
            .success()
        {
            error!("Test failed");
            std::process::exit(1);
        }
    }

    // Package
    title!("🔩", "Packaging");
    env::set_current_dir(srcdir.display().to_string()).unwrap();
    if !Command::new("fakeroot")
        .env("basedir", basedir.display().to_string())
        .env("srcdir", srcdir.display().to_string())
        .env("pkgdir", pkgdir.display().to_string())
        .arg("--")
        .arg("bash")
        .arg("-ec")
        .arg(lulu.script.package)
        .spawn()
        .expect("Failed to execute command")
        .wait()
        .unwrap()
        .success()
    {
        error!("Packaging failed");
        std::process::exit(1);
    }

    env::set_current_dir(basedir.display().to_string()).unwrap();
}

pub fn install(name: Option<String>, no_install: bool) {
    let db = match Db::new(Path::new("/var/lib/lulu/db").to_path_buf()) {
        Ok(db) => db,
        Err(e) => {
            error!("Failed to open database");
            panic!("{:?}", e);
        }
    };
    let ctx = Context { no_install, db };
    match name {
        Some(n) => {
            if n.contains("://") || n.starts_with("git@") {
                install_git(n, ctx)
            } else {
                install_db(n, ctx)
            }
        }
        None => install_local(ctx),
    }
}

#[derive(Clone)]
struct Context {
    pub no_install: bool,
    pub db: Db,
}
