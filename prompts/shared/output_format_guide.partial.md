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
- Use inline code for short identifiers, operators, file names, and one-line snippets.
- When code blocks are needed, use fenced code blocks only for multi-line code, commands, configs, or literal output that materially helps the explanation.
- Every fenced code block must include the correct language identifier for the material. Supported runnable languages: `python`, `javascript`, `typescript`, `rust`, `go`, `c`, `cpp`, `java`, `ruby`, `bash`. Non-runnable: `text`, `sql`, `html`, `css`, `json`, `yaml`, `xml`, `diff`, `toml`.
- Use `bash` for shell commands and `text` for plain-text output, transcripts, or pseudocode that is not valid source code.
- Never nest fenced code blocks or wrap the entire response in a single fenced code block.
- If the topic is code-centric, prefer languages that the learner can actually run in the platform's sandbox.
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
