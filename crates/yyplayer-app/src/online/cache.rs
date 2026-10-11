//! Evict only SHA-256 named files inside the application's managed subdirectories.
use std::path::Path;
pub fn trim(directory: &Path, extension: &str, limit: u64, keep: &Path) {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return;
    };
    let mut files = vec![];
    let mut total = 0u64;
    for entry in entries.flatten().take(10000) {
        let path = entry.path();
        let stem = path
            .file_stem()
            .and_then(|n| n.to_str())
            .unwrap_or_default();
        if path.extension().and_then(|e| e.to_str()) != Some(extension)
            || stem.len() != 64
            || !stem.bytes().all(|c| c.is_ascii_hexdigit())
        {
            continue;
        }
        let Ok(meta) = std::fs::symlink_metadata(&path) else {
            continue;
        };
        if !meta.file_type().is_file() {
            continue;
        }
        total = total.saturating_add(meta.len());
        files.push((meta.modified().ok(), meta.len(), path));
    }
    files.sort_by_key(|(time, _, _)| *time);
    for (_, size, path) in files {
        if total <= limit {
            break;
        }
        if path != keep && std::fs::remove_file(path).is_ok() {
            total = total.saturating_sub(size);
        }
    }
}
