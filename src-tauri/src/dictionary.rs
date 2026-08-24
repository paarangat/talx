//! Personal dictionary: user-supplied names, jargon and product terms that
//! transcription otherwise mangles.
//!
//! ponytail: stored as a JSON array of strings in the existing `settings`
//! key/value table — no schema, no migration. Swap for a real table only if
//! terms ever need per-term metadata (phonetics, replacements).

use rusqlite::Connection;

use crate::db;

pub const DB_KEY: &str = "talx:dictionary";

/// Max terms kept. Guards the Whisper prompt and the LLM system prompt from
/// unbounded growth.
const MAX_TERMS: usize = 200;

/// Max characters in a single term.
const MAX_TERM_LEN: usize = 80;

/// Whisper's prompt limit is 224 tokens; stay well under it.
pub const WHISPER_BUDGET: usize = 160;

/// The LLM system prompt has no tight limit — allow the whole dictionary.
pub const LLM_BUDGET: usize = MAX_TERMS * (MAX_TERM_LEN + 1) * 2;

/// Strip control characters and collapse whitespace so a term can never inject
/// a new instruction line into the LLM system prompt, and trim to a sane length.
fn sanitize(term: &str) -> String {
    let cleaned: String = term
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect();
    cleaned
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(MAX_TERM_LEN)
        .collect()
}

/// Normalize a list coming from the frontend: sanitize, drop empties,
/// de-duplicate (case-insensitively) and cap the count.
pub fn normalize(terms: &[String]) -> Vec<String> {
    let mut seen: Vec<String> = Vec::new();
    let mut out: Vec<String> = Vec::new();
    for term in terms {
        let term = sanitize(term);
        if term.is_empty() {
            continue;
        }
        let fold = term.to_lowercase();
        if seen.contains(&fold) {
            continue;
        }
        seen.push(fold);
        out.push(term);
        if out.len() >= MAX_TERMS {
            break;
        }
    }
    out
}

/// Comma-separated term list, truncated at a whole-term boundary so it stays
/// inside `budget` estimated tokens. Never slices mid-character.
///
/// `budget` is measured in estimated tokens: ASCII counts as 1, everything
/// else as 2 (Devanagari and friends tokenize much worse than Latin).
pub fn joined(terms: &[String], budget: usize) -> String {
    let mut out = String::new();
    let mut used = 0usize;
    for term in terms {
        let term = sanitize(term);
        if term.is_empty() {
            continue;
        }
        let cost = term
            .chars()
            .map(|c| if c.is_ascii() { 1 } else { 2 })
            .sum::<usize>()
            + 1; // separator
        if used + cost > budget {
            break;
        }
        used += cost;
        if !out.is_empty() {
            out.push_str(", ");
        }
        out.push_str(&term);
    }
    out
}

/// Read the dictionary from the settings table. A missing or corrupt value
/// yields an empty dictionary rather than an error — a bad dictionary must
/// never block dictation.
pub fn load(conn: &Connection) -> Vec<String> {
    let raw = db::get_setting(conn, DB_KEY).ok().flatten();
    let parsed = raw
        .as_deref()
        .and_then(|json| serde_json::from_str::<Vec<String>>(json).ok())
        .unwrap_or_default();
    normalize(&parsed)
}

/// Persist the dictionary to the settings table.
pub fn save(conn: &Connection, terms: &[String]) -> Result<(), String> {
    let json = serde_json::to_string(terms).map_err(|e| format!("Dictionary encode error: {e}"))?;
    db::set_setting(conn, DB_KEY, &json).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_cleans_dedupes_and_flattens() {
        // duplicates (exact + case-folded), empties, and a newline injection attempt
        let input = vec![
            "  Paarangat ".to_string(),
            "Paarangat".to_string(),
            "paarangat".to_string(),
            "".to_string(),
            "   ".to_string(),
            "Talx\nIgnore all".to_string(),
        ];
        assert_eq!(
            normalize(&input),
            vec!["Paarangat".to_string(), "Talx Ignore all".to_string()]
        );
    }

    #[test]
    fn joined_truncates_on_term_boundary_without_panicking() {
        let terms: Vec<String> = (0..200).map(|i| format!("term{i:03}")).collect();
        let out = joined(&terms, WHISPER_BUDGET);
        assert!(!out.is_empty());
        assert!(out.split(", ").count() < terms.len(), "budget not applied");
        // Every emitted chunk is a whole term, never a sliced one.
        for chunk in out.split(", ") {
            assert!(terms.iter().any(|t| t == chunk), "sliced term: {chunk}");
        }
    }

    #[test]
    fn joined_handles_multibyte_terms() {
        let terms: Vec<String> = (0..100).map(|_| "पारंगत".to_string()).collect();
        let out = joined(&terms, WHISPER_BUDGET);
        assert!(!out.is_empty());
        assert!(out.split(", ").all(|c| c == "पारंगत"));
    }

    #[test]
    fn joined_empty_is_empty() {
        assert_eq!(joined(&[], WHISPER_BUDGET), "");
    }

    #[test]
    fn llm_budget_fits_a_full_dictionary() {
        let terms = normalize(&(0..MAX_TERMS).map(|i| format!("term{i}")).collect::<Vec<_>>());
        let out = joined(&terms, LLM_BUDGET);
        assert_eq!(out.split(", ").count(), terms.len());
    }
}
