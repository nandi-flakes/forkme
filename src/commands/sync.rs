use anyhow::Result;
use std::collections::HashSet;

use crate::config::Config;
use crate::git::{self, FileContent};
use crate::patch::{self, PatchEntry};

pub fn run() -> Result<()> {
    let config = Config::load()?;
    let repo = git::open_repo()?;
    git::ensure_on_forkme_branch(&repo)?;

    let changes = git::get_changes_from_upstream(&repo, &config.upstream.branch)?;

    if changes.is_empty() {
        println!("No changes from upstream. Patches are up to date.");
        return Ok(());
    }

    // Track which files have been processed
    let mut processed_files: HashSet<String> = HashSet::new();

    // Generate and save patches/binaries
    for change in &changes {
        // First, remove any existing entries for this file (clean slate)
        patch::delete_all_for_file(&change.path)?;

        match (&change.old_content, &change.new_content) {
            // File deleted
            (Some(_), None) => {
                patch::save_deleted_marker(&change.path)?;
                processed_files.insert(change.path.clone());
                println!("  deleted {}", change.path);
            }

            // File added or modified
            (old, Some(new_content)) => {
                let is_new = old.is_none();

                match new_content {
                    FileContent::Binary(bytes) => {
                        // Save binary file directly
                        patch::save_binary(&change.path, bytes)?;
                        processed_files.insert(change.path.clone());
                        let status = if is_new {
                            "added (binary)"
                        } else {
                            "modified (binary)"
                        };
                        println!("  {} {}", status, change.path);
                    }
                    FileContent::Text(new_text) => {
                        // Generate text patch
                        let old_text = old.as_ref().and_then(|c| c.as_text()).unwrap_or("");
                        let patch_content = patch::generate_patch(Some(old_text), Some(new_text));

                        // Skip empty patches (shouldn't happen, but just in case)
                        if patch_content.lines().count() <= 2 {
                            continue;
                        }

                        patch::save_patch(&change.path, &patch_content)?;
                        processed_files.insert(change.path.clone());
                        let status = if is_new { "added" } else { "modified" };
                        println!("  {} {}", status, change.path);
                    }
                }
            }

            // No content (shouldn't happen)
            (None, None) => continue,
        }
    }

    // Remove entries for files that are no longer modified
    let existing_entries = patch::list_all_entries()?;
    for entry in existing_entries {
        let file_path = entry.file_path();
        if !processed_files.contains(file_path) {
            patch::delete_all_for_file(file_path)?;
            let suffix = match entry {
                PatchEntry::TextPatch(_) => ".patch",
                PatchEntry::Binary(_) => " (binary)",
                PatchEntry::Deleted(_) => ".deleted",
            };
            println!("  removed {}{}", file_path, suffix);
        }
    }

    // Clean up empty directories
    patch::cleanup_empty_dirs()?;

    println!("\nSynced {} files.", processed_files.len());

    Ok(())
}
