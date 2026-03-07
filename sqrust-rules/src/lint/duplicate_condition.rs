use sqrust_core::{Diagnostic, FileContext, Rule};

pub struct DuplicateCondition;

impl Rule for DuplicateCondition {
    fn name(&self) -> &'static str {
        "Lint/DuplicateCondition"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        let source = &ctx.source;
        let bytes = source.as_bytes();
        let skip = build_skip(bytes);

        let mut diags = Vec::new();

        // Find all WHERE and HAVING clauses and check each for duplicate conditions.
        check_clauses(source, &skip, &mut diags);

        diags
    }
}

/// Scans the source for WHERE and HAVING clauses and emits a diagnostic for
/// each duplicate condition found within a clause.
fn check_clauses(source: &str, skip: &[bool], diags: &mut Vec<Diagnostic>) {
    let bytes = source.as_bytes();
    let len = bytes.len();
    // Uppercase copy for case-insensitive keyword matching.
    let upper: Vec<u8> = bytes.iter().map(|b| b.to_ascii_uppercase()).collect();

    let mut i = 0usize;
    while i < len {
        if skip[i] {
            i += 1;
            continue;
        }

        // Try to match WHERE or HAVING at position i.
        let clause_start_opt = if let Some(after) = match_keyword_at(&upper, skip, i, len, b"WHERE") {
            Some((after, i))
        } else if let Some(after) = match_keyword_at(&upper, skip, i, len, b"HAVING") {
            Some((after, i))
        } else {
            None
        };

        if let Some((after_kw, kw_start)) = clause_start_opt {
            // Extract clause text: from after_kw until the next clause-terminating
            // keyword (outside strings/comments) or end of source.
            let clause_end = find_clause_end(&upper, skip, after_kw, len);
            let clause_source = &source[after_kw..clause_end];
            let clause_skip = &skip[after_kw..clause_end];

            // Check for duplicates within this clause.
            check_clause_for_duplicates(
                source,
                clause_source,
                clause_skip,
                after_kw,
                kw_start,
                diags,
            );

            // Advance past the keyword to avoid re-matching it.
            i = after_kw;
            continue;
        }

        i += 1;
    }
}

/// Returns the end offset (exclusive) of a WHERE/HAVING clause body, i.e.
/// the offset at which the next statement-terminating keyword begins, or the
/// end of source.
///
/// Terminating keywords: GROUP, ORDER, HAVING, LIMIT, UNION, EXCEPT,
/// INTERSECT, `;`.
fn find_clause_end(upper: &[u8], skip: &[bool], start: usize, len: usize) -> usize {
    let terminators: &[&[u8]] = &[
        b"GROUP", b"ORDER", b"HAVING", b"LIMIT", b"UNION", b"EXCEPT", b"INTERSECT",
    ];
    let mut i = start;
    while i < len {
        if skip[i] {
            i += 1;
            continue;
        }
        // Semicolon terminates the clause.
        if upper[i] == b';' {
            return i;
        }
        // Check each terminating keyword.
        for kw in terminators {
            if match_keyword_at(upper, skip, i, len, kw).is_some() {
                return i;
            }
        }
        i += 1;
    }
    len
}

/// Splits a clause body on ` AND ` and ` OR ` (case-insensitive, spaces
/// required), normalises each piece, then reports any duplicate that appears
/// for the second or subsequent time.
fn check_clause_for_duplicates(
    full_source: &str,
    clause_text: &str,
    clause_skip: &[bool],
    clause_offset: usize,  // byte offset of clause_text within full_source
    _kw_start: usize,      // position of the WHERE/HAVING keyword (unused for now)
    diags: &mut Vec<Diagnostic>,
) {
    // Split the clause on AND / OR connectives.
    // We use a simple approach: split the lowercased clause on " and " and " or "
    // (with spaces), then map the pieces back to their original offsets.
    let conditions = split_clause(clause_text, clause_skip);

    // Normalize each condition and track where we've seen it.
    // seen: map from normalized form to the first raw occurrence.
    let mut seen: Vec<(String, usize)> = Vec::new(); // (normalized, source_offset)

    for (raw, local_offset) in conditions {
        let normalized = normalize_condition(&raw);
        if normalized.is_empty() {
            continue;
        }
        let source_offset = clause_offset + local_offset;

        let already_seen = seen.iter().any(|(norm, _)| norm == &normalized);
        if already_seen {
            // This is a duplicate — report the position of this occurrence.
            let (line, col) = offset_to_line_col(full_source, source_offset);
            diags.push(Diagnostic {
                rule: "Lint/DuplicateCondition",
                message: "Duplicate condition in WHERE/HAVING clause".to_string(),
                line,
                col,
            });
        } else {
            seen.push((normalized, source_offset));
        }
    }
}

