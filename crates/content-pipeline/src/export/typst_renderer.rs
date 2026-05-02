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
                }
                Event::End(TagEnd::CodeBlock) => {
                    output.push_str("\")");
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
                    output.push('_');
                }
                Event::End(TagEnd::Emphasis) => {
                    output.push('_');
                }
                Event::Start(Tag::Strong) => {
                    output.push('*');
                }
                Event::End(TagEnd::Strong) => {
                    output.push('*');
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
                    output.push_str(&format!("${}$", math));
                }
                Event::DisplayMath(math) => {
                    output.push_str(&format!("$ {} $", math));
                }
                Event::Text(text) => {
                    // Escape Typst special characters
                    let escaped = text
                        .replace('#', "\\#")
                        .replace('[', "\\[")
                        .replace(']', "\\]");
                    output.push_str(&escaped);
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

        output
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

#set text(font: ("Inter", "Noto Sans"), size: 11pt, lang: "en")
#set par(justify: true, leading: 0.6em)
#set heading(numbering: "1.")

#show math.equation: set text(font: ("New Computer Modern Math", "Latin Modern Math"))

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

#set text(font: ("Inter", "Noto Sans"), size: 11pt)

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
}
