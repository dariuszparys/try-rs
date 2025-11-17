use crossterm::{
    cursor, execute,
    style::{Attribute, Color, SetAttribute, SetForegroundColor},
    terminal::{
        self, ClearType, EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode,
        enable_raw_mode,
    },
    tty::IsTty,
};
use unicode_width::UnicodeWidthStr;

use std::cmp::min;
use std::io::{self, Write};

use crate::error::Result;

use crate::model::TryDir;

pub struct TermGuard;

impl TermGuard {
    /// Enables raw mode and hides the cursor; restored automatically on drop via `Drop`.
    pub fn new() -> Result<Self> {
        enable_raw_mode()?;
        let mut err = io::stderr();
        let _ = execute!(err, EnterAlternateScreen, cursor::Hide);
        Ok(Self)
    }
}
impl Drop for TermGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let mut err = io::stderr();
        // Leave alt screen, clear, and restore cursor visibility
        let _ = execute!(
            err,
            terminal::Clear(ClearType::All),
            cursor::MoveTo(0, 0),
            cursor::Show,
            LeaveAlternateScreen
        );
    }
}

/// Display width accounting for Unicode (wide) characters.
pub(crate) fn display_width(s: &str) -> usize {
    UnicodeWidthStr::width(s)
}

/// Formats an optional timestamp as a concise relative string like `3h ago`.
pub(crate) fn format_relative_time(t: Option<std::time::SystemTime>) -> String {
    const JUST_NOW_MAX: u64 = 9; // seconds
    const MINUTE: u64 = 60;
    const HOUR: u64 = 3_600;
    const DAY: u64 = 86_400;
    const MONTH: u64 = 2_592_000; // 30 days
    const YEAR: u64 = 31_536_000; // 365 days
    let Some(time) = t else {
        return "?".into();
    };
    let now = std::time::SystemTime::now();
    let Ok(diff) = now.duration_since(time) else {
        return "just now".into();
    };
    let secs = diff.as_secs();

    if secs <= JUST_NOW_MAX {
        "just now".into()
    } else if secs < HOUR {
        format!("{}m ago", secs / MINUTE)
    } else if secs < DAY {
        format!("{}h ago", secs / HOUR)
    } else if secs < MONTH {
        format!("{}d ago", secs / DAY)
    } else if secs < YEAR {
        format!("{}mo ago", secs / MONTH)
    } else {
        format!("{}y ago", secs / YEAR)
    }
}

/// Writes `s` with the given attribute and optional foreground color; resets color afterward.
fn colors_enabled_stderr(err: &io::Stderr) -> bool {
    if !err.is_tty() {
        return false;
    }
    if std::env::var_os("NO_COLOR").is_some() {
        return false;
    }
    if let Ok(v) = std::env::var("CLICOLOR_FORCE")
        && v != "0"
    {
        return true;
    }
    if let Ok(v) = std::env::var("CLICOLOR")
        && v == "0"
    {
        return false;
    }
    true
}

pub(crate) fn styled(
    err: &mut io::Stderr,
    attr: Attribute,
    fg: Option<Color>,
    s: &str,
) -> Result<()> {
    if colors_enabled_stderr(err) {
        if let Some(c) = fg {
            execute!(err, SetForegroundColor(c))?;
        }
        execute!(err, SetAttribute(attr))?;
        write!(err, "{s}")?;
        execute!(err, SetForegroundColor(Color::Reset))?;
    } else {
        write!(err, "{s}")?;
    }
    Ok(())
}
/// Dimmed grey helper wrapper around `styled`.
pub(crate) fn dim(err: &mut io::Stderr, s: &str) -> Result<()> {
    styled(err, Attribute::Dim, Some(Color::Grey), s)
}
/// Bold yellow helper wrapper around `styled`.
pub(crate) fn highlight(err: &mut io::Stderr, s: &str) -> Result<()> {
    styled(err, Attribute::Bold, Some(Color::Yellow), s)
}

