use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::util::{split_date_prefixed, today_prefix};

/// Normalize a user query for exact-match comparison: sanitize allowed chars and
/// replace consecutive whitespace with single '-'.
pub(crate) fn normalize_query_for_match(query: &str) -> String {
    let sanitized = crate::util::sanitize_query(query);
    let mut out = String::with_capacity(sanitized.len());
    let mut last_dash = false;
    for ch in sanitized.chars() {
        if ch.is_whitespace() {
            if !last_dash {
                out.push('-');
                last_dash = true;
            }
        } else {
            out.push(ch);
            last_dash = false;
        }
    }
    out
}

/// Return Some(new_dir_path) if no exact match exists under root for the given query,
/// otherwise None. Exact match ignores a leading date prefix on existing entries.
pub(crate) fn fast_create_target_if_no_exact(
    root: &Path,
    query: &str,
) -> io::Result<Option<PathBuf>> {
    let norm = normalize_query_for_match(query);
    // Scan existing directories for an exact match (ignoring date prefix)
    if let Ok(entries) = fs::read_dir(root) {
        for e in entries.flatten() {
            let Ok(meta) = e.metadata() else { continue };
            if !meta.is_dir() {
                continue;
            }
            let name = e.file_name().to_string_lossy().to_string();
            if name == ".try_trash" {
                continue;
            }
            let stripped = if let Some((_, rest)) = split_date_prefixed(&name) {
                rest.to_string()
            } else {
                name.clone()
            };
            if stripped == norm {
                return Ok(None); // exact match exists; do not fast-create
            }
        }
    }
    // No exact match: propose today's path
    let final_name = format!("{}-{}", today_prefix(), norm);
    Ok(Some(root.join(final_name)))
}
