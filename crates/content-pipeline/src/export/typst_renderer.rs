use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Parser, Tag, TagEnd};

use crate::error::ExportError;

pub struct TypstRenderer {
    heading_depth_offset: u8,
}

impl TypstRenderer {
    pub fn new() -> Self {
        Self {
            heading_depth_offset: 0,
        }
    }

    pub fn with_offset(offset: u8) -> Self {
        Self {
            heading_depth_offset: offset,
        }
    }

    /// Render Markdown to Typst markup
    pub fn render_markdown_to_typst(&self, markdown: &str) -> String {
        let parser = Parser::new(markdown);
        let mut output = String::new();
        let mut in_code_block = false;

        for event in parser {
            match event {
                Event::Start(Tag::Heading { level, .. }) => {
                    let typst_level = match level {
                        HeadingLevel::H1 => 1 + self.heading_depth_offset,
                        HeadingLevel::H2 => 2 + self.heading_depth_offset,
                        HeadingLevel::H3 => 3 + self.heading_depth_offset,
                        HeadingLevel::H4 => 4 + self.heading_depth_offset,
                        HeadingLevel::H5 => 5 + self.heading_depth_offset,
                        HeadingLevel::H6 => 6 + self.heading_depth_offset,
                    };
                    output.push_str(&format!("{} ", "=".repeat(typst_level as usize)));
                }
                Event::End(TagEnd::Heading(_)) => {
                    output.push('\n');
                }
                Event::Start(Tag::Paragraph) => {}
                Event::End(TagEnd::Paragraph) => {
                    output.push_str("\n\n");
                }
                Event::Start(Tag::CodeBlock(kind)) => {
                    let lang = match kind {
                        CodeBlockKind::Fenced(lang) => lang.to_string(),
                        _ => String::new(),
                    };
                    output.push_str("#show raw: set block(below: 0.5em, above: 0.5em)\n");
                    if !lang.is_empty() {
                        output.push_str(&format!("#raw(lang: \"{}\", block: true, \"", lang));
                    } else {
                        output.push_str("#raw(block: true, \"");
                    }
                    in_code_block = true;
                }
                Event::End(TagEnd::CodeBlock) => {
                    output.push_str("\")");
                    in_code_block = false;
                }
                Event::Start(Tag::List(Some(_))) => {
                    output.push_str("#enum(\n");
                }
                Event::End(TagEnd::List(true)) => {
                    output.push_str(")\n");
                }
                Event::Start(Tag::List(None)) => {
                    output.push_str("#list(\n");
                }
                Event::End(TagEnd::List(false)) => {
                    output.push_str(")\n");
                }
                Event::Start(Tag::Item) => {
                    output.push_str("  [");
                }
                Event::End(TagEnd::Item) => {
                    output.push_str("],\n");
                }
                Event::Start(Tag::Emphasis) => {
                    output.push_str("#emph[");
                }
                Event::End(TagEnd::Emphasis) => {
                    output.push(']');
                }
                Event::Start(Tag::Strong) => {
                    output.push_str("#strong[");
                }
                Event::End(TagEnd::Strong) => {
                    output.push(']');
                }
                Event::Start(Tag::Link { dest_url, .. }) => {
                    output.push_str("#link(\"");
                    output.push_str(&dest_url);
                    output.push_str("\")[");
                }
                Event::End(TagEnd::Link) => {
                    output.push(']');
                }
                Event::InlineMath(math) => {
                    output.push_str(&format!("${}$", fix_typst_math(&math)));
                }
                Event::DisplayMath(math) => {
                    output.push_str(&format!("$ {} $", fix_typst_math(&math)));
                }
                Event::Text(text) => {
                    if in_code_block {
                        // Inside #raw("…") string — escape backslash and
                        // double-quote so they don't break the Typst string.
                        output.push_str(
                            &text
                                .replace('\\', "\\\\")
                                .replace('"', "\\\""),
                        );
                    } else {
                        // Escape Typst special characters in prose
                        let escaped = text
                            .replace('#', "\\#")
                            .replace('[', "\\[")
                            .replace(']', "\\]");
                        output.push_str(&escaped);
                    }
                }
                Event::Start(Tag::Table(alignments)) => {
                    let cols = alignments.len();
                    output.push_str(&format!("#table(\n  columns: {},\n", cols));
                }
                Event::End(TagEnd::Table) => {
                    output.push_str(")\n");
                }
                Event::Start(Tag::TableHead) => {}
                Event::End(TagEnd::TableHead) => {}
                Event::Start(Tag::TableRow) => {
                    output.push_str("  ");
                }
                Event::End(TagEnd::TableRow) => {
                    output.push_str(",\n");
                }
                Event::Start(Tag::TableCell) => {
                    output.push('[');
                }
                Event::End(TagEnd::TableCell) => {
                    output.push(']');
                }
                Event::SoftBreak => {
                    output.push(' ');
                }
                Event::HardBreak => {
                    output.push_str("\\\n");
                }
                _ => {}
            }
        }

        // Post-process: pulldown-cmark without the `math` feature emits $…$
        // as Text events.  The $ is not escaped so Typst receives it as math
        // markup.  Fix LaTeX math conventions (adjacent letters = multiplication)
        // inside those spans.
        fix_typst_math_spans(&output)
    }

