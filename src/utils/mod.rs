pub mod display;

pub mod git {
    use std::io::Write;

    fn do_fetch<'a>(
        repo: &'a git2::Repository,
        refs: &[&str],
        remote: &'a mut git2::Remote,
    ) -> Result<git2::AnnotatedCommit<'a>, git2::Error> {
        let mut cb = git2::RemoteCallbacks::new();

        // Print out our transfer progress.
        cb.transfer_progress(|stats| {
            if stats.received_objects() == stats.total_objects() {
                print!(
                    "Resolving deltas {}/{}\r",
                    stats.indexed_deltas(),
                    stats.total_deltas()
                );
            } else if stats.total_objects() > 0 {
                print!(
                    "Received {}/{} objects ({}) in {} bytes\r",
                    stats.received_objects(),
                    stats.total_objects(),
                    stats.indexed_objects(),
                    stats.received_bytes()
                );
            }
            std::io::stdout().flush().unwrap();
            true
        });

        let mut fo = git2::FetchOptions::new();
        fo.remote_callbacks(cb);
        // Always fetch all tags.
        // Perform a download and also update tips
        fo.download_tags(git2::AutotagOption::All);
        println!("Fetching {} for repo", remote.name().unwrap());
        remote.fetch(refs, Some(&mut fo), None)?;

        // If there are local objects (we got a thin pack), then tell the user
        // how many objects we saved from having to cross the network.
        let stats = remote.stats();
        if stats.local_objects() > 0 {
            println!(
                "\rReceived {}/{} objects in {} bytes (used {} local \
             objects)",
                stats.indexed_objects(),
                stats.total_objects(),
                stats.received_bytes(),
                stats.local_objects()
            );
        } else {
            println!(
                "\rReceived {}/{} objects in {} bytes",
                stats.indexed_objects(),
                stats.total_objects(),
                stats.received_bytes()
            );
        }

        let fetch_head = repo.find_reference("FETCH_HEAD")?;
        Ok(repo.reference_to_annotated_commit(&fetch_head)?)
    }

    fn fast_forward(
        repo: &git2::Repository,
        lb: &mut git2::Reference,
        rc: &git2::AnnotatedCommit,
    ) -> Result<(), git2::Error> {
        let name = match lb.name() {
            Some(s) => s.to_string(),
            None => String::from_utf8_lossy(lb.name_bytes()).to_string(),
        };
        let msg = format!("Fast-Forward: Setting {} to id: {}", name, rc.id());
        println!("{}", msg);
        lb.set_target(rc.id(), &msg)?;
        repo.set_head(&name)?;
        repo.checkout_head(Some(
            git2::build::CheckoutBuilder::default()
                // For some reason the force is required to make the working directory actually get updated
                // I suspect we should be adding some logic to handle dirty working directory states
                // but this is just an example so maybe not.
                .force(),
        ))?;
        Ok(())
    }

    fn normal_merge(
        repo: &git2::Repository,
        local: &git2::AnnotatedCommit,
        remote: &git2::AnnotatedCommit,
    ) -> Result<(), git2::Error> {
        let local_tree = repo.find_commit(local.id())?.tree()?;
        let remote_tree = repo.find_commit(remote.id())?.tree()?;
        let ancestor = repo
            .find_commit(repo.merge_base(local.id(), remote.id())?)?
            .tree()?;
        let mut idx = repo.merge_trees(&ancestor, &local_tree, &remote_tree, None)?;

        if idx.has_conflicts() {
            println!("Merge conflicts detected...");
            repo.checkout_index(Some(&mut idx), None)?;
            return Ok(());
        }
        let result_tree = repo.find_tree(idx.write_tree_to(repo)?)?;
        // now create the merge commit
        let msg = format!("Merge: {} into {}", remote.id(), local.id());
        let sig = repo.signature()?;
        let local_commit = repo.find_commit(local.id())?;
        let remote_commit = repo.find_commit(remote.id())?;
        // Do our merge commit and set current branch head to that commit.
        let _merge_commit = repo.commit(
            Some("HEAD"),
            &sig,
            &sig,
            &msg,
            &result_tree,
            &[&local_commit, &remote_commit],
        )?;
        // Set working tree to match head.
        repo.checkout_head(None)?;
        Ok(())
    }

    fn do_merge<'a>(
        repo: &'a git2::Repository,
        remote_branch: &str,
        fetch_commit: git2::AnnotatedCommit<'a>,
    ) -> Result<(), git2::Error> {
        // 1. do a merge analysis
        let analysis = repo.merge_analysis(&[&fetch_commit])?;

        // 2. Do the appropriate merge
        if analysis.0.is_fast_forward() {
            println!("Doing a fast forward");
            // do a fast forward
            let refname = format!("refs/heads/{}", remote_branch);
            match repo.find_reference(&refname) {
                Ok(mut r) => {
                    fast_forward(repo, &mut r, &fetch_commit)?;
                }
                Err(_) => {
                    // The branch doesn't exist so just set the reference to the
                    // commit directly. Usually this is because you are pulling
                    // into an empty repository.
                    repo.reference(
                        &refname,
                        fetch_commit.id(),
                        true,
                        &format!("Setting {} to {}", remote_branch, fetch_commit.id()),
                    )?;
                    repo.set_head(&refname)?;
                    repo.checkout_head(Some(
                        git2::build::CheckoutBuilder::default()
                            .allow_conflicts(true)
                            .conflict_style_merge(true)
                            .force(),
                    ))?;
                }
            };
        } else if analysis.0.is_normal() {
            // do a normal merge
            let head_commit = repo.reference_to_annotated_commit(&repo.head()?)?;
            normal_merge(&repo, &head_commit, &fetch_commit)?;
        } else {
            println!("Nothing to do...");
        }
        Ok(())
    }

    pub fn pull(
        repo: git2::Repository,
        remote_name: &str,
        remote_branch: &str,
    ) -> Result<(), git2::Error> {
        let mut remote = repo.find_remote(remote_name)?;
        let fetch_commit = do_fetch(&repo, &[remote_branch], &mut remote)?;
        do_merge(&repo, &remote_branch, fetch_commit)
    }
}

