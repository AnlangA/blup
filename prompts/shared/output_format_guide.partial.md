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
- When code blocks are needed, include the correct language identifier for the material (for example: ` ```rust `, ` ```sql `, ` ```bash `, ` ```text `). If the topic is not code-centric, prefer prose, tables, formulas, diagrams-in-words, or worked examples instead of inventing code.
- Heading hierarchy: `##` for chapter title, `###` for sections, `####` for subsections.
- Unordered lists: `- `. Ordered lists: `1. `.
- Use `> ` blockquotes for tips, warnings, and key insights.
- Use `**bold**` for key terms on first introduction only.

## General

- Do not wrap responses in unnecessary containers or wrappers.
- Maintain consistent formatting throughout a single response.
- Examples embedded in prompts are illustrative patterns. Copy the required structure, not the sample topic, nouns, tools, or language.
- Match the output medium to the subject: use code only when the task is code-centric; otherwise prefer prose, formulas, tables, scenarios, or worked examples.
</output_format_guide>