    /// Render chapter data to Typst source
    pub fn render_chapter(&self, chapter: &serde_json::Value) -> Result<String, ExportError> {
        let title = chapter
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("Untitled Chapter");

        let mut typst = String::new();

        // Page setup
        typst.push_str(
            r#"#set page(
  paper: "a4",
  margin: (x: 2.5cm, y: 2cm),
  header: align(right)[
    #text(size: 9pt, fill: luma(150))[
      CHAPTER_TITLE
    ]
  ],
  footer: context [
    #text(size: 9pt, fill: luma(150))[
      Page #counter(page).display()
    ]
  ],
)

#set text(size: 11pt, lang: "en")
#set par(justify: true, leading: 0.6em)
#set heading(numbering: "1.")

"#,
        );
        typst = typst.replace("CHAPTER_TITLE", &escape_typst(title));

        // Title
        typst.push_str(&format!("= {}\n\n", escape_typst(title)));

        // Estimated time
        if let Some(minutes) = chapter.get("estimated_minutes").and_then(|v| v.as_u64()) {
            typst.push_str("#align(center)[\n");
            typst.push_str("  #text(size: 10pt, fill: luma(150))[\n");
            typst.push_str(&format!("    Estimated time: {} minutes\n", minutes));
            typst.push_str("  ]\n");
            typst.push_str("]\n\n");
        }

        // Learning objectives
        if let Some(objectives) = chapter.get("objectives").and_then(|v| v.as_array()) {
            if !objectives.is_empty() {
                typst.push_str("== Learning Objectives\n");
                for obj in objectives {
                    if let Some(obj_str) = obj.as_str() {
                        typst.push_str(&format!("- {}\n", escape_typst(obj_str)));
                    }
                }
                typst.push('\n');
            }
        }

        // Prerequisites
        if let Some(prereqs) = chapter.get("prerequisites").and_then(|v| v.as_array()) {
            if !prereqs.is_empty() {
                typst.push_str("== Prerequisites\n");
                for prereq in prereqs {
                    if let Some(prereq_str) = prereq.as_str() {
                        typst.push_str(&format!("- {}\n", escape_typst(prereq_str)));
                    }
                }
                typst.push('\n');
            }
        }

        typst.push_str("#v(1em)\n\n");

        // Content
        if let Some(content) = chapter.get("content").and_then(|v| v.as_str()) {
            typst.push_str(&self.render_markdown_to_typst(content));
            typst.push('\n');
        }

        // Key concepts
        if let Some(concepts) = chapter.get("key_concepts").and_then(|v| v.as_array()) {
            if !concepts.is_empty() {
                typst.push_str("== Key Concepts\n");
                for concept in concepts {
                    if let Some(concept_str) = concept.as_str() {
                        typst.push_str(&format!("- {}\n", escape_typst(concept_str)));
                    }
                }
                typst.push('\n');
            }
        }

        // Exercises
        if let Some(exercises) = chapter.get("exercises").and_then(|v| v.as_array()) {
            if !exercises.is_empty() {
                typst.push_str("#pagebreak()\n\n== Exercises\n");
                for (i, exercise) in exercises.iter().enumerate() {
                    typst.push_str("#set heading(numbering: none)\n");
                    typst.push_str(&format!("=== Exercise {}\n", i + 1));
                    typst.push_str("#set heading(numbering: \"1.\")\n\n");

                    if let Some(question) = exercise.get("question").and_then(|v| v.as_str()) {
                        typst.push_str(&format!("{}\n\n", escape_typst(question)));
                    }

                    if let Some(options) = exercise.get("options").and_then(|v| v.as_array()) {
                        for (j, option) in options.iter().enumerate() {
                            if let Some(opt_str) = option.as_str() {
                                typst.push_str(&format!("{}. {}\n", j + 1, escape_typst(opt_str)));
                            }
                        }
                        typst.push_str("#v(0.5em)\n");
                    }
                }
            }
        }

        Ok(typst)
    }

    /// Render curriculum data to Typst source
    pub fn render_curriculum(&self, curriculum: &serde_json::Value) -> Result<String, ExportError> {
        let title = curriculum
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("Untitled Curriculum");

        let description = curriculum
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let mut typst = String::new();

        // Page setup
        typst.push_str(
            r#"#set page(
  paper: "a4",
  margin: (x: 2.5cm, y: 2cm),
  footer: context [
    #text(size: 9pt, fill: luma(150))[Page #counter(page).display()]
  ],
)

#set text(size: 11pt)

"#,
        );

        // Title page
        typst.push_str("#align(center + horizon)[\n");
        typst.push_str("  #v(4cm)\n");
        typst.push_str(&format!(
            "  #text(size: 24pt, weight: \"bold\")[{}]\n",
            escape_typst(title)
        ));
        typst.push_str("  #v(0.5cm)\n");
        typst.push_str(&format!(
            "  #text(size: 14pt, fill: luma(100))[{}]\n",
            escape_typst(description)
        ));
        typst.push_str("  #v(1cm)\n\n");

        if let Some(duration) = curriculum
            .get("estimated_duration")
            .and_then(|v| v.as_str())
        {
            typst.push_str(&format!(
                "  #text(size: 11pt)[Estimated duration: {}]\n",
                escape_typst(duration)
            ));
        }

        typst.push_str("  #v(2cm)\n");
        typst.push_str(
            "  #text(size: 10pt, fill: luma(150))[\n    Generated by Blup Learning Platform\n  ]\n",
        );
        typst.push_str("]\n\n");

        // Table of contents
        typst.push_str("#pagebreak()\n\n#outline(\n  title: [Table of Contents],\n  depth: 2,\n)\n\n#pagebreak()\n\n");

        // Chapters placeholder
        typst.push_str("// Chapters would be included here\n");
        typst.push_str("// #for chapter in data.at(\"chapters\") [\n");
        typst.push_str("//   #include \"chapter.typst\"\n");
        typst.push_str("//   #pagebreak()\n");
        typst.push_str("// ]\n");

        Ok(typst)
    }
}

