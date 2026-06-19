//! Integration tests for forkme

use std::fs;
use tempfile::TempDir;

use forkme::config::{Config, Upstream};
use forkme::git::FileContent;
use forkme::patch::{self, PatchEntry};
use git2::Repository;

/// Helper to create a test git repository using git2
fn create_test_repo(dir: &std::path::Path) -> Repository {
    let repo = Repository::init(dir).unwrap();

    // Configure repo
    let mut config = repo.config().unwrap();
    config.set_str("user.email", "test@test.com").unwrap();
    config.set_str("user.name", "Test User").unwrap();

    repo
}

/// Helper to commit files to a repo
fn commit_files(repo: &Repository, files: &[(&str, &str)], message: &str) {
    let mut index = repo.index().unwrap();

    for (path, content) in files {
        let full_path = repo.workdir().unwrap().join(path);
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(&full_path, content).unwrap();
        index.add_path(std::path::Path::new(path)).unwrap();
    }

    index.write().unwrap();
    let tree_id = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();

    let sig = repo.signature().unwrap();

    // Get parent commit if exists
    let parent = repo.head().ok().and_then(|h| h.peel_to_commit().ok());

    match parent {
        Some(p) => {
            repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &[&p])
                .unwrap();
        }
        None => {
            repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &[])
                .unwrap();
        }
    }
}

#[test]
fn test_patch_generation_and_application() {
    let original = "line1\nline2\nline3\n";
    let modified = "line1\nmodified_line2\nline3\nnew_line4\n";

    // Generate patch
    let patch_content = patch::generate_patch(Some(original), Some(modified));

    // Verify patch contains expected changes
    assert!(patch_content.contains("-line2"));
    assert!(patch_content.contains("+modified_line2"));
    assert!(patch_content.contains("+new_line4"));

    // Apply patch
    let result = patch::apply_patch(original, &patch_content).unwrap();
    assert_eq!(result, modified);
}

#[test]
fn test_config_roundtrip() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("forkme.toml");

    let config = Config {
        upstream: Upstream {
            url: "https://github.com/example/repo.git".to_string(),
            branch: "main".to_string(),
        },
    };

    config.save_to(&config_path).unwrap();

    let loaded = Config::load_from(&config_path).unwrap();
    assert_eq!(loaded.upstream.url, config.upstream.url);
    assert_eq!(loaded.upstream.branch, config.upstream.branch);
}

#[test]
fn test_patch_entry_types() {
    let text = PatchEntry::TextPatch("src/lib.rs".to_string());
    let binary = PatchEntry::Binary("assets/logo.png".to_string());
    let deleted = PatchEntry::Deleted("old_file.rs".to_string());

    assert_eq!(text.file_path(), "src/lib.rs");
    assert_eq!(binary.file_path(), "assets/logo.png");
    assert_eq!(deleted.file_path(), "old_file.rs");
}

#[test]
fn test_new_file_patch() {
    // Create a patch for a completely new file
    let new_content = "pub fn new_function() {\n    // todo\n}\n";
    let patch_content = patch::generate_patch(None, Some(new_content));

    // Should indicate this is a new file (starts from nothing)
    assert!(patch_content.contains("@@ -0,0"));

    // Apply to empty string should work
    let result = patch::apply_patch("", &patch_content).unwrap();
    assert_eq!(result, new_content);
}

#[test]
fn test_file_deletion_patch() {
    let old_content = "this file will be deleted\n";
    let patch_content = patch::generate_patch(Some(old_content), None);

    // Should indicate deletion
    assert!(patch_content.contains("-this file will be deleted"));
}

#[test]
fn test_file_content_text_detection() {
    let text = FileContent::Text("hello world".to_string());
    assert!(!text.is_binary());
    assert_eq!(text.as_text(), Some("hello world"));
}

#[test]
fn test_file_content_binary_detection() {
    let binary = FileContent::Binary(vec![0x89, 0x50, 0x4E, 0x47]); // PNG magic bytes
    assert!(binary.is_binary());
    assert_eq!(binary.as_text(), None);
    assert_eq!(binary.as_bytes(), &[0x89, 0x50, 0x4E, 0x47]);
}

#[test]
fn test_create_repo_with_git2() {
    let temp_dir = TempDir::new().unwrap();
    let repo = create_test_repo(temp_dir.path());

    // Create initial commit
    commit_files(
        &repo,
        &[("README.md", "# Test\n"), ("src/main.rs", "fn main() {}\n")],
        "Initial commit",
    );

    // Verify commit exists
    let head = repo.head().unwrap();
    let commit = head.peel_to_commit().unwrap();
    assert_eq!(commit.message().unwrap(), "Initial commit");

    // Verify files exist
    assert!(temp_dir.path().join("README.md").exists());
    assert!(temp_dir.path().join("src/main.rs").exists());
}

#[test]
fn test_repo_diff_detection() {
    let temp_dir = TempDir::new().unwrap();
    let repo = create_test_repo(temp_dir.path());

    // Create initial commit
    commit_files(&repo, &[("file.txt", "original\n")], "Initial");

    // Create second commit with changes
    commit_files(&repo, &[("file.txt", "modified\n")], "Modified");

    // Get the two commits
    let head = repo.head().unwrap().peel_to_commit().unwrap();
    let parent = head.parent(0).unwrap();

    // Diff between commits
    let old_tree = parent.tree().unwrap();
    let new_tree = head.tree().unwrap();
    let diff = repo
        .diff_tree_to_tree(Some(&old_tree), Some(&new_tree), None)
        .unwrap();

    // Should have one file changed
    assert_eq!(diff.deltas().count(), 1);
}

