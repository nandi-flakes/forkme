use anyhow::{bail, Context, Result};
use git2::{DiffOptions, Repository, ResetType};
use std::path::Path;

pub const SOURCE_DIR: &str = "source";
pub const FORKME_BRANCH: &str = "forkme";

pub fn clone_repo(url: &str, branch: &str, depth: Option<usize>) -> Result<Repository> {
    let depth_msg = match depth {
        Some(d) => format!(" with depth {}", d),
        None => String::new(),
    };
    println!("Cloning {} (branch: {}){}...", url, branch, depth_msg);

    let mut builder = git2::build::RepoBuilder::new();
    builder.branch(branch);

    // Set fetch depth if provided
    if let Some(d) = depth {
        let mut fetch_options = git2::FetchOptions::new();
        fetch_options.depth(d as i32);
        builder.fetch_options(fetch_options);
    }

    let repo = builder
        .clone(url, Path::new(SOURCE_DIR))
        .with_context(|| format!("Failed to clone {} into {}", url, SOURCE_DIR))?;

    println!("Clone complete.");
    Ok(repo)
}

pub fn open_repo() -> Result<Repository> {
    Repository::open(SOURCE_DIR)
        .with_context(|| format!("Failed to open repository at {}", SOURCE_DIR))
}

pub fn create_forkme_branch(repo: &Repository, upstream_branch: &str) -> Result<()> {
    let upstream_ref = format!("origin/{}", upstream_branch);
    let reference = repo
        .find_reference(&format!("refs/remotes/{}", upstream_ref))
        .with_context(|| format!("Failed to find remote branch {}", upstream_ref))?;

    let commit = reference.peel_to_commit()?;

    // Create the forkme branch
    repo.branch(FORKME_BRANCH, &commit, false)
        .with_context(|| format!("Failed to create branch {}", FORKME_BRANCH))?;

    // Checkout the forkme branch
    let obj = repo.revparse_single(&format!("refs/heads/{}", FORKME_BRANCH))?;
    repo.checkout_tree(&obj, None)?;
    repo.set_head(&format!("refs/heads/{}", FORKME_BRANCH))?;

    println!("Created and checked out branch '{}'", FORKME_BRANCH);
    Ok(())
}

pub fn get_upstream_commit(repo: &Repository, branch: &str) -> Result<git2::Oid> {
    let upstream_ref = format!("refs/remotes/origin/{}", branch);
    let reference = repo
        .find_reference(&upstream_ref)
        .with_context(|| format!("Failed to find upstream branch {}", branch))?;

    Ok(reference.peel_to_commit()?.id())
}

pub fn reset_to_upstream(repo: &Repository, branch: &str) -> Result<()> {
    let upstream_oid = get_upstream_commit(repo, branch)?;
    let commit = repo.find_commit(upstream_oid)?;
    let obj = commit.as_object();

    repo.reset(obj, ResetType::Hard, None)
        .with_context(|| "Failed to reset to upstream")?;

    println!("Reset to upstream {}", branch);
    Ok(())
}

pub fn is_working_tree_clean(repo: &Repository) -> Result<bool> {
    let statuses = repo.statuses(None)?;
    Ok(statuses.is_empty())
}

pub fn ensure_on_forkme_branch(repo: &Repository) -> Result<()> {
    let head = repo.head()?;
    let branch_name = head.shorthand().unwrap_or("");

    if branch_name != FORKME_BRANCH {
        bail!(
            "Not on '{}' branch. Currently on '{}'. Please checkout '{}'.",
            FORKME_BRANCH,
            branch_name,
            FORKME_BRANCH
        );
    }
    Ok(())
}

pub enum FileContent {
    Text(String),
    Binary(Vec<u8>),
}

impl FileContent {
    #[allow(dead_code)]
    pub fn is_binary(&self) -> bool {
        matches!(self, FileContent::Binary(_))
    }

    pub fn as_text(&self) -> Option<&str> {
        match self {
            FileContent::Text(s) => Some(s),
            FileContent::Binary(_) => None,
        }
    }

    #[allow(dead_code)]
    pub fn as_bytes(&self) -> &[u8] {
        match self {
            FileContent::Text(s) => s.as_bytes(),
            FileContent::Binary(b) => b,
        }
    }
}

pub struct FileDiff {
    pub path: String,
    pub old_content: Option<FileContent>,
    pub new_content: Option<FileContent>,
}

pub fn get_changes_from_upstream(
    repo: &Repository,
    upstream_branch: &str,
) -> Result<Vec<FileDiff>> {
    let upstream_oid = get_upstream_commit(repo, upstream_branch)?;
    let upstream_commit = repo.find_commit(upstream_oid)?;
    let upstream_tree = upstream_commit.tree()?;

    let head = repo.head()?;
    let head_commit = head.peel_to_commit()?;
    let head_tree = head_commit.tree()?;

    let mut diff_opts = DiffOptions::new();
    let diff =
        repo.diff_tree_to_tree(Some(&upstream_tree), Some(&head_tree), Some(&mut diff_opts))?;

    let mut changes = Vec::new();

    diff.foreach(
        &mut |delta, _| {
            let path = delta
                .new_file()
                .path()
                .or_else(|| delta.old_file().path())
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default();

            let old_content = if delta.old_file().id().is_zero() {
                None
            } else {
                repo.find_blob(delta.old_file().id())
                    .ok()
                    .map(|blob| blob_to_content(&blob))
            };

            let new_content = if delta.new_file().id().is_zero() {
                None
            } else {
                repo.find_blob(delta.new_file().id())
                    .ok()
                    .map(|blob| blob_to_content(&blob))
            };

            changes.push(FileDiff {
                path,
                old_content,
                new_content,
            });

            true
        },
        None,
        None,
        None,
    )?;

    Ok(changes)
}

fn blob_to_content(blob: &git2::Blob) -> FileContent {
    let bytes = blob.content();
    // Check if content is valid UTF-8 and doesn't contain null bytes
    if let Ok(text) = std::str::from_utf8(bytes) {
        if !bytes.contains(&0) {
            return FileContent::Text(text.to_string());
        }
    }
    FileContent::Binary(bytes.to_vec())
}

pub fn commit_changes(repo: &Repository, message: &str) -> Result<()> {
    let mut index = repo.index()?;
    index.add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)?;
    index.write()?;

    let tree_id = index.write_tree()?;
    let tree = repo.find_tree(tree_id)?;

    let head = repo.head()?;
    let parent = head.peel_to_commit()?;

    let sig = repo.signature()?;
    repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &[&parent])?;

    Ok(())
}
