use sqrust_core::{Diagnostic, FileContext, Rule};

use crate::capitalisation::{is_word_char, SkipMap};

pub struct SubqueryInJoinCondition;

impl Rule for SubqueryInJoinCondition {
    fn name(&self) -> &'static str {
        "Structure/SubqueryInJoinCondition"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        if !ctx.parse_errors.is_empty() {
            return Vec::new();
        }

        find_subqueries_in_join_on(&ctx.source, self.name())
    }
}

// ── source-level scan ─────────────────────────────────────────────────────────

/// Scans source for occurrences of `ON` keyword followed (after optional
/// whitespace/newlines) by `(SELECT`. Emits one diagnostic per occurrence.
///
/// The scan is a state machine:
/// 1. Find a JOIN keyword (marks that we're inside a JOIN chain)
/// 2. Look for the next ON keyword after JOIN
/// 3. After ON, skip whitespace; if `(SELECT` follows, emit a diagnostic
///
/// We re-scan the whole source to find all JOIN...ON...subquery patterns.
fn find_subqueries_in_join_on(source: &str, rule: &'static str) -> Vec<Diagnostic> {
    let bytes = source.as_bytes();
    let len = bytes.len();
    let skip_map = SkipMap::build(source);

    let mut diags = Vec::new();

    // Find all ON keyword positions where the ON is preceded by a JOIN clause.
    // Strategy: walk through source, find each ON keyword that is a whole word
    // and is code (not inside a string/comment). For each such ON, scan forward
    // skipping whitespace. If we find `(SELECT` (case-insensitive), emit a
    // diagnostic at the position of the `ON` keyword.
    //
    // To avoid false positives (ON that is not a join ON), we check that there
    // was a JOIN keyword somewhere before this ON in the same statement context.
    let join_keywords: &[&[u8]] = &[
        b"JOIN",
        b"INNER JOIN",
        b"LEFT JOIN",
        b"RIGHT JOIN",
        b"FULL JOIN",
        b"CROSS JOIN",
        b"LEFT OUTER JOIN",
        b"RIGHT OUTER JOIN",
        b"FULL OUTER JOIN",
    ];

    // Collect byte offsets of all JOIN keyword positions.
    let join_offsets = collect_keyword_offsets(bytes, len, &skip_map, b"JOIN");

    // Collect byte offsets of all ON keyword positions.
    let on_offsets = collect_keyword_offsets(bytes, len, &skip_map, b"ON");

    // For each ON, check if there is a JOIN somewhere before it (within the
    // statement). Then check if (SELECT follows the ON.
    for &on_pos in &on_offsets {
        // There must be at least one JOIN before this ON.
        let has_prior_join = join_offsets.iter().any(|&j| j < on_pos);
        if !has_prior_join {
            continue;
        }

        // Skip whitespace and newlines after the ON keyword.
        let after_on = on_pos + 2; // len("ON") == 2
        let mut scan = after_on;
        while scan < len && (bytes[scan] == b' ' || bytes[scan] == b'\t' || bytes[scan] == b'\n' || bytes[scan] == b'\r') {
            scan += 1;
        }

        // Check for `(SELECT` (case-insensitive).
        if scan + 7 <= len
            && bytes[scan] == b'('
            && bytes[scan + 1..scan + 7].eq_ignore_ascii_case(b"SELECT")
        {
            // Ensure the S-E-L-E-C-T is followed by a non-word char (i.e. it's
            // really the SELECT keyword, not something like `(SELECTX`).
            let after_select = scan + 7;
            let select_ends = after_select >= len || !is_word_char(bytes[after_select]);
            if select_ends {
                let (line, col) = offset_to_line_col(source, on_pos);
                diags.push(Diagnostic {
                    rule,
                    message: "Subquery in JOIN ON condition may prevent index use; consider pre-computing as a CTE or derived table".to_string(),
                    line,
                    col,
                });
            }
        }
    }

    // Suppress unused variable warning — join_keywords is used for documentation intent
    let _ = join_keywords;

    diags
}

/// Collect byte offsets of all whole-word occurrences of `keyword` in `bytes`
/// that are code (not inside strings/comments) according to `skip_map`.
fn collect_keyword_offsets(
    bytes: &[u8],
    len: usize,
    skip_map: &SkipMap,
    keyword: &[u8],
) -> Vec<usize> {
    let kw_len = keyword.len();
    let mut offsets = Vec::new();
    let mut i = 0;

    while i + kw_len <= len {
        if !skip_map.is_code(i) {
            i += 1;
            continue;
        }

        // Word boundary before.
        let before_ok = i == 0 || !is_word_char(bytes[i - 1]);
        if !before_ok {
            i += 1;
            continue;
        }

        // Case-insensitive match.
        let matches = bytes[i..i + kw_len]
            .iter()
            .zip(keyword.iter())
            .all(|(a, b)| a.eq_ignore_ascii_case(b));

        if matches {
            let after = i + kw_len;
            let after_ok = after >= len || !is_word_char(bytes[after]);
            let all_code = (i..i + kw_len).all(|k| skip_map.is_code(k));

            if after_ok && all_code {
                offsets.push(i);
                i += kw_len;
                continue;
            }
        }

        i += 1;
    }

    offsets
}

// ── helpers ───────────────────────────────────────────────────────────────────

/// Converts a byte offset to a 1-indexed (line, col) pair.
fn offset_to_line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset];
    let line = before.chars().filter(|&c| c == '\n').count() + 1;
    let col = before.rfind('\n').map(|p| offset - p - 1).unwrap_or(offset) + 1;
    (line, col)
}
