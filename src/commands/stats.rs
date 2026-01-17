use anyhow::Result;

use crate::patch::{self, PatchEntry};

pub fn run() -> Result<()> {
    let entries = patch::list_all_entries()?;

    if entries.is_empty() {
        println!("No patches found.");
        return Ok(());
    }

    let mut text_added = 0;
    let mut text_modified = 0;
    let mut binary_added = 0;
    let mut deleted = 0;

    for entry in &entries {
        match entry {
            PatchEntry::Deleted(_) => {
                deleted += 1;
            }
            PatchEntry::Binary(_) => {
                // Binary files are always considered "added" since we store the full file
                // We could check if the file exists in upstream, but for simplicity
                // we'll classify based on whether there's an upstream version
                // For now, count all binaries as added (new files)
                // A more sophisticated approach would check the source repo
                binary_added += 1;
            }
            PatchEntry::TextPatch(file_path) => {
                let patch_content = patch::read_patch(file_path)?;
                match classify_text_patch(&patch_content) {
                    TextPatchType::Added => text_added += 1,
                    TextPatchType::Modified => text_modified += 1,
                }
            }
        }
    }

    let total_added = text_added + binary_added;
    let total_modified = text_modified;
    let total = entries.len();

    println!("Patch statistics");
    println!("================");
    println!();
    println!(
        "  Added:    {:>4} ({} text, {} binary)",
        total_added, text_added, binary_added
    );
    println!("  Modified: {:>4}", total_modified);
    println!("  Deleted:  {:>4}", deleted);
    println!("  ─────────────");
    println!("  Total:    {:>4}", total);

    Ok(())
}

enum TextPatchType {
    Added,
    Modified,
}

fn classify_text_patch(content: &str) -> TextPatchType {
    // Look for the first hunk header
    for line in content.lines() {
        if line.starts_with("@@") {
            // Parse hunk header: @@ -start,count +start,count @@
            // New file: -0,0 means nothing in original
            if line.contains("-0,0") {
                return TextPatchType::Added;
            }
            return TextPatchType::Modified;
        }
    }
    // Default to modified if we can't determine
    TextPatchType::Modified
}
