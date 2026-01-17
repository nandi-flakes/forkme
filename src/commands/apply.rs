use anyhow::{bail, Result};
use git2::Repository;
use std::fs;
use std::path::Path;

use crate::config::Config;
use crate::git::{self, SOURCE_DIR};
use crate::patch::{self, PatchEntry};

pub fn run() -> Result<()> {
    let config = Config::load()?;
    let repo = git::open_repo()?;

    if !git::is_working_tree_clean(&repo)? {
        bail!(
            "Working tree in {} has uncommitted changes. Please commit or stash them first.",
            SOURCE_DIR
        );
    }

    git::ensure_on_forkme_branch(&repo)?;
    git::reset_to_upstream(&repo, &config.upstream.branch)?;

    apply_patches(&repo)?;

    println!("\nPatches applied successfully.");

    Ok(())
}

pub fn apply_patches(repo: &Repository) -> Result<()> {
    let entries = patch::list_all_entries()?;

    if entries.is_empty() {
        println!("No patches to apply.");
        return Ok(());
    }

    println!("Applying {} entries...", entries.len());

    for entry in &entries {
        let file_path = entry.file_path();
        let source_file_path = Path::new(SOURCE_DIR).join(file_path);

        match entry {
            PatchEntry::Deleted(_) => {
                // Delete the file
                if source_file_path.exists() {
                    fs::remove_file(&source_file_path)?;
                    println!("  deleted {}", file_path);
                }
            }

            PatchEntry::Binary(_) => {
                // Copy binary file directly
                let content = patch::read_binary(file_path)?;
                if let Some(parent) = source_file_path.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::write(&source_file_path, content)?;
                println!("  copied (binary) {}", file_path);
            }

            PatchEntry::TextPatch(_) => {
                // Apply text patch
                let patch_content = patch::read_patch(file_path)?;

                // Detect if new file by checking hunk header
                let is_new_file = patch_content.lines().any(|l| l.starts_with("@@ -0,0"));

                if is_new_file {
                    // Create new file from patch
                    let new_content = patch::apply_patch("", &patch_content)?;
                    if let Some(parent) = source_file_path.parent() {
                        fs::create_dir_all(parent)?;
                    }
                    fs::write(&source_file_path, new_content)?;
                    println!("  created {}", file_path);
                } else {
                    // Modify existing file
                    let original = fs::read_to_string(&source_file_path)?;
                    let patched = patch::apply_patch(&original, &patch_content)?;
                    fs::write(&source_file_path, patched)?;
                    println!("  patched {}", file_path);
                }
            }
        }
    }

    // Commit the changes
    if !entries.is_empty() {
        git::commit_changes(repo, "Apply forkme patches")?;
        println!("Committed patched changes.");
    }

    Ok(())
}