/// End-to-end test simulating the forkme workflow:
/// 1. Create upstream repo with initial commit on main
/// 2. Clone and create forkme branch
/// 3. Make changes on forkme branch
/// 4. Use library functions to generate patches
/// 5. Verify patches can be applied to restore changes
#[test]
fn test_full_sync_workflow() {
    // Create "upstream" repo
    let upstream_dir = TempDir::new().unwrap();
    let upstream_repo = create_test_repo(upstream_dir.path());

    // Initial commit on main
    commit_files(
        &upstream_repo,
        &[
            ("README.md", "# Original Project\n"),
            ("src/lib.rs", "pub fn original() {}\n"),
        ],
        "Initial commit",
    );

    // Create "local" repo (simulating clone)
    let local_dir = TempDir::new().unwrap();
    let local_repo =
        Repository::clone(upstream_dir.path().to_str().unwrap(), local_dir.path()).unwrap();

    // Configure local repo
    let mut config = local_repo.config().unwrap();
    config.set_str("user.email", "test@test.com").unwrap();
    config.set_str("user.name", "Test User").unwrap();

    // Create forkme branch from origin/main (which is now called origin/master or main)
    let head = local_repo.head().unwrap();
    let head_commit = head.peel_to_commit().unwrap();
    local_repo.branch("forkme", &head_commit, false).unwrap();

    // Checkout forkme branch
    let obj = local_repo.revparse_single("refs/heads/forkme").unwrap();
    local_repo.checkout_tree(&obj, None).unwrap();
    local_repo.set_head("refs/heads/forkme").unwrap();

    // Make changes on forkme branch
    commit_files(
        &local_repo,
        &[
            ("README.md", "# Modified Project\n\nWith extra content.\n"),
            (
                "src/lib.rs",
                "pub fn original() {}\n\npub fn new_function() {\n    // added by fork\n}\n",
            ),
            (
                "src/new_file.rs",
                "// Completely new file\npub fn fork_only() {}\n",
            ),
        ],
        "Fork modifications",
    );

    // Now simulate what sync does: get changes between upstream and forkme
    // First, find the upstream commit (origin/master or origin/main)
    let upstream_ref = local_repo
        .find_reference("refs/remotes/origin/master")
        .or_else(|_| local_repo.find_reference("refs/remotes/origin/main"))
        .unwrap();
    let upstream_commit = upstream_ref.peel_to_commit().unwrap();
    let upstream_tree = upstream_commit.tree().unwrap();

    let forkme_head = local_repo.head().unwrap().peel_to_commit().unwrap();
    let forkme_tree = forkme_head.tree().unwrap();

    // Get diff
    let diff = local_repo
        .diff_tree_to_tree(Some(&upstream_tree), Some(&forkme_tree), None)
        .unwrap();

    // Should have 3 files changed
    assert_eq!(diff.deltas().count(), 3);

    // Collect changes and generate patches
    let mut patches: Vec<(String, String)> = Vec::new();

    diff.foreach(
        &mut |delta, _| {
            let path = delta
                .new_file()
                .path()
                .unwrap()
                .to_string_lossy()
                .to_string();

            let old_content = if delta.old_file().id().is_zero() {
                None
            } else {
                local_repo
                    .find_blob(delta.old_file().id())
                    .ok()
                    .and_then(|blob| String::from_utf8(blob.content().to_vec()).ok())
            };

            let new_content = if delta.new_file().id().is_zero() {
                None
            } else {
                local_repo
                    .find_blob(delta.new_file().id())
                    .ok()
                    .and_then(|blob| String::from_utf8(blob.content().to_vec()).ok())
            };

            let patch_content =
                patch::generate_patch(old_content.as_deref(), new_content.as_deref());
            patches.push((path, patch_content));
            true
        },
        None,
        None,
        None,
    )
    .unwrap();

    // Verify we have patches for all 3 files
    assert_eq!(patches.len(), 3);

    // Verify README patch contains the changes
    let readme_patch = patches.iter().find(|(p, _)| p == "README.md").unwrap();
    assert!(readme_patch.1.contains("-# Original Project"));
    assert!(readme_patch.1.contains("+# Modified Project"));

    // Verify new file patch
    let new_file_patch = patches
        .iter()
        .find(|(p, _)| p == "src/new_file.rs")
        .unwrap();
    assert!(new_file_patch.1.contains("@@ -0,0")); // New file indicator

    // Test applying patches to original content restores the fork changes
    let original_readme = "# Original Project\n";
    let patched_readme = patch::apply_patch(original_readme, &readme_patch.1).unwrap();
    assert_eq!(
        patched_readme,
        "# Modified Project\n\nWith extra content.\n"
    );

    // Test new file can be created from patch
    let new_file_content = patch::apply_patch("", &new_file_patch.1).unwrap();
    assert!(new_file_content.contains("pub fn fork_only()"));
}
