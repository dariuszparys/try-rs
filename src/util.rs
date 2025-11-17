use dirs::home_dir;
use std::path::Path;
use std::path::PathBuf;

// Time constants
const SECONDS_PER_DAY: u64 = 86_400;

// Date-prefix parsing constants for YYYY-MM-DD-
const DATE_PREFIX_TOTAL_LEN: usize = 11; // "YYYY-MM-DD-"
const YEAR_HYPHEN_POS: usize = 4;
const MONTH_HYPHEN_POS: usize = 7;
const DAY_HYPHEN_POS: usize = 10;

/// Expands a leading `~/` to the user's home directory; returns the original path otherwise.
pub(crate) fn shellexpand_home(p: &str) -> PathBuf {
    if p.starts_with("~/")
        && let Some(home) = home_dir()
    {
        return home.join(&p[2..]);
    }
    PathBuf::from(p)
}

/// Returns today's date prefix in UTC as `YYYY-MM-DD` using a civil-from-days conversion.
pub(crate) fn today_prefix() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let days = (secs / SECONDS_PER_DAY) as i64;
    let (y, m, d) = civil_from_days(days);
    format!("{y:04}-{m:02}-{d:02}")
}

/// Howard Hinnant civil-from-days (UTC)
pub(crate) fn civil_from_days(days: i64) -> (i32, u32, u32) {
    // Constants from the algorithm
    const DAYS_FROM_CE: i64 = 719_468;
    const DAYS_PER_ERA: i64 = 146_097;
    const ERA_ADJUST: i64 = 146_096;
    const DAYS_PER_4_YEARS: i64 = 1_460;
    const DAYS_PER_100_YEARS: i64 = 36_524;
    const MONTHS_BLOCK: i64 = 153;
    const MONTH_SHIFT_THRESHOLD: i64 = 10;
    const MONTH_SHIFT_BEFORE: i64 = 3;
    const MONTH_SHIFT_AFTER: i64 = -9;

    let z = days + DAYS_FROM_CE;
    let era = if z >= 0 { z } else { z - ERA_ADJUST } / DAYS_PER_ERA;
    let doe = z - era * DAYS_PER_ERA;
    let yoe = (doe - doe / DAYS_PER_4_YEARS + doe / DAYS_PER_100_YEARS - doe / ERA_ADJUST) / 365;
    let mut y = (yoe + era * 400) as i32;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / MONTHS_BLOCK;
    let d = doy - (MONTHS_BLOCK * mp + 2) / 5 + 1;
    let m = mp
        + if mp < MONTH_SHIFT_THRESHOLD {
            MONTH_SHIFT_BEFORE
        } else {
            MONTH_SHIFT_AFTER
        };
    if m <= 2 {
        y += 1;
    }
    (y, m as u32, d as u32)
}

/// If the input begins with `YYYY-MM-DD-...`, returns that date part and the remainder.
pub(crate) fn split_date_prefixed(s: &str) -> Option<(&str, &str)> {
    // Check for YYYY-MM-DD- pattern at the beginning
    if s.len() >= DATE_PREFIX_TOTAL_LEN
        && s.as_bytes().get(YEAR_HYPHEN_POS) == Some(&b'-')
        && s.as_bytes().get(MONTH_HYPHEN_POS) == Some(&b'-')
        && s.as_bytes().get(DAY_HYPHEN_POS) == Some(&b'-')
    {
        // Return date part (YYYY-MM-DD) and the rest after the third dash
        return Some((&s[0..10], &s[11..]));
    }
    None
}

/// Shell-escapes a path using single quotes suitable for POSIX shells.
pub(crate) fn shell_escape(p: PathBuf) -> String {
    let s = p.to_string_lossy();
    let esc = s.replace('\'', r#"'\''"#);
    format!("'{esc}'")
}

/// Filters a free-form query to a safe subset of characters for display and matching.
pub(crate) fn sanitize_query(q: &str) -> String {
    q.chars()
        .filter(|&c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | ' '))
        .collect()
}

/// Returns whether a typed character should be accepted into the query buffer.
pub(crate) fn is_printable(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.' | ' ')
}

/// Extracts `--flag value` and `--flag=value` from `args`, removing all occurrences; returns the last value.
/// Kept for test coverage and backward-compat benchmarks; parsing now uses `clap`.
#[allow(dead_code)]
pub(crate) fn extract_option_with_value(
    args: &mut Vec<std::ffi::OsString>,
    flag: &str,
) -> Option<String> {
    // Remove all occurrences of the flag and return the last value encountered.
    let mut result: Option<String> = None;
    let mut i = args.len();
    while i > 0 {
        i -= 1;
        if let Some(s) = args[i].to_str() {
            if s == flag {
                let val = args
                    .get(i + 1)
                    .and_then(|o| o.to_str())
                    .map(|s| s.to_string());
                // Remove value (if present) then the flag
                if args.len() > i + 1 {
                    args.remove(i + 1);
                }
                args.remove(i);
                if result.is_none() {
                    result = val;
                }
            } else if let Some(rest) = s.strip_prefix(&format!("{flag}=")) {
                let val = rest.to_string();
                args.remove(i);
                if result.is_none() {
                    result = Some(val);
                }
            }
        }
    }
    result
}

