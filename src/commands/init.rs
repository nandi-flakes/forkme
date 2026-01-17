use anyhow::{bail, Context, Result};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;

use crate::config::{Config, Upstream};
use crate::git::{self, SOURCE_DIR};
use crate::patch;

pub fn run(url: Option<String>, branch: &str, depth: Option<usize>) -> Result<()> {
    // Determine the URL - either from argument or existing config
    let url = match url {
        Some(u) => u,
        None => {
            if Config::exists() {
                let config = Config::load()?;
                config.upstream.url
            } else {
                bail!("No URL provided and no forkme.toml found. Use --url to specify upstream repository.");
            }
        }
    };

    // Check if source directory already exists
    if Path::new(SOURCE_DIR).exists() {
        bail!(
            "Source directory '{}' already exists. Remove it first if you want to reinitialize.",
            SOURCE_DIR
        );
    }

    // Create/update the config file
    let config = Config {
        upstream: Upstream {
            url: url.clone(),
            branch: branch.into(),
        },
    };
    config
        .save()
        .with_context(|| "Failed to save forkme.toml")?;
    println!("Created forkme.toml");

    // Clone the repository
    let repo = git::clone_repo(&url, branch, depth)?;

    // Create the forkme branch
    git::create_forkme_branch(&repo, branch)?;

    // Create patches directory
    patch::ensure_patches_dir()?;
    println!("Created patches/ directory");

    // Add source/ to .gitignore if not already there
    add_to_gitignore(SOURCE_DIR)?;

    // Apply any existing patches
    let patches = patch::list_patches()?;
    if !patches.is_empty() {
        println!("Found {} existing patches, applying...", patches.len());
        super::apply::apply_patches(&repo)?;
    }

    println!("\nInitialization complete!");
    println!("You can now work in the {} directory.", SOURCE_DIR);
    println!("Use 'forkme sync' to save your changes as patches.");

    Ok(())
}

fn add_to_gitignore(entry: &str) -> Result<()> {
    let gitignore_path = Path::new(".gitignore");
    let entry_line = format!("{}/", entry);

    // Check if .gitignore exists and if entry is already there
    if gitignore_path.exists() {
        let content = fs::read_to_string(gitignore_path)?;
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed == entry
                || trimmed == entry_line.trim_end_matches('/')
                || trimmed == entry_line
            {
                println!(".gitignore already contains {}", entry);
                return Ok(());
            }
        }
    }

    // Append to .gitignore
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(gitignore_path)?;

    // Add newline before if file doesn't end with one
    if gitignore_path.exists() {
        let content = fs::read_to_string(gitignore_path)?;
        if !content.is_empty() && !content.ends_with('\n') {
            writeln!(file)?;
        }
    }

    writeln!(file, "{}", entry_line)?;
    println!("Added {} to .gitignore", entry_line);

    Ok(())
}
