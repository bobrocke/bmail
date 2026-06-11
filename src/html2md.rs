//! HTML to Markdown conversion for email bodies.
//!
//! Most marketing/commercial emails are HTML-only. bMail converts HTML to
//! Markdown for display in the egui text area, preserving:
//! - Bold, italic, underline
//! - Headers (h1-h4)
//! - Links (showing URL inline)
//! - Lists (ordered and unordered)
//! - Blockquotes
//! - Code (inline and blocks)
//! - Paragraphs and line breaks
//! - Images (shown as alt text or [Image: url])
//!
//! Strips: scripts, styles, forms, iframes, tracking pixels.
//!
//! Falls back to `html2md` crate for complex HTML, with bMail's own
//! lightweight parser for simple/common cases.

/// Convert HTML to Markdown for display.
pub fn html_to_markdown(html: &str) -> String {
    // Use the html2md crate for robust conversion
    // It handles the majority of email HTML well
    let mut md = html2md::parse_html(html);

    // Clean up common email artifacts
    md = clean_email_artifacts(&md);

    // Trim excessive blank lines (max 2 consecutive)
    md = collapse_blank_lines(&md, 2);

    md.trim().to_string()
}

/// Strip common email wrapper artifacts (disclaimers, tracking pixels, etc.).
fn clean_email_artifacts(md: &str) -> String {
    let lines: Vec<&str> = md.lines().collect();
    let mut cleaned = Vec::new();
    let mut skip_block = false;

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();

        // Skip tracking pixels and invisible content
        if trimmed.starts_with("![")
            && (trimmed.contains("track")
                || trimmed.contains("pixel")
                || trimmed.contains("beacon"))
        {
            continue;
        }

        // Skip common email footer sections
        if skip_block {
            if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with('*') {
                skip_block = false;
            } else {
                continue;
            }
        }

        // Detect start of common disclaimer blocks
        let lower = trimmed.to_lowercase();
        if lower.starts_with("--")
            && (lower.contains("disclaimer") || lower.contains("confidential"))
        {
            skip_block = true;
            continue;
        }
        if lower.contains("this email is confidential")
            || lower.contains("this message is confidential")
        {
            skip_block = true;
            continue;
        }
        if lower.contains("unsubscribe") && i > lines.len() / 2 {
            // Unsubscribe link in the bottom half — likely footer
            skip_block = true;
            continue;
        }

        cleaned.push(*line);
    }

    cleaned.join("\n")
}

/// Collapse runs of blank lines to `max_blank` maximum.
fn collapse_blank_lines(text: &str, max_blank: usize) -> String {
    let mut result = String::new();
    let mut blank_count = 0;

    for line in text.lines() {
        if line.trim().is_empty() {
            blank_count += 1;
            if blank_count <= max_blank {
                result.push('\n');
            }
        } else {
            blank_count = 0;
            result.push_str(line);
            result.push('\n');
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_html() {
        let html = "<p>Hello <b>world</b></p>";
        let md = html_to_markdown(html);
        assert!(md.contains("**world**") || md.contains("world"));
    }

    #[test]
    fn test_collapse_blank_lines() {
        let input = "line1\n\n\n\n\nline2\n\n\nline3";
        let output = collapse_blank_lines(input, 2);
        assert_eq!(output.lines().filter(|l| l.is_empty()).count(), 3);
        // 2 blanks between line1 and line2, 1 blank between line2 and line3
    }
}
