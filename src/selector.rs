use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers, read},
    terminal,
    tty::IsTty,
};

use std::{
    env, fs,
    io::{self, Write},
    path::{Path, PathBuf},
    time::Duration,
};

use crate::error::Result;
use crate::model::TryDir;
use crate::score::calculate_score;
use crate::tui::{self, TermGuard, render};
use crate::util::{is_printable, sanitize_query, shellexpand_home};

// Terminal defaults and UI timing
const DEFAULT_TERM_WIDTH: u16 = 80;
const DEFAULT_TERM_HEIGHT: u16 = 24;
const POLL_INTERVAL_MS: u64 = 200;
// Number of extra rows (e.g., "Create new") accounted for in list sizing
const EXTRA_LIST_ROWS: usize = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ActionType {
    Cd,
    Mkdir,
    Cancel,
}

#[derive(Clone, Debug)]
pub(crate) struct Selection {
    pub(crate) kind: ActionType,
    pub(crate) path: Option<PathBuf>,
}

pub(crate) struct TrySelector {
    term_w: u16,
    term_h: u16,
    cursor: usize,
    pub(crate) scroll: usize,
    pub(crate) input_buf: String,
    pub(crate) all_tries: Option<Vec<TryDir>>,
    pub(crate) base_path: PathBuf,
    pub(crate) selected: Option<Selection>,
    status_msg: Option<String>,
    // no vim/undo mode in Ruby semantics
}

impl TrySelector {
    /// Resolves the default base path for tries, honoring the `TRY_PATH` env var.
    pub(crate) fn default_base_path() -> PathBuf {
        if let Ok(p) = env::var("TRY_PATH") {
            return shellexpand_home(&p);
        }
        shellexpand_home("~/src/tries")
    }

    pub(crate) fn new(initial_query: &str, base_path: PathBuf) -> Result<Self> {
        if !base_path.exists() {
            fs::create_dir_all(&base_path)?;
        }
        let (w, h) = terminal::size().unwrap_or((DEFAULT_TERM_WIDTH, DEFAULT_TERM_HEIGHT));
        Ok(Self {
            term_w: w,
            term_h: h,
            cursor: 0,
            scroll: 0,
            input_buf: sanitize_query(initial_query),
            all_tries: None,
            base_path,
            selected: None,
            status_msg: None,
        })
    }

