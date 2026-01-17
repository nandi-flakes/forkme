use anyhow::{Context, Result};
use diffy::{apply, create_patch, Patch};
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub const PATCHES_DIR: &str = "patches";

// Extension for text patches
const PATCH_EXT: &str = ".patch";
// Extension for deleted file markers
const DELETED_EXT: &str = ".deleted";

pub fn generate_patch(old_content: Option<&str>, new_content: Option<&str>) -> String {
    let old = old_content.unwrap_or("");
    let new = new_content.unwrap_or("");
    create_patch(old, new).to_string()
}

pub fn apply_patch(original: &str, patch_content: &str) -> Result<String> {
    let patch: Patch<'_, str> = Patch::from_str(patch_content)?;
    apply(original, &patch).with_context(|| "Failed to apply patch")
}

pub fn patch_path_for_file(file_path: &str) -> PathBuf {
    PathBuf::from(PATCHES_DIR).join(format!("{}{}", file_path, PATCH_EXT))
}

pub fn binary_path_for_file(file_path: &str) -> PathBuf {
    PathBuf::from(PATCHES_DIR).join(file_path)
}

pub fn deleted_path_for_file(file_path: &str) -> PathBuf {
    PathBuf::from(PATCHES_DIR).join(format!("{}{}", file_path, DELETED_EXT))
}

pub fn file_path_from_patch(patch_path: &Path) -> Option<String> {
    let patches_prefix = PathBuf::from(PATCHES_DIR);
    patch_path
        .strip_prefix(&patches_prefix)
        .ok()
        .and_then(|p| p.to_str())
        .map(|s| s.trim_end_matches(PATCH_EXT).to_string())
}

pub fn file_path_from_binary(binary_path: &Path) -> Option<String> {
    let patches_prefix = PathBuf::from(PATCHES_DIR);
    binary_path
        .strip_prefix(&patches_prefix)
        .ok()
        .and_then(|p| p.to_str())
        .map(|s| s.to_string())
}

pub fn file_path_from_deleted(deleted_path: &Path) -> Option<String> {
    let patches_prefix = PathBuf::from(PATCHES_DIR);
    deleted_path
        .strip_prefix(&patches_prefix)
        .ok()
        .and_then(|p| p.to_str())
        .map(|s| s.trim_end_matches(DELETED_EXT).to_string())
}

pub fn save_patch(file_path: &str, patch_content: &str) -> Result<()> {
    let patch_file = patch_path_for_file(file_path);

    if let Some(parent) = patch_file.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::write(&patch_file, patch_content)
        .with_context(|| format!("Failed to write patch file {}", patch_file.display()))?;

    Ok(())
}

pub fn save_binary(file_path: &str, content: &[u8]) -> Result<()> {
    let binary_file = binary_path_for_file(file_path);

    if let Some(parent) = binary_file.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::write(&binary_file, content)
        .with_context(|| format!("Failed to write binary file {}", binary_file.display()))?;

    Ok(())
}

pub fn save_deleted_marker(file_path: &str) -> Result<()> {
    let deleted_file = deleted_path_for_file(file_path);

    if let Some(parent) = deleted_file.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::write(&deleted_file, "")
        .with_context(|| format!("Failed to write deleted marker {}", deleted_file.display()))?;

    Ok(())
}

pub fn read_patch(file_path: &str) -> Result<String> {
    let patch_file = patch_path_for_file(file_path);
    fs::read_to_string(&patch_file)
        .with_context(|| format!("Failed to read patch file {}", patch_file.display()))
}

pub fn read_binary(file_path: &str) -> Result<Vec<u8>> {
    let binary_file = binary_path_for_file(file_path);
    fs::read(&binary_file)
        .with_context(|| format!("Failed to read binary file {}", binary_file.display()))
}

pub fn delete_patch(file_path: &str) -> Result<()> {
    let patch_file = patch_path_for_file(file_path);
    if patch_file.exists() {
        fs::remove_file(&patch_file)?;
    }
    Ok(())
}

pub fn delete_binary(file_path: &str) -> Result<()> {
    let binary_file = binary_path_for_file(file_path);
    if binary_file.exists() {
        fs::remove_file(&binary_file)?;
    }
    Ok(())
}

pub fn delete_deleted_marker(file_path: &str) -> Result<()> {
    let deleted_file = deleted_path_for_file(file_path);
    if deleted_file.exists() {
        fs::remove_file(&deleted_file)?;
    }
    Ok(())
}

pub fn delete_all_for_file(file_path: &str) -> Result<()> {
    delete_patch(file_path)?;
    delete_binary(file_path)?;
    delete_deleted_marker(file_path)?;
    Ok(())
}

#[derive(Debug, Clone, PartialEq)]
pub enum PatchEntry {
    TextPatch(String), // file path
    Binary(String),    // file path
    Deleted(String),   // file path
}

impl PatchEntry {
    pub fn file_path(&self) -> &str {
        match self {
            PatchEntry::TextPatch(p) => p,
            PatchEntry::Binary(p) => p,
            PatchEntry::Deleted(p) => p,
        }
    }
}

