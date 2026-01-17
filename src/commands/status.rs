use anyhow::Result;
use std::path::Path;

use crate::config::Config;
use crate::git::{self, SOURCE_DIR};
use crate::patch;

pub fn run() -> Result<()> {
    // Check if config exists
    if !Config::exists() {
        println!("Not a forkme project (no forkme.toml found)");
        return Ok(());
    }

    let config = Config::load()?;
    println!("Forkme project status");
    println!("=====================");
    println!();

    // Upstream info
    println!("Upstream:");
    println!("  URL:    {}", config.upstream.url);
    println!("  Branch: {}", config.upstream.branch);
    println!();

    // Source directory status
    println!("Source directory:");
    if Path::new(SOURCE_DIR).exists() {
        let repo = git::open_repo()?;

        // Current branch
        let head = repo.head()?;
        let branch_name = head.shorthand().unwrap_or("(detached)");
        println!("  Branch: {}", branch_name);

        if branch_name != git::FORKME_BRANCH {
            println!("  ⚠ Not on '{}' branch", git::FORKME_BRANCH);
        }

        if git::is_working_tree_clean(&repo)? {
            println!("  Working tree: clean");
        } else {
            println!("  Working tree: has uncommitted changes");
        }

        let changes = git::get_changes_from_upstream(&repo, &config.upstream.branch)?;
        let existing_patches = patch::list_patches()?;

        if changes.len() != existing_patches.len() {
            println!("  Patches: may be out of sync (run 'forkme sync' to update)");
        } else {
            println!("  Patches: {} files patched", existing_patches.len());
        }
    } else {
        println!("  Not initialized (run 'forkme init' first)");
    }

    println!();

    // Patches info
    println!("Patches directory:");
    let patches = patch::list_patches()?;
    if patches.is_empty() {
        println!("  No patches");
    } else {
        println!("  {} patch file(s)", patches.len());
    }

    Ok(())
}