/// Splits `text` into individual conditions by scanning for word-boundary
/// `AND` and `OR` connectives that are not inside skip regions.
/// Returns a list of `(raw_condition_text, byte_offset_within_text)` pairs.
fn split_clause<'a>(text: &'a str, skip: &[bool]) -> Vec<(String, usize)> {
    let bytes = text.as_bytes();
    let len = bytes.len();
    // Uppercase for keyword matching.
    let upper: Vec<u8> = bytes.iter().map(|b| b.to_ascii_uppercase()).collect();

    let mut conditions: Vec<(String, usize)> = Vec::new();
    let mut segment_start = 0usize;
    let mut i = 0usize;

    while i < len {
        let skip_here = skip.get(i).copied().unwrap_or(false);
        if skip_here {
            i += 1;
            continue;
        }

        // Try to match AND or OR at position i.
        let split_end = if let Some(after) = match_keyword_at(&upper, skip, i, len, b"AND") {
            Some(after)
        } else if let Some(after) = match_keyword_at(&upper, skip, i, len, b"OR") {
            Some(after)
        } else {
            None
        };

        if let Some(after_kw) = split_end {
            // Push the segment from segment_start to i (before the connective).
            let segment = text[segment_start..i].to_string();
            conditions.push((segment, segment_start));
            segment_start = after_kw;
            i = after_kw;
            continue;
        }

        i += 1;
    }

    // Push the trailing segment.
    if segment_start < len {
        let segment = text[segment_start..len].to_string();
        conditions.push((segment, segment_start));
    }

    conditions
}

/// Normalises a condition for duplicate detection:
/// - lowercase
/// - collapse runs of whitespace to a single space
/// - trim leading/trailing whitespace
fn normalize_condition(raw: &str) -> String {
    let lower = raw.to_lowercase();
    // Collapse whitespace runs.
    let mut result = String::with_capacity(lower.len());
    let mut prev_space = true; // start as true to trim leading whitespace
    for ch in lower.chars() {
        if ch.is_whitespace() {
            if !prev_space {
                result.push(' ');
                prev_space = true;
            }
        } else {
            result.push(ch);
            prev_space = false;
        }
    }
    // Trim trailing space.
    if result.ends_with(' ') {
        result.pop();
    }
    result
}

/// Returns `Some(pos_after_keyword)` if `kw` matches at `pos` in `upper`
/// with word boundaries on both sides and not inside a skip region.
fn match_keyword_at(
    upper: &[u8],
    skip: &[bool],
    pos: usize,
    len: usize,
    kw: &[u8],
) -> Option<usize> {
    let kw_len = kw.len();
    if pos + kw_len > len {
        return None;
    }
    if skip.get(pos).copied().unwrap_or(false) {
        return None;
    }
    if &upper[pos..pos + kw_len] != kw {
        return None;
    }
    // Word boundary before.
    let before_ok = pos == 0 || {
        let b = upper[pos - 1];
        !b.is_ascii_alphanumeric() && b != b'_'
    };
    // Word boundary after.
    let after_pos = pos + kw_len;
    let after_ok = after_pos >= len || {
        let b = upper[after_pos];
        !b.is_ascii_alphanumeric() && b != b'_'
    };
    if before_ok && after_ok {
        Some(after_pos)
    } else {
        None
    }
}

/// Converts a byte offset in `source` to a 1-indexed (line, col) pair.
fn offset_to_line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset.min(source.len())];
    let line = before.chars().filter(|&c| c == '\n').count() + 1;
    let col = before.rfind('\n').map(|p| offset - p - 1).unwrap_or(offset) + 1;
    (line, col)
}

/// Builds a skip table: `true` for every byte offset that is inside a
/// string literal, line comment, block comment, or quoted identifier.
fn build_skip(bytes: &[u8]) -> Vec<bool> {
    let len = bytes.len();
    let mut skip = vec![false; len];
    let mut i = 0usize;

    while i < len {
        // Line comment: -- ... end-of-line
        if i + 1 < len && bytes[i] == b'-' && bytes[i + 1] == b'-' {
            let start = i;
            while i < len && bytes[i] != b'\n' {
                i += 1;
            }
            for s in &mut skip[start..i] {
                *s = true;
            }
            continue;
        }

        // Block comment: /* ... */
        if i + 1 < len && bytes[i] == b'/' && bytes[i + 1] == b'*' {
            let start = i;
            i += 2;
            while i + 1 < len && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                i += 1;
            }
            let end = if i + 1 < len { i + 2 } else { i + 1 };
            for s in &mut skip[start..end.min(len)] {
                *s = true;
            }
            i = end;
            continue;
        }

        // Single-quoted string: '...' with '' as escaped quote
        if bytes[i] == b'\'' {
            let start = i;
            i += 1;
            while i < len {
                if bytes[i] == b'\'' {
                    if i + 1 < len && bytes[i + 1] == b'\'' {
                        i += 2; // escaped quote
                    } else {
                        i += 1; // closing quote
                        break;
                    }
                } else {
                    i += 1;
                }
            }
            for s in &mut skip[start..i.min(len)] {
                *s = true;
            }
            continue;
        }

        // Double-quoted identifier: "..."
        if bytes[i] == b'"' {
            let start = i;
            i += 1;
            while i < len && bytes[i] != b'"' {
                i += 1;
            }
            let end = if i < len { i + 1 } else { i };
            for s in &mut skip[start..end.min(len)] {
                *s = true;
            }
            i = end;
            continue;
        }

        // Backtick identifier: `...`
        if bytes[i] == b'`' {
            let start = i;
            i += 1;
            while i < len && bytes[i] != b'`' {
                i += 1;
            }
            let end = if i < len { i + 1 } else { i };
            for s in &mut skip[start..end.min(len)] {
                *s = true;
            }
            i = end;
            continue;
        }

        i += 1;
    }

    skip
}
