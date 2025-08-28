mod cli;
mod error;
mod model;
mod score;
mod selector;
mod storage;
mod tui;
mod util;

use crate::error::Result;
use clap::error::ErrorKind;
use clap::{Parser, Subcommand};
use std::{env, ffi::OsString, io, path::PathBuf};

#[derive(Parser, Debug)]
#[command(
    name = "try",
    version,
    about = "Interactive try-dir selector",
    disable_help_subcommand = true
)]
struct Cli {
    /// Override base tries directory
    #[arg(long, global = true, value_name = "PATH")]
    path: Option<PathBuf>,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Initialize shell function for aliasing
    Init {
        /// Override base tries directory for generated alias
        #[arg(long, value_name = "PATH")]
        path: Option<PathBuf>,
        /// Optional absolute path positional to keep backward behavior
        #[arg(value_name = "PATH")]
        abs_path: Option<PathBuf>,
    },
    /// Interactive selector; prints shell cd commands
    Cd {
        /// Query terms; use `--` before hyphen-leading terms
        #[arg(value_name = "QUERY", trailing_var_arg = true)]
        query: Vec<String>,
    },
    /// Clone git repo into date-prefixed directory
    Clone {
        /// Git URI (https://... or git@...)
        git_uri: String,
        /// Optional directory name override
        name: Option<String>,
    },
}