pub fn list_all_entries() -> Result<Vec<PatchEntry>> {
    let patches_dir = Path::new(PATCHES_DIR);
    if !patches_dir.exists() {
        return Ok(Vec::new());
    }

    let mut entries = Vec::new();

    for entry in WalkDir::new(patches_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        let path = entry.path();
        let path_str = path.to_string_lossy();

        if path_str.ends_with(PATCH_EXT) {
            if let Some(file_path) = file_path_from_patch(path) {
                entries.push(PatchEntry::TextPatch(file_path));
            }
        } else if path_str.ends_with(DELETED_EXT) {
            if let Some(file_path) = file_path_from_deleted(path) {
                entries.push(PatchEntry::Deleted(file_path));
            }
        } else {
            // It's a binary file (no special extension)
            if let Some(file_path) = file_path_from_binary(path) {
                entries.push(PatchEntry::Binary(file_path));
            }
        }
    }

    Ok(entries)
}

pub fn list_patches() -> Result<Vec<String>> {
    let entries = list_all_entries()?;
    Ok(entries
        .into_iter()
        .filter_map(|e| match e {
            PatchEntry::TextPatch(p) => Some(p),
            _ => None,
        })
        .collect())
}

pub fn ensure_patches_dir() -> Result<()> {
    fs::create_dir_all(PATCHES_DIR)?;
    Ok(())
}

pub fn cleanup_empty_dirs() -> Result<()> {
    let patches_dir = Path::new(PATCHES_DIR);
    if !patches_dir.exists() {
        return Ok(());
    }

    // Collect directories in reverse depth order (deepest first)
    let mut dirs: Vec<_> = WalkDir::new(patches_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_dir())
        .map(|e| e.path().to_path_buf())
        .collect();

    dirs.sort_by_key(|b| std::cmp::Reverse(b.components().count()));

    for dir in dirs {
        if dir == patches_dir {
            continue;
        }
        // Try to remove; will fail if not empty (which is fine)
        let _ = fs::remove_dir(&dir);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_generate_patch_addition() {
        let patch = generate_patch(None, Some("hello\nworld\n"));
        assert!(patch.contains("@@ -0,0 +1,2 @@"));
        assert!(patch.contains("+hello"));
        assert!(patch.contains("+world"));
    }

    #[test]
    fn test_generate_patch_modification() {
        let patch = generate_patch(Some("hello\n"), Some("hello\nworld\n"));
        assert!(patch.contains("+world"));
    }

    #[test]
    fn test_generate_patch_deletion() {
        let patch = generate_patch(Some("hello\nworld\n"), None);
        assert!(patch.contains("-hello"));
        assert!(patch.contains("-world"));
    }

    #[test]
    fn test_apply_patch_new_file() {
        let patch = generate_patch(None, Some("hello\nworld\n"));
        let result = apply_patch("", &patch).unwrap();
        assert_eq!(result, "hello\nworld\n");
    }

    #[test]
    fn test_apply_patch_modification() {
        let original = "line1\nline2\nline3\n";
        let modified = "line1\nmodified\nline3\n";
        let patch = generate_patch(Some(original), Some(modified));
        let result = apply_patch(original, &patch).unwrap();
        assert_eq!(result, modified);
    }

    #[test]
    fn test_patch_path_for_file() {
        let path = patch_path_for_file("src/main.rs");
        assert_eq!(path, PathBuf::from("patches/src/main.rs.patch"));
    }

    #[test]
    fn test_binary_path_for_file() {
        let path = binary_path_for_file("assets/logo.png");
        assert_eq!(path, PathBuf::from("patches/assets/logo.png"));
    }

    #[test]
    fn test_deleted_path_for_file() {
        let path = deleted_path_for_file("old/file.rs");
        assert_eq!(path, PathBuf::from("patches/old/file.rs.deleted"));
    }

    #[test]
    fn test_file_path_from_patch() {
        let patch_path = Path::new("patches/src/lib.rs.patch");
        let file_path = file_path_from_patch(patch_path);
        assert_eq!(file_path, Some("src/lib.rs".to_string()));
    }

    #[test]
    fn test_file_path_from_binary() {
        let binary_path = Path::new("patches/assets/image.png");
        let file_path = file_path_from_binary(binary_path);
        assert_eq!(file_path, Some("assets/image.png".to_string()));
    }

    #[test]
    fn test_file_path_from_deleted() {
        let deleted_path = Path::new("patches/old/removed.rs.deleted");
        let file_path = file_path_from_deleted(deleted_path);
        assert_eq!(file_path, Some("old/removed.rs".to_string()));
    }

    #[test]
    fn test_patch_entry_file_path() {
        let text = PatchEntry::TextPatch("src/main.rs".to_string());
        let binary = PatchEntry::Binary("logo.png".to_string());
        let deleted = PatchEntry::Deleted("old.rs".to_string());

        assert_eq!(text.file_path(), "src/main.rs");
        assert_eq!(binary.file_path(), "logo.png");
        assert_eq!(deleted.file_path(), "old.rs");
    }
}