/// Scan Typst output for `$…$` math spans and fix LaTeX math conventions.
///
/// `#raw(…, […])` content blocks are protected first so that bare `$` inside
/// code blocks is not misinterpreted as math delimiters.
fn fix_typst_math_spans(text: &str) -> String {
    // Protect #raw(…, […]) blocks with placeholders
    let (protected, raw_blocks) = protect_raw_blocks(text);

    let mut out = String::with_capacity(protected.len());
    for line in protected.lines() {
        let mut processed = String::with_capacity(line.len());
        let chars: Vec<char> = line.chars().collect();
        let n = chars.len();
        let mut j = 0;

        while j < n {
            if chars[j] == '\\' && j + 1 < n && chars[j + 1] == '$' {
                processed.push_str("\\$");
                j += 2;
                continue;
            }

            if chars[j] == '$' {
                // Display math $$…$$
                if j + 1 < n && chars[j + 1] == '$' {
                    if let Some(end) = line[j + 2..].find("$$") {
                        let body = &line[j + 2..j + 2 + end];
                        processed.push_str(&format!("$ {} $", fix_typst_math(body)));
                        j += 2 + end + 2;
                        continue;
                    }
                }
                // Inline math $…$
                if let Some(end) = line[j + 1..].find('$') {
                    let body = &line[j + 1..j + 1 + end];
                    processed.push_str(&format!("${}$", fix_typst_math(body)));
                    j += 1 + end + 1;
                    continue;
                }
            }

            processed.push(chars[j]);
            j += 1;
        }

        out.push_str(&processed);
        out.push('\n');
    }

    if !protected.ends_with('\n') && out.ends_with('\n') {
        out.pop();
    }

    // Restore #raw(…) blocks
    let mut result = out;
    for (idx, block) in raw_blocks.iter().enumerate() {
        result = result.replace(&format!("\x00RAW{}\x00", idx), block);
    }
    result
}

