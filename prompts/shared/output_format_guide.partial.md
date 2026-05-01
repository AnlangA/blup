<output_format_guide>
## JSON Output

When the task requires JSON output:
- Return ONLY the JSON object. No markdown fences, no explanatory text before or after.
- The JSON must be valid and conform to the specified schema.
- All required fields must be present. Do not add fields not defined in the schema.
- Use `null` only if the schema explicitly allows it. Otherwise, provide a valid value.
- Strings must be properly escaped. Numbers must be numeric types, not strings.

## Markdown Output

When the task requires Markdown output:
- Use CommonMark syntax with KaTeX for math (`$inline$` and `$$display$$`).
- Code blocks must include language identifiers: ` ```python ... ``` `
- Heading hierarchy: `##` for chapter title, `###` for sections, `####` for subsections.
- Unordered lists: `- `. Ordered lists: `1. `.
- Use `> ` blockquotes for tips, warnings, and key insights.
- Use `**bold**` for key terms on first introduction only.

## General

- Do not wrap responses in unnecessary containers or wrappers.
- Maintain consistent formatting throughout a single response.
</output_format_guide>
