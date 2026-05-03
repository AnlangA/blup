use std::sync::OnceLock;

use regex::Regex;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct MarkdownValidationIssue {
    pub code: String,
    pub message: String,
    pub line: Option<usize>,
}

#[derive(Debug, Clone, Error)]
#[error("{message}")]
pub struct MarkdownValidationError {
    message: String,
    issues: Vec<MarkdownValidationIssue>,
}

impl MarkdownValidationError {
    pub fn new(issues: Vec<MarkdownValidationIssue>) -> Self {
        let message = match issues.len() {
            0 => "Chapter Markdown validation failed".to_string(),
            1 => format_issue(&issues[0]),
            count => format!(
                "Chapter Markdown validation failed with {count} issues: {}",
                issues
                    .iter()
                    .take(3)
                    .map(format_issue)
                    .collect::<Vec<_>>()
                    .join("; ")
            ),
        };

        Self { message, issues }
    }

    pub fn issues(&self) -> &[MarkdownValidationIssue] {
        &self.issues
    }

    pub fn summary(&self) -> &str {
        &self.message
    }
}

pub fn validate_chapter_markdown(markdown: &str) -> Result<(), MarkdownValidationError> {
    let mut issues = Vec::new();
    let lines: Vec<&str> = markdown.lines().collect();
    let mut in_fenced_code = false;
    let mut idx = 0usize;

    for mat in placeholder_regex().find_iter(markdown) {
        let line = markdown[..mat.start()]
            .bytes()
            .filter(|b| *b == b'\n')
            .count()
            + 1;
        issues.push(MarkdownValidationIssue {
            code: "placeholder-artifact".to_string(),
            message: "Found clipboard/editor placeholder artifact".to_string(),
            line: Some(line),
        });
    }

    while idx < lines.len() {
        let line = lines[idx];
        let trimmed = line.trim_start();

        if is_fence_marker(trimmed) {
            in_fenced_code = !in_fenced_code;
            idx += 1;
            continue;
        }

        if in_fenced_code {
            idx += 1;
            continue;
        }

        if idx + 1 < lines.len()
            && line.contains('|')
            && delimiter_cell_count(lines[idx + 1]).is_some()
        {
            let Some(header_cells) = parse_pipe_table_row(line) else {
                idx += 1;
                continue;
            };
            let Some(delimiter_count) = delimiter_cell_count(lines[idx + 1]) else {
                idx += 1;
                continue;
            };

            if header_cells.len() != delimiter_count {
                issues.push(MarkdownValidationIssue {
                    code: "malformed-pipe-table".to_string(),
                    message: format!(
                        "Table header has {} columns but delimiter row has {} columns",
                        header_cells.len(),
                        delimiter_count
                    ),
                    line: Some(idx + 2),
                });
            }

            let expected_cols = header_cells.len();
            let mut row_idx = idx + 2;
            while row_idx < lines.len() {
                let row = lines[row_idx];
                if row.trim().is_empty() || !row.contains('|') || is_fence_marker(row.trim_start())
                {
                    break;
                }

                if let Some(row_cells) = parse_pipe_table_row(row) {
                    if row_cells.len() != expected_cols {
                        issues.push(MarkdownValidationIssue {
                            code: "malformed-pipe-table".to_string(),
                            message: format!(
                                "Table row has {} columns but expected {}; this usually means an unescaped `|` inside a cell",
                                row_cells.len(),
                                expected_cols
                            ),
                            line: Some(row_idx + 1),
                        });
                    }
                }

                row_idx += 1;
            }

            idx = row_idx;
            continue;
        }

        idx += 1;
    }

    if issues.is_empty() {
        Ok(())
    } else {
        Err(MarkdownValidationError::new(issues))
    }
}

fn format_issue(issue: &MarkdownValidationIssue) -> String {
    match issue.line {
        Some(line) => format!("line {line}: {}", issue.message),
        None => issue.message.clone(),
    }
}

fn placeholder_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(?i)\[pasted[^\]]*\]").expect("valid placeholder regex"))
}

fn delimiter_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^:?-{3,}:?$").expect("valid delimiter regex"))
}

fn is_fence_marker(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.starts_with("```") || trimmed.starts_with("~~~")
}

fn delimiter_cell_count(line: &str) -> Option<usize> {
    let cells = parse_pipe_table_row(line)?;
    if cells.len() < 2
        || !cells
            .iter()
            .all(|cell| delimiter_regex().is_match(cell.trim()))
    {
        return None;
    }
    Some(cells.len())
}

fn parse_pipe_table_row(line: &str) -> Option<Vec<String>> {
    if !line.contains('|') {
        return None;
    }

    let starts_with_pipe = line.trim_start().starts_with('|');
    let ends_with_pipe = line.trim_end().ends_with('|');
    let mut cells = Vec::new();
    let mut current = String::new();
    let chars = line.chars();
    let mut escaped = false;
    let mut in_inline_code = false;
    let mut saw_separator = false;

    for ch in chars {
        if escaped {
            current.push(ch);
            escaped = false;
            continue;
        }

        match ch {
            '\\' => {
                current.push(ch);
                escaped = true;
            }
            '`' => {
                in_inline_code = !in_inline_code;
                current.push(ch);
            }
            '|' if !in_inline_code => {
                saw_separator = true;
                cells.push(std::mem::take(&mut current));
            }
            _ => current.push(ch),
        }
    }

    cells.push(current);

    if !saw_separator {
        return None;
    }

    if starts_with_pipe && cells.first().is_some_and(|cell| cell.trim().is_empty()) {
        cells.remove(0);
    }
    if ends_with_pipe && cells.last().is_some_and(|cell| cell.trim().is_empty()) {
        cells.pop();
    }

    Some(
        cells
            .into_iter()
            .map(|cell| cell.trim().to_string())
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::validate_chapter_markdown;

    #[test]
    fn accepts_valid_table_with_escaped_pipes() {
        let markdown = "| Operator | Meaning | Example |\n|---|---|---|\n| OR | At least one must be true | `A || B` |\n| Shell | Send output onward | `ls | wc -l` |";
        assert!(validate_chapter_markdown(markdown).is_ok());
    }

    #[test]
    fn rejects_placeholder_artifacts() {
        let markdown = "## Broken\n\n[Pasted ~2 lines]\n";
        let err = validate_chapter_markdown(markdown).expect_err("should reject placeholder");
        assert!(err.summary().contains("placeholder"));
    }

    #[test]
    fn rejects_placeholder_artifacts_inside_table_rows() {
        let markdown =
            "| Operator | Meaning |\n|---|---|\n| [Pasted ~2 lines] | At least one must be true |";
        let err =
            validate_chapter_markdown(markdown).expect_err("should reject placeholder in table");
        assert!(err.summary().contains("placeholder"));
    }

    #[test]
    fn rejects_malformed_pipe_tables() {
        let markdown = "| Operator | Meaning | Example |\n|---|---|---|\n| OR | At least one must be true | A || B |";
        let err = validate_chapter_markdown(markdown).expect_err("should reject malformed table");
        assert!(err.summary().contains("unescaped `|`"));
    }

    #[test]
    fn ignores_pipe_like_text_inside_code_fences() {
        let markdown = "```text\n| not | a | real |\nA || B\n```\n";
        assert!(validate_chapter_markdown(markdown).is_ok());
    }
}
