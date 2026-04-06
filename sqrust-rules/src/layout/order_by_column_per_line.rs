use sqrust_core::{Diagnostic, FileContext, Rule};

pub struct OrderByColumnPerLine;

impl Rule for OrderByColumnPerLine {
    fn name(&self) -> &'static str {
        "Layout/OrderByColumnPerLine"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        find_violations(&ctx.source, self.name())
    }
}

fn find_violations(source: &str, rule_name: &'static str) -> Vec<Diagnostic> {
    if !source.contains('\n') {
        return Vec::new();
    }

    let upper = source.to_ascii_uppercase();
    if !contains_keyword(&upper, b"ORDER") {
        return Vec::new();
    }

    let bytes = source.as_bytes();
    let len = bytes.len();
    let skip = build_skip_set(bytes, len);

    // Find ORDER BY position outside strings/comments
    let order_by_start = match find_clause_start(&upper, &skip, b"ORDER", b"BY") {
        Some(pos) => pos,
        None => return Vec::new(),
    };

    // Find the position right after ORDER BY (past the BY token)
    let after_order_by = find_after_two_word_keyword(&upper, order_by_start);

    // Find end of ORDER BY clause — next major clause or end of string
    let clause_end = find_clause_end(&upper, after_order_by);

    let region = &source[after_order_by..clause_end];

    let mut diags = Vec::new();

    // Calculate line number offset for diagnostics
    let lines_before = source[..after_order_by].chars().filter(|&c| c == '\n').count();

    for (rel_line_idx, line) in region.split('\n').enumerate() {
        let abs_line = lines_before + rel_line_idx + 1;

        // Check if this line has a comma that is followed by more content (not just whitespace)
        // on the same line — meaning two columns are on the same line
        let line_skip_offset = source_offset_of_region_line(source, after_order_by, rel_line_idx);

        if let Some(col) = find_inline_comma(line, line_skip_offset, &skip) {
            diags.push(Diagnostic {
                rule: rule_name,
                message: "In multi-line ORDER BY, each column should be on its own line"
                    .to_string(),
                line: abs_line,
                col,
            });
        }
    }

    diags
}

/// Returns the byte offset within `source` where the given line within the region starts.
fn source_offset_of_region_line(source: &str, region_start: usize, rel_line_idx: usize) -> usize {
    if rel_line_idx == 0 {
        return region_start;
    }
    let region = &source[region_start..];
    let mut offset = region_start;
    for (i, line) in region.split('\n').enumerate() {
        if i == rel_line_idx {
            return offset;
        }
        offset += line.len() + 1; // +1 for '\n'
    }
    offset
}

/// Find a comma on `line` that is followed by non-whitespace content on the same line
/// (meaning a second column follows on the same line). Returns 1-indexed column if found.
fn find_inline_comma(line: &str, line_abs_offset: usize, skip: &[bool]) -> Option<usize> {
    let bytes = line.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        let abs = line_abs_offset + i;

        if bytes[i] == b',' && (abs >= skip.len() || !skip[abs]) {
            // Check if there is non-whitespace content after this comma on the same line
            let rest = &bytes[i + 1..];
            let has_content_after = rest.iter().any(|&b| b != b' ' && b != b'\t');
            if has_content_after {
                return Some(i + 1); // 1-indexed column of the comma
            }
        }
        i += 1;
    }

    None
}

/// Find the position right after ORDER BY (skipping past the BY and any trailing whitespace).
fn find_after_two_word_keyword(upper: &str, keyword_start: usize) -> usize {
    let bytes = upper.as_bytes();
    let len = bytes.len();

    // Skip first word (ORDER or GROUP)
    let mut i = keyword_start;
    while i < len && is_word_char(bytes[i]) {
        i += 1;
    }
    // Skip whitespace between words
    while i < len && (bytes[i] == b' ' || bytes[i] == b'\t' || bytes[i] == b'\n') {
        i += 1;
    }
    // Skip second word (BY)
    while i < len && is_word_char(bytes[i]) {
        i += 1;
    }

    i
}