/// Styled warning line: prints "Warning: " in bold yellow, then the message, and a newline.
pub(crate) fn warn(err: &mut io::Stderr, msg: &str) -> Result<()> {
    highlight(err, "Warning: ")?;
    execute!(err, SetAttribute(Attribute::Reset))?;
    writeln!(err, "{msg}")?;
    Ok(())
}

/// Styled error line: prints "Error: " in bold red, then the message, and a newline.
pub(crate) fn error(err: &mut io::Stderr, msg: &str) -> Result<()> {
    styled(err, Attribute::Bold, Some(Color::Red), "Error: ")?;
    execute!(err, SetAttribute(Attribute::Reset))?;
    writeln!(err, "{msg}")?;
    Ok(())
}

/// Writes text highlighting the next matching characters from `query` in bold, case-insensitively.
pub(crate) fn write_highlighted(
    err: &mut io::Stderr,
    text: &str,
    query: &str,
    is_sel: bool,
) -> Result<()> {
    if query.is_empty() {
        write!(err, "{text}")?;
        return Ok(());
    }
    let tl_chars: Vec<char> = text.to_lowercase().chars().collect();
    let text_chars: Vec<char> = text.chars().collect();
    let q_chars: Vec<char> = query.to_lowercase().chars().collect();
    let mut qi = 0usize;

    for i in 0..text_chars.len() {
        let ch = text_chars[i];
        if qi < q_chars.len() && tl_chars[i] == q_chars[qi] {
            // Don't reset all attributes, just bold and color
            highlight(err, &ch.to_string())?;
            if is_sel {
                // If selected, we need to re-apply the reverse attribute
                execute!(err, SetAttribute(Attribute::Reverse))?;
            } else {
                execute!(err, SetAttribute(Attribute::Reset))?;
            }
            qi += 1;
        } else {
            write!(err, "{ch}")?;
        }
    }
    Ok(())
}

/// Computes the viewport scroll and end index for a list UI given the current cursor position.
/// Returns (scroll, end), where end is exclusive and clamped to total.
pub(crate) fn compute_viewport(
    cursor: usize,
    scroll: usize,
    max_visible: usize,
    total: usize,
) -> (usize, usize) {
    let mut s = scroll;
    if cursor < s {
        s = cursor;
    } else if cursor >= s.saturating_add(max_visible) {
        s = cursor + 1 - max_visible;
    }
    let end = min(s + max_visible, total);
    (s, end)
}

pub struct RenderCtx<'a> {
    pub term_w: u16,
    pub term_h: u16,
    pub cursor: usize,
    pub scroll: usize,
    pub input_buf: &'a str,
    pub tries: &'a [TryDir],
    pub status_msg: Option<String>,
    pub show_delete_pending: bool,
}