/// Returns true if the current shell appears to be fish (based on SHELL env path).
pub(crate) fn is_fish_shell() -> bool {
    std::env::var("SHELL")
        .map(|s| s.contains("fish"))
        .unwrap_or(false)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GitUri {
    pub host: String,
    pub user: String,
    pub repo: String,
}

/// Attempts to parse a Git URI of common forms into host/user/repo.
/// Supports:
/// - https://github.com/user/repo(.git)
/// - git@github.com:user/repo(.git)
/// - https://host/user/repo
/// - git@host:user/repo
pub(crate) fn parse_git_uri(input: &str) -> Option<GitUri> {
    let mut uri = input.trim().to_string();
    if let Some(rest) = uri.strip_suffix(".git") {
        uri = rest.to_string();
    }
    // https://host/user/repo
    if let Some(rest) = uri
        .strip_prefix("http://")
        .or_else(|| uri.strip_prefix("https://"))
    {
        let parts: Vec<&str> = rest.split('/').collect();
        if parts.len() >= 3 {
            let host = parts[0].to_string();
            let user = parts[1].to_string();
            let repo = parts[2].to_string();
            return Some(GitUri { host, user, repo });
        }
        return None;
    }
    // git@host:user/repo
    if let Some(rest) = uri.strip_prefix("git@") {
        let mut it = rest.split(':');
        let host = it.next()?.to_string();
        let path = it.next()?;
        let mut it2 = path.split('/');
        let user = it2.next()?.to_string();
        let repo = it2.next()?.to_string();
        return Some(GitUri { host, user, repo });
    }
    None
}

/// Heuristic to decide if an argument looks like a git URI.
pub(crate) fn is_git_uri(arg: &str) -> bool {
    let a = arg.trim();
    a.starts_with("http://")
        || a.starts_with("https://")
        || a.starts_with("git@")
        || a.contains("github.com")
        || a.contains("gitlab.com")
        || a.ends_with(".git")
}

/// Generate directory name for cloning.
/// If `custom_name` provided and non-empty, returns it as-is; otherwise uses
/// `YYYY-MM-DD-user-repo` based on the parsed git URI.
pub(crate) fn generate_clone_directory_name(
    git_uri: &str,
    custom_name: Option<&str>,
) -> Option<String> {
    if let Some(n) = custom_name
        && !n.is_empty()
    {
        return Some(n.to_string());
    }
    let parsed = parse_git_uri(git_uri)?;
    let date_prefix = today_prefix();
    Some(format!("{}-{}-{}", date_prefix, parsed.user, parsed.repo))
}

/// Join commands with ` && `, returning a single shell-evaluable line.
pub(crate) fn join_shell(parts: &[String]) -> String {
    parts.join(" && ")
}

/// Build a shell assignment for directory variable depending on shell.
pub(crate) fn dir_assign_for_shell(dir: &Path) -> String {
    let escaped = shell_escape(dir.to_path_buf());
    if is_fish_shell() {
        format!("set -l dir {escaped}")
    } else {
        format!("dir={escaped}")
    }
}

/// Format a byte size as a human-readable string (e.g., "1.5K", "23.4M").
pub(crate) fn format_human_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "K", "M", "G", "T"];
    const BYTES_PER_KIB: f64 = 1_024.0;
    let mut val = bytes as f64;
    let mut idx = 0;
    while val >= BYTES_PER_KIB && idx + 1 < UNITS.len() {
        val /= BYTES_PER_KIB;
        idx += 1;
    }
    if idx == 0 {
        format!("{bytes}B")
    } else {
        format!("{:.1}{}", val, UNITS[idx])
    }
}

/// Calculate the total size of a directory recursively.
pub(crate) fn calculate_dir_size(path: &Path) -> u64 {
    let mut total = 0u64;
    fn walk(p: &Path, total: &mut u64) {
        if let Ok(md) = std::fs::symlink_metadata(p) {
            if md.is_file() {
                *total += md.len();
            } else if md.is_dir()
                && let Ok(rd) = std::fs::read_dir(p)
            {
                for e in rd.flatten() {
                    walk(&e.path(), total);
                }
            }
        }
    }
    walk(path, &mut total);
    total
}