    pub(crate) fn run(&mut self) -> Result<Option<Selection>> {
        let mut err = io::stderr();
        if !io::stdin().is_tty() || !io::stderr().is_tty() {
            crate::tui::error(&mut err, "try requires an interactive terminal")?;
            return Ok(None);
        }

        let _guard = TermGuard::new()?; // raw mode on; auto-restores on drop
        self.setup_terminal(&mut err)?; // initial clear + move

        // Lazy redraw to reduce flicker
        let mut dirty = true;
        let mut tries: Vec<TryDir> = Vec::new();
        let (mut last_w, mut last_h) =
            terminal::size().unwrap_or((DEFAULT_TERM_WIDTH, DEFAULT_TERM_HEIGHT));
        self.term_w = last_w;
        self.term_h = last_h;

        loop {
            let (w, h) = terminal::size().unwrap_or((DEFAULT_TERM_WIDTH, DEFAULT_TERM_HEIGHT));
            if w != last_w || h != last_h {
                self.term_w = w;
                self.term_h = h;
                last_w = w;
                last_h = h;
                dirty = true;
            }

            if dirty {
                tries = self.get_tries();
                let total_items = tries.len() + EXTRA_LIST_ROWS;
                self.cursor = self.cursor.min(total_items.saturating_sub(1));
                let ctx = tui::RenderCtx {
                    term_w: self.term_w,
                    term_h: self.term_h,
                    cursor: self.cursor,
                    scroll: self.scroll,
                    input_buf: &self.input_buf,
                    tries: &tries,
                    status_msg: self.status_msg.clone(),
                    show_delete_pending: false,
                };
                render(&mut err, &ctx)?;
                dirty = false;
            }

            if !event::poll(Duration::from_millis(POLL_INTERVAL_MS))? {
                continue;
            }

            match read()? {
                Event::Key(KeyEvent {
                    code, modifiers, ..
                }) => match (code, modifiers) {
                    (KeyCode::Esc, _) => {
                        self.selected = Some(Selection {
                            kind: ActionType::Cancel,
                            path: None,
                        });
                        break;
                    }
                    (KeyCode::Up, _) | (KeyCode::Char('p'), KeyModifiers::CONTROL) => {
                        if self.cursor > 0 {
                            self.cursor -= 1;
                            dirty = true;
                        }
                    }
                    (KeyCode::Down, _) | (KeyCode::Char('n'), KeyModifiers::CONTROL) => {
                        let total_items = tries.len() + EXTRA_LIST_ROWS;
                        if self.cursor + 1 < total_items {
                            self.cursor += 1;
                            dirty = true;
                        }
                    }
                    (KeyCode::Left, _) | (KeyCode::Right, _) => {}
                    (KeyCode::Enter, _) => {
                        if self.cursor < tries.len() {
                            self.handle_select_existing(&tries[self.cursor]);
                            break;
                        } else if !self.input_buf.is_empty() {
                            let date_prefix = crate::util::today_prefix();
                            let final_name = format!("{}-{}", date_prefix, self.input_buf)
                                .replace(char::is_whitespace, "-");
                            let full_path = self.base_path.join(final_name);
                            self.selected = Some(Selection {
                                kind: ActionType::Mkdir,
                                path: Some(full_path),
                            });
                            break;
                        } else {
                            self.prompt_new_name(&mut err)?;
                            if self.selected.is_some() {
                                break;
                            }
                            dirty = true;
                        }
                    }
                    (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                        self.selected = Some(Selection {
                            kind: ActionType::Cancel,
                            path: None,
                        });
                        break;
                    }
                    (KeyCode::Backspace, _) => {
                        self.input_buf.pop();
                        self.cursor = 0;
                        dirty = true;
                    }
                    (KeyCode::Char('d'), KeyModifiers::CONTROL) => {
                        if self.cursor < tries.len() {
                            let t = &tries[self.cursor];
                            if self.confirm_and_delete(&mut err, t)? {
                                self.all_tries = None;
                                self.status_msg = Some(format!("Deleted: {}", t.basename));
                                dirty = true;
                            } else {
                                self.status_msg = Some("Delete cancelled".into());
                                dirty = true;
                            }
                        }
                    }
                    (KeyCode::Char(ch), mods) => {
                        if mods.is_empty() && is_printable(ch) {
                            self.input_buf.push(ch);
                            self.cursor = 0;
                            dirty = true;
                        }
                    }
                    _ => {}
                },
                Event::Resize(w, h) => {
                    self.term_w = w;
                    self.term_h = h;
                    dirty = true;
                }
                _ => {}
            }
        }
        Ok(self.selected.clone())
    }

    /// Clears the terminal and moves the cursor to the top-left origin.
    fn setup_terminal(&self, err: &mut io::Stderr) -> Result<()> {
        crossterm::execute!(
            err,
            crossterm::terminal::Clear(crossterm::terminal::ClearType::All),
            crossterm::cursor::MoveTo(0, 0)
        )?;
        Ok(())
    }

    fn load_all(&mut self) {
        if self.all_tries.is_some() {
            return;
        }
        let mut out = Vec::new();
        if let Ok(entries) = fs::read_dir(&self.base_path) {
            for e in entries.flatten() {
                let path = e.path();
                let Ok(meta) = e.metadata() else { continue };
                if !meta.is_dir() {
                    continue;
                }
                let basename = e.file_name().to_string_lossy().to_string();
                if basename == ".try_trash" {
                    continue;
                }
                let ctime = meta.created().ok();
                let mtime = meta.modified().ok();
                out.push(TryDir {
                    basename,
                    path,
                    ctime,
                    mtime,
                    score: 0.0,
                });
            }
        }
        self.all_tries = Some(out);
    }

    fn get_tries(&mut self) -> Vec<TryDir> {
        self.load_all();
        let mut tries = self.all_tries.clone().unwrap_or_default();
        for t in &mut tries {
            t.score = calculate_score(&t.basename, &self.input_buf, t.ctime, t.mtime);
        }
        if self.input_buf.is_empty() {
            tries.sort_by(|a, b| b.score.total_cmp(&a.score));
            tries
        } else {
            let mut filtered: Vec<_> = tries.into_iter().filter(|t| t.score > 0.0).collect();
            filtered.sort_by(|a, b| b.score.total_cmp(&a.score));
            filtered
        }
    }

