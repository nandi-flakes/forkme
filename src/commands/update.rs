use anyhow::{bail, Context, Result};
use std::process::Command;

use crate::config::Config;
use crate::git::{self, SOURCE_DIR};

pub fn run() -> Result<()> {
    let config = Config::load()?;
    let repo = git::open_repo()?;
    git::ensure_on_forkme_branch(&repo)?;

    if !git::is_working_tree_clean(&repo)? {
        bail!("Working tree has uncommitted changes. Please commit or stash them before updating.");
    }

    let upstream_branch = &config.upstream.branch;

    println!("Fetching from origin...");
    let fetch_status = Command::new("git")
        .args(["fetch", "origin"])
        .current_dir(SOURCE_DIR)
        .status()
        .context("Failed to run git fetch")?;

    if !fetch_status.success() {
        bail!("git fetch failed");
    }

    let head_before = repo.head()?.peel_to_commit()?.id();

    // Rebase onto upstream
    println!("Rebasing onto origin/{}...", upstream_branch);
    let rebase_status = Command::new("git")
        .args(["rebase", &format!("origin/{}", upstream_branch)])
        .current_dir(SOURCE_DIR)
        .status()
        .context("Failed to run git rebase")?;

    if !rebase_status.success() {
        println!();
        println!("Rebase encountered conflicts.");
        println!();
        println!("To resolve:");
        println!("  1. cd {}", SOURCE_DIR);
        println!("  2. Fix conflicts in the listed files");
        println!("  3. git add <fixed files>");
        println!("  4. git rebase --continue");
        println!("  5. Repeat until rebase is complete");
        println!("  6. Run 'forkme sync' to update patches");
        println!();
        println!("To abort the rebase: git rebase --abort");
        return Ok(());
    }

    // Check if anything changed
    let repo = git::open_repo()?; // Re-open to get fresh state
    let head_after = repo.head()?.peel_to_commit()?.id();

    if head_before == head_after {
        println!("Already up to date.");
    } else {
        println!();
        println!("Rebase successful!");
        println!("Run 'forkme sync' to update your patches.");
    }

    Ok(())
}
