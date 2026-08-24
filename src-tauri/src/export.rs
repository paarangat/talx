//! Serialization of transcription history to CSV and Markdown.
//!
//! Timestamps are rendered by SQLite itself via `datetime(..., 'localtime')` so
//! the export reads in the user's own timezone without pulling in a date crate.

use rusqlite::{Connection, Result};

/// A history row flattened for export, with the timestamp already rendered as a
/// local-time `YYYY-MM-DD HH:MM:SS` string.
pub struct ExportRow {
    pub created_at_local: String,
    pub word_count: u32,
    pub duration_secs: u32,
    pub original_text: String,
    pub polished_text: String,
}

/// Load the whole history, oldest first, for export.
pub fn fetch_all(conn: &Connection) -> Result<Vec<ExportRow>> {
    // `localtime` can yield NULL if the platform has no timezone database, so
    // fall back to UTC rather than losing the timestamp entirely.
    let mut stmt = conn.prepare(
        "SELECT COALESCE(
                    datetime(created_at / 1000, 'unixepoch', 'localtime'),
                    datetime(created_at / 1000, 'unixepoch')
                ),
                word_count, duration_secs, original_text, polished_text
         FROM transcriptions
         ORDER BY created_at ASC",
    )?;

    let rows = stmt.query_map([], |row| {
        Ok(ExportRow {
            created_at_local: row.get(0)?,
            word_count: row.get::<_, u32>(1)?,
            duration_secs: row.get::<_, u32>(2)?,
            original_text: row.get(3)?,
            polished_text: row.get(4)?,
        })
    })?;

    rows.collect()
}

/// Escape a single CSV field per RFC 4180.
///
/// A field containing a comma, a double quote, CR or LF is wrapped in double
/// quotes and any embedded double quote is doubled. Everything else is emitted
/// bare. Transcripts routinely contain all three, so this is load-bearing.
fn csv_field(value: &str) -> String {
    if value.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

pub fn to_csv(rows: &[ExportRow]) -> String {
    let mut out =
        String::from("created_at,word_count,duration_secs,original_text,polished_text\r\n");
    for row in rows {
        out.push_str(&format!(
            "{},{},{},{},{}\r\n",
            csv_field(&row.created_at_local),
            row.word_count,
            row.duration_secs,
            csv_field(&row.original_text),
            csv_field(&row.polished_text),
        ));
    }
    out
}

pub fn to_markdown(rows: &[ExportRow]) -> String {
    let mut out = String::from("# Talx history\n\n");
    if rows.is_empty() {
        out.push_str("_No transcriptions yet._\n");
        return out;
    }
    for row in rows {
        // ponytail: heading + paragraph rather than a table — transcripts contain
        // newlines and pipes, which no amount of escaping makes readable in a
        // Markdown table cell.
        out.push_str(&format!(
            "## {}\n\n{} words \u{b7} {}s\n\n{}\n\n",
            row.created_at_local,
            row.word_count,
            row.duration_secs,
            row.polished_text.trim(),
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(text: &str) -> ExportRow {
        ExportRow {
            created_at_local: "2026-08-24 09:30:00".to_string(),
            word_count: 7,
            duration_secs: 12,
            original_text: text.to_string(),
            polished_text: text.to_string(),
        }
    }

    #[test]
    fn csv_field_escapes_comma_quote_and_newline() {
        let hazard = "He said \"hi\", then\nleft";
        let escaped = csv_field(hazard);

        // Wrapped in quotes because it contains a comma, a quote and a newline.
        assert!(escaped.starts_with('"'), "field must be quoted: {escaped}");
        assert!(escaped.ends_with('"'), "field must be quoted: {escaped}");

        // The embedded quotes are doubled, and nothing else is altered.
        assert_eq!(escaped, "\"He said \"\"hi\"\", then\nleft\"");

        // The comma and the newline survive verbatim inside the quotes.
        let inner = &escaped[1..escaped.len() - 1];
        assert!(inner.contains(", then"), "comma must survive: {inner}");
        assert!(inner.contains("then\nleft"), "newline must survive: {inner}");

        // Unwrapping per RFC 4180 (strip the wrapping quotes, undo the doubling)
        // returns exactly the original field.
        assert_eq!(inner.replace("\"\"", "\""), hazard);
    }

    #[test]
    fn csv_field_leaves_plain_text_bare() {
        assert_eq!(csv_field("just words"), "just words");
    }

    #[test]
    fn csv_row_round_trips_a_hazardous_transcript() {
        let hazard = "He said \"hi\", then\nleft";
        let csv = to_csv(&[row(hazard)]);

        let (header, body) = csv
            .split_once("\r\n")
            .expect("csv must have a header record");
        assert_eq!(
            header,
            "created_at,word_count,duration_secs,original_text,polished_text"
        );

        // The record terminator is the only bare CRLF; the newline inside the
        // transcript stays inside its quoted field.
        assert!(body.ends_with("\r\n"), "record must end with CRLF");
        assert_eq!(body.matches("\r\n").count(), 1);

        // Walk the record splitting on commas only while outside quotes — this
        // is what any RFC 4180 reader does — and confirm the field count and
        // the recovered text.
        let record = body.trim_end_matches("\r\n");
        let mut fields: Vec<String> = Vec::new();
        let mut current = String::new();
        let mut in_quotes = false;
        let mut chars = record.chars().peekable();
        while let Some(c) = chars.next() {
            match c {
                '"' if in_quotes && chars.peek() == Some(&'"') => {
                    chars.next();
                    current.push('"');
                }
                '"' => in_quotes = !in_quotes,
                ',' if !in_quotes => fields.push(std::mem::take(&mut current)),
                _ => current.push(c),
            }
        }
        fields.push(current);

        assert_eq!(fields.len(), 5, "parsed fields: {fields:?}");
        assert_eq!(fields[0], "2026-08-24 09:30:00");
        assert_eq!(fields[1], "7");
        assert_eq!(fields[2], "12");
        assert_eq!(fields[3], hazard);
        assert_eq!(fields[4], hazard);
    }
}