/// Renders the interactive UI for the list of tries and the input query.
pub(crate) fn render(err: &mut io::Stderr, ctx: &RenderCtx<'_>) -> Result<()> {
    execute!(err, terminal::Clear(ClearType::All), cursor::MoveTo(0, 0))?;
    const MIN_SEPARATOR_WIDTH: usize = 1;
    const RESERVED_LINES: u16 = 8; // header, spacing, footer, etc.
    const MIN_VISIBLE_ITEMS: usize = 3;
    let sep_w = ctx.term_w.saturating_sub(1) as usize;
    let separator = "─".repeat(std::cmp::max(sep_w, MIN_SEPARATOR_WIDTH));

    highlight(err, "📁 Try Directory Selection")?;
    execute!(err, SetAttribute(Attribute::Reset))?;
    write!(err, "\r\n")?;
    dim(err, &separator)?;
    execute!(err, SetAttribute(Attribute::Reset))?;
    write!(err, "\r\n")?;

    write!(err, "Search: {}", ctx.input_buf)?;
    write!(err, "\r\n\r\n")?;

    let max_visible = usize::max(
        ctx.term_h.saturating_sub(RESERVED_LINES) as usize,
        MIN_VISIBLE_ITEMS,
    );
    const EXTRA_LIST_ROWS: usize = 1; // "Create new" row
    let total = ctx.tries.len() + EXTRA_LIST_ROWS;

    let (_, end) = compute_viewport(ctx.cursor, ctx.scroll, max_visible, total);

    for idx in ctx.scroll..end {
        if idx == ctx.tries.len() && !ctx.tries.is_empty() {
            write!(err, "\r\n")?;
        }

        let is_sel = idx == ctx.cursor;
        if idx < ctx.tries.len() {
            let t = &ctx.tries[idx];
            // Compose and print prefix (arrow + icon), measure width accurately
            let prefix = if is_sel { "→ " } else { "  " };
            write!(err, "{prefix}")?;
            write!(err, "📁 ")?;
            let prefix_w = display_width(&format!("{}{}", prefix, "📁 "));

            // Selected row: enter reverse for the name portion only
            if is_sel {
                execute!(err, SetAttribute(Attribute::Reverse))?;
            }
            write_highlighted(err, &t.basename, ctx.input_buf, is_sel)?;

            // Right-side meta: size and mtime
            let size_text = t
                .size
                .map(crate::util::format_human_size)
                .unwrap_or_else(|| "...".to_string());
            let time_text = format_relative_time(t.mtime);
            let meta = format!("{size_text}, {time_text}");

            // Compute remaining columns; ensure we never overflow terminal width
            let name_w = display_width(&t.basename);
            let left_w = prefix_w + name_w;
            if (left_w as u16) < ctx.term_w {
                let rem = ctx.term_w as usize - left_w;
                let meta_w = meta.len(); // ASCII-only, width == len
                execute!(err, SetAttribute(Attribute::Reset))?; // meta not reversed
                if rem == 0 {
                    // Nothing fits
                } else if meta_w >= rem {
                    // Print a space then a truncated meta to avoid wrap
                    write!(err, " ")?;
                    let keep = rem.saturating_sub(1);
                    let truncated: String = meta.chars().take(keep).collect();
                    dim(err, &truncated)?;
                } else {
                    // Right align meta within the remaining width
                    let meta_fmt = format!("{meta:>rem$}");
                    dim(err, &meta_fmt)?;
                }
            }
            execute!(err, SetAttribute(Attribute::Reset))?;
        } else {
            // New entry row
            if is_sel {
                highlight(err, "→ ")?;
                execute!(err, SetAttribute(Attribute::Reset))?;
            } else {
                write!(err, "  ")?;
            }
            write!(err, "+ ")?;
            if is_sel {
                execute!(err, SetAttribute(Attribute::Reverse))?;
            }
            if ctx.input_buf.is_empty() {
                write!(err, "Create new")?;
            } else {
                write!(err, "Create new: {}", ctx.input_buf)?;
            }
            execute!(err, SetAttribute(Attribute::Reset))?;
        }
        write!(err, "\r\n")?;
    }

    // Separator below list and new-entry row
    dim(err, &separator)?;
    execute!(err, SetAttribute(Attribute::Reset))?;
    write!(err, "\r\n")?;

    // Instructions
    dim(
        err,
        "↑↓: Navigate  Enter: Select  Ctrl-D: Delete  ESC: Cancel",
    )?;
    execute!(err, SetAttribute(Attribute::Reset))?;
    write!(err, "\r\n")?;

    // Status/prompt line
    if ctx.show_delete_pending {
        dim(err, "delete pending: press d to confirm; Esc to cancel")?;
        execute!(err, SetAttribute(Attribute::Reset))?;
    } else if let Some(s) = &ctx.status_msg {
        dim(err, s)?;
        execute!(err, SetAttribute(Attribute::Reset))?;
    }

    err.flush()?;
    Ok(())
}