/// Returns the start position of the ORDER BY / GROUP BY clause in the source.
fn find_clause_start(upper: &str, skip: &[bool], word1: &[u8], word2: &[u8]) -> Option<usize> {
    let bytes = upper.as_bytes();
    let len = bytes.len();
    let w1_len = word1.len();
    let w2_len = word2.len();

    let mut i = 0;
    while i + w1_len <= len {
        if &bytes[i..i + w1_len] == word1 {
            let abs = i;
            let before_ok = i == 0 || !is_word_char(bytes[i - 1]);
            let after_ok = i + w1_len >= len || !is_word_char(bytes[i + w1_len]);

            if before_ok && after_ok && (abs >= skip.len() || !skip[abs]) {
                // Scan forward past whitespace for second word
                let mut j = i + w1_len;
                while j < len && (bytes[j] == b' ' || bytes[j] == b'\t' || bytes[j] == b'\n') {
                    j += 1;
                }
                if j + w2_len <= len && &bytes[j..j + w2_len] == word2 {
                    let by_after_ok = j + w2_len >= len || !is_word_char(bytes[j + w2_len]);
                    let abs_by = j;
                    if by_after_ok && (abs_by >= skip.len() || !skip[abs_by]) {
                        return Some(i);
                    }
                }
            }
        }
        i += 1;
    }

    None
}

/// Find the end of the current ORDER BY / GROUP BY clause block.
/// Ends at the start of the next top-level clause or end of string.
fn find_clause_end(upper: &str, from: usize) -> usize {
    let bytes = upper.as_bytes();
    let len = bytes.len();

    // Keywords that terminate the current clause
    const TERMINATORS: &[&[u8]] = &[
        b"LIMIT", b"HAVING", b"UNION", b"INTERSECT", b"EXCEPT",
    ];

    let mut i = from;
    while i < len {
        // Check for a semicolon
        if bytes[i] == b';' {
            return i;
        }

        for term in TERMINATORS {
            let t_len = term.len();
            if i + t_len <= len && &bytes[i..i + t_len] == *term {
                let before_ok = i == 0 || !is_word_char(bytes[i - 1]);
                let after_ok = i + t_len >= len || !is_word_char(bytes[i + t_len]);
                if before_ok && after_ok {
                    return i;
                }
            }
        }

        i += 1;
    }

    len
}

fn contains_keyword(upper: &str, keyword: &[u8]) -> bool {
    let bytes = upper.as_bytes();
    let len = bytes.len();
    let kw_len = keyword.len();
    let mut i = 0;
    while i + kw_len <= len {
        if &bytes[i..i + kw_len] == keyword {
            let before_ok = i == 0 || !is_word_char(bytes[i - 1]);
            let after_ok = i + kw_len >= len || !is_word_char(bytes[i + kw_len]);
            if before_ok && after_ok {
                return true;
            }
        }
        i += 1;
    }
    false
}

#[inline]
fn is_word_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// Build a boolean skip-set: `skip[i] == true` means byte `i` is inside a
/// string literal, quoted identifier, or comment.
fn build_skip_set(bytes: &[u8], len: usize) -> Vec<bool> {
    let mut skip = vec![false; len];
    let mut i = 0;

    while i < len {
        // Single-quoted string
        if bytes[i] == b'\'' {
            skip[i] = true;
            i += 1;
            while i < len {
                skip[i] = true;
                if bytes[i] == b'\'' {
                    if i + 1 < len && bytes[i + 1] == b'\'' {
                        i += 1;
                        skip[i] = true;
                        i += 1;
                        continue;
                    }
                    i += 1;
                    break;
                }
                i += 1;
            }
            continue;
        }

        // Double-quoted identifier
        if bytes[i] == b'"' {
            skip[i] = true;
            i += 1;
            while i < len {
                skip[i] = true;
                if bytes[i] == b'"' {
                    if i + 1 < len && bytes[i + 1] == b'"' {
                        i += 1;
                        skip[i] = true;
                        i += 1;
                        continue;
                    }
                    i += 1;
                    break;
                }
                i += 1;
            }
            continue;
        }

        // Block comment
        if i + 1 < len && bytes[i] == b'/' && bytes[i + 1] == b'*' {
            skip[i] = true;
            skip[i + 1] = true;
            i += 2;
            while i < len {
                skip[i] = true;
                if i + 1 < len && bytes[i] == b'*' && bytes[i + 1] == b'/' {
                    skip[i + 1] = true;
                    i += 2;
                    break;
                }
                i += 1;
            }
            continue;
        }

        // Line comment
        if i + 1 < len && bytes[i] == b'-' && bytes[i + 1] == b'-' {
            skip[i] = true;
            skip[i + 1] = true;
            i += 2;
            while i < len && bytes[i] != b'\n' {
                skip[i] = true;
                i += 1;
            }
            continue;
        }

        i += 1;
    }

    skip
}