    fn handle_select_existing(&mut self, t: &TryDir) {
        self.selected = Some(Selection {
            kind: ActionType::Cd,
            path: Some(t.path.clone()),
        });
    }

    fn prompt_new_name(&mut self, err: &mut io::Stderr) -> Result<()> {
        // flip to cooked for line input
        crossterm::terminal::disable_raw_mode()?;
        crossterm::execute!(err, crossterm::cursor::Show)?;
        crossterm::execute!(
            err,
            crossterm::terminal::Clear(crossterm::terminal::ClearType::All),
            crossterm::cursor::MoveTo(0, 0)
        )?;
        tui::styled(
            err,
            crossterm::style::Attribute::Bold,
            Some(crossterm::style::Color::Cyan),
            "Enter new try name",
        )?;
        writeln!(err)?;
        let prefix = crate::util::today_prefix();
        write!(err, "> ")?;
        tui::dim(err, &format!("{}-", prefix))?;
        err.flush()?;

        let mut line = String::new();
        io::stdin().read_line(&mut line)?;
        let line = line.trim();

        crossterm::terminal::enable_raw_mode()?;
        crossterm::execute!(err, crossterm::cursor::Hide)?;

        if line.is_empty() {
            return Ok(());
        }
        let final_name = format!("{}-{}", prefix, line).replace(char::is_whitespace, "-");
        let full = self.base_path.join(final_name);
        self.selected = Some(Selection {
            kind: ActionType::Mkdir,
            path: Some(full),
        });
        Ok(())
    }

    fn confirm_and_delete(&mut self, err: &mut io::Stderr, t: &TryDir) -> Result<bool> {
        // Compute size and file count recursively
        let (mut files, mut bytes) = (0u64, 0u64);
        fn walk(p: &Path, files: &mut u64, bytes: &mut u64) {
            if let Ok(md) = std::fs::symlink_metadata(p) {
                if md.is_file() {
                    *files += 1;
                    *bytes += md.len();
                } else if md.is_dir()
                    && let Ok(rd) = std::fs::read_dir(p)
                {
                    for e in rd.flatten() {
                        walk(&e.path(), files, bytes);
                    }
                }
            }
        }
        walk(&t.path, &mut files, &mut bytes);

        // Switch to cooked mode for line input
        crossterm::terminal::disable_raw_mode()?;
        crossterm::execute!(err, crossterm::cursor::Show)?;
        crossterm::execute!(
            err,
            crossterm::terminal::Clear(crossterm::terminal::ClearType::All),
            crossterm::cursor::MoveTo(0, 0)
        )?;
        tui::styled(
            err,
            crossterm::style::Attribute::Bold,
            Some(crossterm::style::Color::Cyan),
            "Delete Directory",
        )?;
        writeln!(err)?;
        writeln!(err)?;
        write!(
            err,
            "Are you sure you want to delete: {}\r\n  in {}\r\n  files: {} files\r\n  size: {}\r\n\r\n",
            t.basename,
            t.path.display(),
            files,
            format_human_size(bytes)
        )?;
        tui::styled(
            err,
            crossterm::style::Attribute::Bold,
            Some(crossterm::style::Color::Yellow),
            "Type ",
        )?;
        tui::styled(
            err,
            crossterm::style::Attribute::Reset,
            Some(crossterm::style::Color::Reset),
            "YES",
        )?;
        tui::styled(
            err,
            crossterm::style::Attribute::Bold,
            Some(crossterm::style::Color::Yellow),
            " to confirm: ",
        )?;
        err.flush()?;

        let mut line = String::new();
        std::io::stdin().read_line(&mut line)?;

        crossterm::terminal::enable_raw_mode()?;
        crossterm::execute!(err, crossterm::cursor::Hide)?;

        if line.trim() == "YES" {
            // Hard delete
            let _ = std::fs::remove_dir_all(&t.path);
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

fn format_human_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "K", "M", "G", "T"];
    const BYTES_PER_KIB: f64 = 1_024.0;
    let mut val = bytes as f64;
    let mut idx = 0;
    while val >= BYTES_PER_KIB && idx + 1 < UNITS.len() {
        val /= BYTES_PER_KIB;
        idx += 1;
    }
    if idx == 0 {
        format!("{}B", bytes)
    } else {
        format!("{:.1}{}", val, UNITS[idx])
    }
}
