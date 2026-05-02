use content_pipeline::export::TypstRenderer;
use serde_json::json;

#[test]
fn test_render_simple_chapter() {
    let renderer = TypstRenderer::new();
    let chapter = json!({
        "title": "Introduction to Python",
        "objectives": ["Learn basics", "Write programs"],
        "content": "# Hello\n\nThis is content.",
        "estimated_minutes": 30
    });

    let typst = renderer.render_chapter(&chapter).unwrap();

    assert!(typst.contains("= Introduction to Python"));
    assert!(typst.contains("Learn basics"));
    assert!(typst.contains("Write programs"));
    assert!(typst.contains("30 minutes"));
}

#[test]
fn test_render_chapter_with_exercises() {
    let renderer = TypstRenderer::new();
    let chapter = json!({
        "title": "Test Chapter",
        "exercises": [
            {
                "question": "What is 2+2?",
                "type": "short_answer"
            },
            {
                "question": "Choose the correct answer:",
                "type": "multiple_choice",
                "options": ["A", "B", "C", "D"]
            }
        ]
    });

    let typst = renderer.render_chapter(&chapter).unwrap();

    assert!(typst.contains("== Exercises"));
    assert!(typst.contains("Exercise 1"));
    assert!(typst.contains("What is 2+2?"));
    assert!(typst.contains("Exercise 2"));
    assert!(typst.contains("A"));
    assert!(typst.contains("B"));
}

#[test]
fn test_render_curriculum() {
    let renderer = TypstRenderer::new();
    let curriculum = json!({
        "title": "Python Programming",
        "description": "A comprehensive Python course",
        "estimated_duration": "10 hours"
    });

    let typst = renderer.render_curriculum(&curriculum).unwrap();

    assert!(typst.contains("Python Programming"));
    assert!(typst.contains("A comprehensive Python course"));
    assert!(typst.contains("10 hours"));
    assert!(typst.contains("#outline("));
    assert!(typst.contains("Table of Contents"));
}

#[test]
fn test_markdown_to_typst_conversion() {
    let renderer = TypstRenderer::new();
    let markdown = "# Hello\n\nThis is **bold** and *italic*.";

    let typst = renderer.render_markdown_to_typst(markdown);

    assert!(typst.contains("= Hello"));
    assert!(typst.contains("*bold*"));
    assert!(typst.contains("_italic_"));
}

#[test]
fn test_escape_typst_special_chars() {
    let renderer = TypstRenderer::new();
    let markdown = "Use # to start commands and [brackets] for links.";

    let typst = renderer.render_markdown_to_typst(markdown);

    assert!(typst.contains("\\#"));
    assert!(typst.contains("\\["));
    assert!(typst.contains("\\]"));
}

#[test]
fn test_render_chapter_with_key_concepts() {
    let renderer = TypstRenderer::new();
    let chapter = json!({
        "title": "Variables",
        "key_concepts": ["int", "float", "string", "boolean"]
    });

    let typst = renderer.render_chapter(&chapter).unwrap();

    assert!(typst.contains("== Key Concepts"));
    assert!(typst.contains("- int"));
    assert!(typst.contains("- float"));
    assert!(typst.contains("- string"));
    assert!(typst.contains("- boolean"));
}

#[test]
fn test_render_chapter_with_prerequisites() {
    let renderer = TypstRenderer::new();
    let chapter = json!({
        "title": "Advanced Topics",
        "prerequisites": ["Basic Python", "Functions"]
    });

    let typst = renderer.render_chapter(&chapter).unwrap();

    assert!(typst.contains("== Prerequisites"));
    assert!(typst.contains("- Basic Python"));
    assert!(typst.contains("- Functions"));
}

#[test]
fn test_render_empty_chapter() {
    let renderer = TypstRenderer::new();
    let chapter = json!({
        "title": "Empty Chapter"
    });

    let typst = renderer.render_chapter(&chapter).unwrap();

    assert!(typst.contains("= Empty Chapter"));
}
