use content_pipeline::import::markdown::parse_markdown_with_headings;

#[test]
fn test_simple_markdown_parsing() {
    let md = r#"# Introduction

This is the introduction content.

## Section 1

Section 1 content here.

## Section 2

Section 2 content here.
"#;

    let chunks = parse_markdown_with_headings(md);
    assert_eq!(chunks.len(), 3);

    // First chunk: Introduction
    assert_eq!(chunks[0].0, vec!["Introduction"]);
    assert!(chunks[0].1.contains("introduction content"));

    // Second chunk: Section 1
    assert_eq!(chunks[1].0, vec!["Introduction", "Section 1"]);
    assert!(chunks[1].1.contains("Section 1 content"));

    // Third chunk: Section 2
    assert_eq!(chunks[2].0, vec!["Introduction", "Section 2"]);
    assert!(chunks[2].1.contains("Section 2 content"));
}

#[test]
fn test_nested_headings() {
    let md = r#"# Chapter 1

## Section 1.1

### Subsection 1.1.1

Content here.
"#;

    let chunks = parse_markdown_with_headings(md);
    assert_eq!(chunks.len(), 2);

    // First chunk: Chapter 1
    assert_eq!(chunks[0].0, vec!["Chapter 1"]);

    // Second chunk: Subsection
    assert_eq!(
        chunks[1].0,
        vec!["Chapter 1", "Section 1.1", "Subsection 1.1.1"]
    );
}

#[test]
fn test_code_blocks_preserved() {
    let md = r#"# Code Example

```python
def hello():
    print("world")
```

More text here.
"#;

    let chunks = parse_markdown_with_headings(md);
    assert_eq!(chunks.len(), 1);
    assert!(chunks[0].1.contains("```"));
    assert!(chunks[0].1.contains("hello()"));
}

#[test]
fn test_empty_content_skipped() {
    let md = r#"# Title

## Empty Section

## Non-Empty Section

Content here.
"#;

    let chunks = parse_markdown_with_headings(md);
    // Empty section should be skipped
    assert!(chunks.len() <= 2);
}

#[test]
fn test_inline_code() {
    let md = r"# Example

Use the `print()` function.
";

    let chunks = parse_markdown_with_headings(md);
    assert_eq!(chunks.len(), 1);
    assert!(chunks[0].1.contains("`print()`"));
}