fn main() -> Result<()> {
    // Use try_parse so we can route help/version to stderr, keeping stdout clean
    // for shell-evaluable output.
    let cli = match Cli::try_parse() {
        Ok(c) => c,
        Err(e) => {
            match e.kind() {
                ErrorKind::DisplayHelp | ErrorKind::DisplayVersion => {
                    // Force printing to stderr so shell wrapper doesn't eval help text.
                    eprintln!("{}", e);
                    std::process::exit(0);
                }
                _ => {
                    // Let clap print the error to stderr and set appropriate exit code.
                    e.exit();
                }
            }
        }
    };

    let base_path = cli
        .path
        .clone()
        .unwrap_or_else(selector::TrySelector::default_base_path);

    match cli.command {
        None => {
            // Default to interactive selector, equivalent to `try cd` with empty query
            cli::run_cd_flow(String::new(), &base_path)
        }
        Some(Commands::Init { path, abs_path }) => {
            let script_path = env::current_exe()
                .ok()
                .and_then(|p| p.canonicalize().ok())
                .unwrap_or_else(|| PathBuf::from("try"));
            let mut tries_path = path
                .or(abs_path.filter(|p| p.is_absolute()))
                .unwrap_or(base_path.clone());
            // Normalize ~ if passed via clap as a plain string previously; keep as PathBuf otherwise
            if let Some(s) = tries_path.to_str()
                && s.starts_with("~/")
            {
                tries_path = util::shellexpand_home(s);
            }
            let path_arg = format!(r#" --path "{}""#, tries_path.display());
            if util::is_fish_shell() {
                println!(
                    r#"function try
  set -l script_path "{}"
  set -l cmd (/usr/bin/env "{}" cd{} $argv 2>/dev/tty | string collect)
  test $status -eq 0 && eval $cmd || echo $cmd
end"#,
                    script_path.display(),
                    script_path.display(),
                    path_arg
                );
            } else {
                println!(
                    r#"try() {{
  script_path='{}';
  case "$1" in
    -h|--help|-V|--version)
      /usr/bin/env "{}" "$@" 2>/dev/tty
      return;;
    cd|init|clone)
      case "$2" in
        -h|--help|-V|--version)
          /usr/bin/env "{}" "$@" 2>/dev/tty
          return;;
      esac;;
  esac
  cmd=$(/usr/bin/env "{}" cd{} "$@" 2>/dev/tty);
  [ $? -eq 0 ] && eval "$cmd" || echo "$cmd";
}}"#,
                    script_path.display(),
                    script_path.display(),
                    script_path.display(),
                    script_path.display(),
                    path_arg
                );
            }
            Ok(())
        }
        Some(Commands::Cd { query }) => {
            let query_os: Vec<OsString> = query.into_iter().map(OsString::from).collect();
            let query_str = cli::build_cd_query(&query_os);
            cli::run_cd_flow(query_str, &base_path)
        }
        Some(Commands::Clone { git_uri, name }) => {
            let dir_name = util::generate_clone_directory_name(&git_uri, name.as_deref());
            if dir_name.is_none() {
                let mut err = io::stderr();
                let _ =
                    crate::tui::error(&mut err, &format!("Unable to parse git URI: {}", git_uri));
                std::process::exit(1);
            }
            let full = base_path.join(dir_name.unwrap());
            let mut parts: Vec<String> = Vec::new();
            parts.push(util::dir_assign_for_shell(&full));
            parts.push("mkdir -p \"$dir\"".into());
            parts.push(format!("git clone '{}' \"$dir\"", git_uri));
            parts.push("touch \"$dir\"".into());
            parts.push("cd \"$dir\"".into());
            println!("{}", util::join_shell(&parts));
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;
    use std::fs;
    use std::io;
    // use std::io::Write; // removed; no tests require direct Write now
    use std::path::PathBuf;
    use std::time::{Duration, SystemTime};

    #[test]
    fn test_sanitize_query_filters_disallowed() {
        let input = "Hello,_World-!@$ 42.";
        let out = crate::util::sanitize_query(input);
        assert_eq!(out, "Hello_World- 42.");
    }

    #[test]
    fn test_is_printable() {
        assert!(crate::util::is_printable('a'));
        assert!(crate::util::is_printable('-'));
        assert!(crate::util::is_printable('_'));
        assert!(crate::util::is_printable('.'));
        assert!(crate::util::is_printable(' '));
        assert!(!crate::util::is_printable('\n'));
        assert!(!crate::util::is_printable('!'));
    }

    #[test]
    fn test_split_date_prefixed() {
        let s = "2025-08-26-hello";
        let p = crate::util::split_date_prefixed(s);
        assert_eq!(p, Some(("2025-08-26", "hello")));
        assert_eq!(crate::util::split_date_prefixed("foo"), None);
        assert_eq!(
            crate::util::split_date_prefixed("2025-08-2x-hello"),
            Some(("2025-08-2x", "hello"))
        );
    }

    #[test]
    fn test_display_width_ascii() {
        assert_eq!(crate::tui::display_width("hello"), 5);
        assert_eq!(crate::tui::display_width(""), 0);
    }

    #[test]
    fn test_civil_from_days_epoch() {
        // 1970-01-01 is day 0 in our usage
        let (y, m, d) = crate::util::civil_from_days(0);
        assert_eq!((y, m, d), (1970, 1, 1));
    }

    #[test]
    fn test_calculate_score_basic() {
        // Empty query -> date-prefixed gets a positive boost; non-date stays 0 without recency
        let s1 = crate::score::calculate_score("2025-08-26-test", "", None, None);
        let s2 = crate::score::calculate_score("foo", "", None, None);
        assert!(s1 > s2);
        assert_eq!(s2, 0.0);

        // Non-matching query => 0
        assert_eq!(crate::score::calculate_score("abc", "zz", None, None), 0.0);

        // Simple positive fuzzy match
        assert!(crate::score::calculate_score("foo-test", "ft", None, None) > 0.0);
    }

    #[test]
    fn test_format_relative_time_none() {
        assert_eq!(crate::tui::format_relative_time(None), "?");
    }

    #[test]
    fn test_extract_option_with_value_removes_and_last_wins() {
        // Case: flag separate arg form
        let mut args = vec![
            OsString::from("--path"),
            OsString::from("/tmp/x"),
            OsString::from("cd"),
        ];
        let val = crate::util::extract_option_with_value(&mut args, "--path");
        assert_eq!(val.as_deref(), Some("/tmp/x"));
        assert_eq!(args, vec![OsString::from("cd")]);

        // Case: mix of = and separate; rightmost wins
        let mut args = vec![
            OsString::from("--path=/a"),
            OsString::from("--path"),
            OsString::from("/b"),
            OsString::from("init"),
        ];
        let val = crate::util::extract_option_with_value(&mut args, "--path");
        assert_eq!(val.as_deref(), Some("/b"));
        assert_eq!(args, vec![OsString::from("init")]);
    }

    #[test]
    fn test_shell_escape_single_quotes() {
        let path = PathBuf::from("/tmp/it's ok");
        let escaped = crate::util::shell_escape(path);
        assert_eq!(escaped, "'/tmp/it'\\''s ok'");
    }

    #[test]
    fn test_normalize_query_for_match_spaces_to_dash_and_sanitize() {
        let q = "Hello,  World!!";
        let norm = crate::storage::normalize_query_for_match(q);
        assert_eq!(norm, "Hello-World");
    }

    #[test]
    fn test_fast_create_skips_when_exact_exists_ignoring_date() -> io::Result<()> {
        let tmp_root = std::env::temp_dir().join(format!("tryrs-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp_root);
        fs::create_dir_all(&tmp_root)?;
        // existing dir: date-prefixed version of "foo-bar"
        let existing = tmp_root.join("2025-08-26-foo-bar");
        fs::create_dir_all(&existing)?;
        // exact match by stripping date and normalizing spaces -> '-'
        let res = crate::storage::fast_create_target_if_no_exact(&tmp_root, "foo bar")?;
        assert!(res.is_none());
        let _ = fs::remove_dir_all(&tmp_root);
        Ok(())
    }

    #[test]
    fn test_fast_create_returns_target_when_no_match() -> io::Result<()> {
        let tmp_root = std::env::temp_dir().join(format!("tryrs-test2-{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp_root);
        fs::create_dir_all(&tmp_root)?;
        let q = "new thing";
        let res = crate::storage::fast_create_target_if_no_exact(&tmp_root, q)?;
        assert!(res.is_some());
        let p = res.unwrap();
        let bn = p.file_name().unwrap().to_string_lossy().to_string();
        assert!(bn.starts_with(&crate::util::today_prefix()));
        assert!(bn.ends_with("-new-thing"));
        let _ = fs::remove_dir_all(&tmp_root);
        Ok(())
    }

    #[test]
    fn test_fast_create_sanitizes_query() -> io::Result<()> {
        let tmp_root = std::env::temp_dir().join(format!("tryrs-test3-{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp_root);
        fs::create_dir_all(&tmp_root)?;
        let q = "Hello,_World-!@$ 42."; // sanitize removes !@$ but keeps underscore, dash, space, dot
        let res = crate::storage::fast_create_target_if_no_exact(&tmp_root, q)?;
        let p = res.unwrap();
        let bn = p.file_name().unwrap().to_string_lossy().to_string();
        assert!(bn.contains("Hello_World-"));
        assert!(bn.ends_with("-42."));
        let _ = fs::remove_dir_all(&tmp_root);
        Ok(())
    }

    #[test]
    fn test_compute_viewport_no_scroll_needed() {
        // total 5 items, max_visible 3, starting at top
        let (s, e) = crate::tui::compute_viewport(0, 0, 3, 5);
        assert_eq!((s, e), (0, 3));
        let (s, e) = crate::tui::compute_viewport(1, 0, 3, 5);
        assert_eq!((s, e), (0, 3));
        let (s, e) = crate::tui::compute_viewport(2, 0, 3, 5);
        assert_eq!((s, e), (0, 3));
    }

    #[test]
    fn test_compute_viewport_scrolls_down() {
        // When cursor moves past the last visible index, scroll advances
        let (s, e) = crate::tui::compute_viewport(3, 0, 3, 6);
        assert_eq!((s, e), (1, 4));
        let (s, e) = crate::tui::compute_viewport(4, 1, 3, 6);
        assert_eq!((s, e), (2, 5));
    }

    #[test]
    fn test_compute_viewport_scrolls_up() {
        // If cursor moves above current scroll, scroll follows cursor
        let (s, e) = crate::tui::compute_viewport(1, 2, 3, 6);
        assert_eq!((s, e), (1, 4));
        let (s, e) = crate::tui::compute_viewport(0, 1, 3, 6);
        assert_eq!((s, e), (0, 3));
    }

    #[test]
    fn test_compute_viewport_clamps_end_to_total() {
        // Near the end, end should not exceed total
        let (s, e) = crate::tui::compute_viewport(5, 3, 3, 6);
        assert_eq!((s, e), (3, 6));
    }

    #[test]
    fn test_build_cd_query_strips_duplicate_cd() {
        let args = vec![
            OsString::from("cd"),
            OsString::from("foo"),
            OsString::from("bar"),
        ];
        let q = crate::cli::build_cd_query(&args);
        assert_eq!(q, "foo bar");
    }

    #[test]
    fn test_build_cd_query_no_change_when_no_dup() {
        let args = vec![OsString::from("notes"), OsString::from("proj")];
        let q = crate::cli::build_cd_query(&args);
        assert_eq!(q, "notes proj");
    }

    #[test]
    fn test_git_uri_parsing_and_dirname() {
        use crate::util::{generate_clone_directory_name, parse_git_uri};
        let p1 = parse_git_uri("https://github.com/user/repo.git").unwrap();
        assert_eq!(p1.user, "user");
        assert_eq!(p1.repo, "repo");
        let dn = generate_clone_directory_name("git@github.com:user/repo.git", None).unwrap();
        assert!(dn.ends_with("-user-repo"));
        // custom name wins
        let dn2 = generate_clone_directory_name("https://gitlab.com/u/r", Some("my-fork"));
        assert_eq!(dn2.unwrap(), "my-fork");
    }

    #[test]
    fn test_score_recency_mtime_and_ctime() {
        let now = SystemTime::now();
        let older = now - Duration::from_secs(10 * 86_400); // ~10 days ago
        let recent = now - Duration::from_secs(2 * 3_600); // 2 hours ago

        // With empty query and non-date-prefixed text, score is only recency-based
        let s_old_m = crate::score::calculate_score("hello", "", None, Some(older));
        let s_new_m = crate::score::calculate_score("hello", "", None, Some(recent));
        assert!(s_new_m > s_old_m);
        assert!(s_new_m > 0.0);

        let s_old_c = crate::score::calculate_score("hello", "", Some(older), None);
        let s_new_c = crate::score::calculate_score("hello", "", Some(recent), None);
        assert!(s_new_c > s_old_c);
        assert!(s_new_c > 0.0);
    }

    #[test]
    fn test_format_relative_time_buckets() {
        let now = SystemTime::now();
        // just now (<= 9s)
        assert_eq!(crate::tui::format_relative_time(Some(now)), "just now");
        // minutes
        assert_eq!(
            crate::tui::format_relative_time(Some(now - Duration::from_secs(2 * 60))),
            "2m ago"
        );
        // hours
        assert_eq!(
            crate::tui::format_relative_time(Some(now - Duration::from_secs(3 * 3_600))),
            "3h ago"
        );
        // days
        assert_eq!(
            crate::tui::format_relative_time(Some(now - Duration::from_secs(5 * 86_400))),
            "5d ago"
        );
        // months (30-day blocks)
        assert_eq!(
            crate::tui::format_relative_time(Some(now - Duration::from_secs(2 * 2_592_000))),
            "2mo ago"
        );
        // years
        assert_eq!(
            crate::tui::format_relative_time(Some(now - Duration::from_secs(3 * 31_536_000))),
            "3y ago"
        );
        // future timestamp gracefully treated as "just now"
        assert_eq!(
            crate::tui::format_relative_time(Some(now + Duration::from_secs(60))),
            "just now"
        );
    }

    #[test]
    fn test_display_width_wide_chars() {
        // Each CJK char typically counts as width 2
        assert_eq!(crate::tui::display_width("你好"), 4);
        assert_eq!(crate::tui::display_width("ab"), 2);
    }

    #[test]
    fn test_is_git_uri_matrix() {
        use crate::util::is_git_uri;
        assert!(is_git_uri("https://github.com/user/repo"));
        assert!(is_git_uri("git@github.com:user/repo.git"));
        assert!(is_git_uri("https://gitlab.com/u/r"));
        // permissive detection: contains host or .git suffix
        assert!(is_git_uri("ssh://git@github.com/user/repo.git"));
        assert!(!is_git_uri("notes/proj"));
        assert!(!is_git_uri("foo"));
        assert!(is_git_uri("bar.git"));
    }

    #[test]
    fn test_shellexpand_home_and_join_shell() {
        use crate::util::shellexpand_home;
        // shellexpand_home expands ~/
        if let Some(home) = dirs::home_dir() {
            let p = shellexpand_home("~/abc");
            assert!(p.starts_with(&home));
            assert!(p.ends_with("abc"));
        }
        // Non-tilde path unchanged
        assert_eq!(shellexpand_home("/tmp/x").to_string_lossy(), "/tmp/x");
        // join_shell trivial
        assert_eq!(crate::util::join_shell(&["a".into(), "b".into()]), "a && b");
    }
}