pub mod lulu {
    use crate::error;
    use crate::package::Lulu;
    use fork::{fork, Fork};
    use log::trace;
    use std::env;
    use std::io::{Error, Read};
    use std::path::Path;
    use yansi::{Color, Paint};

    pub fn lulu_file<P: AsRef<Path>>(path: P) -> Result<Result<Lulu, toml::de::Error>, Error> {
        let file = std::fs::File::open(path)?;
        let mut buf_reader = std::io::BufReader::new(file);
        let mut contents = String::new();
        buf_reader.read_to_string(&mut contents)?;
        Ok(toml::from_str(&contents))
    }

    pub fn fork_wait<F>(child: F) -> i32
    where
        F: Fn(),
    {
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
                let sudo = env::var("SUDO_USER");
                if sudo.is_ok() && sudo.unwrap() != "" {
                    let uid: u32 = env::var("SUDO_UID").unwrap().parse().unwrap();
                    let gid: u32 = env::var("SUDO_GID").unwrap().parse().unwrap();
                    unsafe { libc::setuid(uid) };
                    unsafe { libc::setgid(gid) };
                }

                child();

                std::process::exit(0);
            }
            Err(_) => error!("Fork failed"),
        }
        status
    }
}

pub mod db {
    use crate::db::Db;
    use crate::error;
    use std::io::Error;
    use std::path::Path;
    use yansi::{Color, Paint};

    pub fn open_db() -> Result<Db, Error> {
        let db = match Db::new(Path::new("/var/lib/lulu/db").to_path_buf()) {
            Ok(db) => db,
            Err(e) => {
                error!("Failed to open database");
                return Err(e);
            }
        };

        Ok(db)
    }

    pub fn open_and_lock_db() -> Result<Db, Error> {
        let mut db = open_db()?;

        match db.lock() {
            Ok(_) => {}
            Err(e) => {
                error!("Failed to lock database");
                return Err(e);
            }
        };

        Ok(db)
    }
}
