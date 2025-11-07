use std::ffi::OsString;
use std::io;
use std::path::Path;

use crate::error::Result;
use crate::selector::{ActionType, TrySelector};
use crate::storage::fast_create_target_if_no_exact;
use crate::tui;
use crate::util::{dir_assign_for_shell, generate_clone_directory_name, is_git_uri, join_shell};

pub(crate) fn run_cd_flow(query_str: String, base_path: &Path) -> Result<()> {
    let trimmed = query_str.trim();
    // Shorthand: if query looks like a git URI, produce a clone pipeline
    if !trimmed.is_empty() && is_git_uri(trimmed) {
        if let Some(dir_name) = generate_clone_directory_name(trimmed, None) {
            let full = base_path.join(dir_name);
            let parts: Vec<String> = vec![
                dir_assign_for_shell(&full),
                "mkdir -p \"$dir\"".into(),
                format!("git clone '{}' \"$dir\"", trimmed),
                "touch \"$dir\"".into(),
                "cd \"$dir\"".into(),
            ];
            println!("{}", join_shell(&parts));
            return Ok(());
        } else {
            let mut err = io::stderr();
            let _ = tui::warn(&mut err, &format!("Unable to parse git URI: {trimmed}"));
            return Ok(());
        }
    }

    if !trimmed.is_empty()
        && let Some(dir) = fast_create_target_if_no_exact(base_path, trimmed)?
    {
        let parts: Vec<String> = vec![
            dir_assign_for_shell(&dir),
            "mkdir -p \"$dir\"".into(),
            "touch \"$dir\"".into(),
            "cd \"$dir\"".into(),
        ];
        println!("{}", join_shell(&parts));
        return Ok(());
    }

    let mut selector = TrySelector::new(&query_str, base_path.to_path_buf())?;
    if let Some(sel) = selector.run()?
        && let Some(dir) = sel.path
    {
        let mut parts: Vec<String> = vec![dir_assign_for_shell(&dir)];
        match sel.kind {
            ActionType::Mkdir => {
                parts.push(r#"mkdir -p "$dir""#.into());
                parts.push(r#"touch "$dir""#.into());
                parts.push(r#"cd "$dir""#.into());
            }
            ActionType::Cd => {
                parts.push(r#"touch "$dir""#.into());
                parts.push(r#"cd "$dir""#.into());
            }
            ActionType::Cancel => {}
        }
        println!("{}", parts.join(" && "));
    }
    Ok(())
}

/// Build the fuzzy query for the `cd` command from remaining args, removing a
/// redundant leading "cd" token if present.
pub(crate) fn build_cd_query(rest: &[OsString]) -> String {
    let mut parts: Vec<String> = rest
        .iter()
        .map(|s| s.to_string_lossy().to_string())
        .collect();
    if parts.first().map(|s| s == "cd").unwrap_or(false) {
        parts.remove(0);
    }
    parts.join(" ")
}