/// Find every `#raw(…, "…")` span (code block as string) and replace it
/// with a placeholder.  Returns the modified text and the original block
/// texts so that `$` inside code blocks is not misinterpreted as math.
///
/// We must track escaped characters (`\"`, `\\`) inside the string so that
/// we correctly find the closing `"` that ends the raw content.
fn protect_raw_blocks(text: &str) -> (String, Vec<String>) {
    let mut blocks: Vec<String> = Vec::new();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;

    while i < text.len() {
        if text[i..].starts_with("#raw(") {
            // Look for the string that holds the code content.
            // #raw(block: true, "…") or #raw(lang: "…", block: true, "…")
            // The last " argument before the closing ) is the code body.
            // Find the last unescaped " before ) that ends the call.
            if let Some(body_start) = find_raw_body_start(&text[i..]) {
                let abs_start = i + body_start;
                // body_start points to the opening " of the code string
                // Now scan for closing " (unescaped)
                let mut j = abs_start + 1;
                let bytes = text.as_bytes();
                while j < bytes.len() {
                    if bytes[j] == b'\\' && j + 1 < bytes.len() {
                        j += 2; // skip escaped char
                        continue;
                    }
                    if bytes[j] == b'"' {
                        // Found closing ". Expect ) to follow.
                        let end = if j + 1 < bytes.len() && bytes[j + 1] == b')' {
                            j + 1 // include )
                        } else {
                            j
                        };
                        let full = text[i..=end].to_string();
                        blocks.push(full);
                        out.push_str(&format!("\x00RAW{}\x00", blocks.len() - 1));
                        i = end + 1;
                        break;
                    }
                    j += 1;
                }
                if j < bytes.len() {
                    continue; // already processed
                }
            }
        }
        // Copy one char forward
        if let Some(c) = text[i..].chars().next() {
            out.push(c);
            i += c.len_utf8();
        } else {
            break;
        }
    }

    (out, blocks)
}

/// Given text starting with `#raw(…`, find the byte offset of the opening
/// `"` that starts the code body (the last string argument).
fn find_raw_body_start(text: &str) -> Option<usize> {
    // Skip "#raw("
    let after_open = &text[5..];
    // The code body string is the last "…" argument before the closing ).
    // Walk from the end backwards to find the opening ".
    let mut last_quote = None;
    let bytes = after_open.as_bytes();
    let mut j = 0;
    while j < bytes.len() {
        if bytes[j] == b'\\' && j + 1 < bytes.len() {
            j += 2;
            continue;
        }
        if bytes[j] == b'"' {
            last_quote = Some(j);
        }
        j += 1;
    }
    last_quote.map(|pos| 5 + pos)
}

/// Adapt LaTeX-style math to Typst syntax.
///
/// Typst treats adjacent letters as a single variable identifier (e.g. `mc`
/// is one variable), while LaTeX treats them as individual variables
/// multiplied. Insert spaces between adjacent ASCII letters to preserve
/// LaTeX semantics.
///
/// Also strips the backslash from LaTeX commands (`\sum` → `sum`,
/// `\alpha` → `alpha`) since Typst math uses bare names.
fn fix_typst_math(math: &str) -> String {
    // Protect LaTeX commands: strip backslash and save as placeholder
    // so that the letters inside the command name are not split apart
    let re_cmd = regex::Regex::new(r"\\[a-zA-Z]+").unwrap();
    let mut placeholders: Vec<String> = Vec::new();
    let protected = re_cmd.replace_all(math, |caps: &regex::Captures| {
        let name = caps[0][1..].to_string(); // drop leading backslash
        placeholders.push(name);
        format!("\x00{}\x00", placeholders.len() - 1)
    });

    // Insert spaces between adjacent ASCII letters
    let re_letters = regex::Regex::new(r"([a-zA-Z])([a-zA-Z])").unwrap();
    let mut result = re_letters
        .replace_all(&protected, "$1 $2")
        .to_string();

    // Restore identifiers (now without backslash for Typst compatibility)
    for (i, name) in placeholders.iter().enumerate() {
        result = result.replace(&format!("\x00{}\x00", i), name);
    }

    result
}

