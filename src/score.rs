use std::time::SystemTime;

use crate::util::split_date_prefixed;

/// Computes a fuzzy match score for `text` against `query`, with recency boosts from ctime/mtime.
pub(crate) fn calculate_score(
    text: &str,
    query: &str,
    ctime: Option<SystemTime>,
    mtime: Option<SystemTime>,
) -> f64 {
    // Tunable weights; kept identical to previous literals.
    const DATE_PREFIX_BONUS: f64 = 2.0;
    const LENGTH_SMOOTHING: f64 = 10.0;
    const CTIME_WEIGHT: f64 = 2.0;
    const MTIME_WEIGHT: f64 = 3.0;
    // Time constants for recency boosts
    const SECONDS_PER_DAY: f64 = 86_400.0;
    const SECONDS_PER_HOUR: f64 = 3_600.0;

    let mut score = 0.0;
    if split_date_prefixed(text).is_some() {
        score += DATE_PREFIX_BONUS;
    }

    if !query.is_empty() {
        let tl = text.to_lowercase();
        let ql = query.to_lowercase();

        let q_len = ql.chars().count();
        let mut q_iter = ql.chars();
        let mut current_q = q_iter.next();
        let mut matched = 0usize;

        let mut last_pos: Option<usize> = None;
        let mut prev_ch: Option<char> = None;

        for (pos, ch) in tl.chars().enumerate() {
            if current_q.is_none() {
                break;
            }
            if Some(ch) == current_q {
                score += 1.0;
                let prev_is_boundary = prev_ch.map(|c| !c.is_alphanumeric()).unwrap_or(true);
                if prev_is_boundary {
                    score += 1.0;
                }
                if let Some(lp) = last_pos {
                    let gap = pos.saturating_sub(lp + 1) as f64;
                    score += 1.0 / (gap + 1.0).sqrt();
                }
                last_pos = Some(pos);
                matched += 1;
                current_q = q_iter.next();
            }
            prev_ch = Some(ch);
        }
        if matched < q_len {
            return 0.0;
        }
        if let Some(lp) = last_pos {
            score *= q_len as f64 / (lp as f64 + 1.0);
        }
        let text_chars_len = text.chars().count() as f64;
        score *= LENGTH_SMOOTHING / (text_chars_len + LENGTH_SMOOTHING);
    }

    let now = SystemTime::now();
    if let Some(ct) = ctime
        && let Ok(age) = now.duration_since(ct)
    {
        let days = age.as_secs_f64() / SECONDS_PER_DAY;
        score += CTIME_WEIGHT / (days + 1.0).sqrt();
    }
    if let Some(mt) = mtime
        && let Ok(age) = now.duration_since(mt)
    {
        let hours = age.as_secs_f64() / SECONDS_PER_HOUR;
        score += MTIME_WEIGHT / (hours + 1.0).sqrt();
    }
    score
}