fn escape_typst(text: &str) -> String {
    text.replace('#', "\\#")
        .replace('[', "\\[")
        .replace(']', "\\]")
        .replace('$', "\\$")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_simple_markdown() {
        let renderer = TypstRenderer::new();
        let markdown = "# Hello\n\nThis is a paragraph.";
        let typst = renderer.render_markdown_to_typst(markdown);
        assert!(typst.contains("= Hello"));
        assert!(typst.contains("This is a paragraph."));
    }

    #[test]
    fn test_escape_typst() {
        assert_eq!(escape_typst("hello #world"), "hello \\#world");
        assert_eq!(escape_typst("[test]"), "\\[test\\]");
    }

    #[test]
    fn test_fix_typst_math() {
        // Adjacent letters get spaces
        assert_eq!(fix_typst_math("E = mc^2"), "E = m c^2");
        // LaTeX commands: backslash stripped, name preserved as one token
        assert_eq!(fix_typst_math(r"\alpha + \beta"), "alpha + beta");
        assert_eq!(fix_typst_math(r"\sin x + \cos y"), "sin x + cos y");
        // Multiple adjacent letters get split
        assert_eq!(fix_typst_math("ab cd"), "a b c d");
        // Inside braces, letters still get split; \frac becomes frac
        assert_eq!(fix_typst_math(r"\frac{ab}{cd}"), "frac{a b}{c d}");
    }

    #[test]
    fn test_math_and_code_blocks_render_without_delimiter_errors() {
        let renderer = TypstRenderer::new();
        let chapter = serde_json::json!({
            "title": "Test",
            "content": "# Math & Code\n\nInline math: $E = mc^2$\n\nDisplay: $$\\sum x_i$$\n\n```python\nprint(\"hello $world\")\nx = f\"${var}\"\n```\n\nAfter code: $a+b=c$.\n\n```sh\necho \"$HOME\"\n```",
            "estimated_minutes": 10,
            "objectives": [],
            "prerequisites": [],
            "key_concepts": [],
            "exercises": []
        });
        let source = renderer.render_chapter(&chapter).unwrap();

        // Math in prose should be fixed
        assert!(source.contains("$E = m c^2$"), "inline math should get spaces");
        assert!(source.contains("$a+b=c$"), "inline math after code block should work");

        // Code block uses #raw("...") string with escaped quotes
        assert!(source.contains("#raw(lang: \"python\", block: true, \""), "should use string block");
        assert!(source.contains("print(\\\"hello"), "quotes in code should be escaped");

        // Verify the raw block syntax is intact: should NOT have unescaped "
        // inside #raw() strings (check that the #raw(...) blocks are well-formed)
        // The Typst compiler can verify this — try compiling
        let dir = tempfile::TempDir::new().unwrap();
        let input = dir.path().join("input.typ");
        let output = dir.path().join("output.pdf");
        std::fs::write(&input, &source).unwrap();

        let result = std::process::Command::new("typst")
            .args(["compile", input.to_str().unwrap(), output.to_str().unwrap()])
            .output();

        match result {
            Ok(out) if out.status.success() => {
                let pdf = std::fs::read(&output).unwrap();
                assert!(pdf.starts_with(b"%PDF-"), "should produce valid PDF");
            }
            Ok(out) => {
                panic!(
                    "typst compile failed:\nSTDERR:\n{}\n\nSOURCE (first 2000 chars):\n{}",
                    String::from_utf8_lossy(&out.stderr),
                    &source[..source.len().min(2000)]
                );
            }
            Err(_e) => {
                // typst not installed — skip compilation check
            }
        }
    }
}
