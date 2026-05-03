<task>
Repair invalid chapter Markdown without changing the lesson's meaning.
</task>

<input>
- **chapter_id**: `{{chapter_id}}`
- **chapter_title**: `{{chapter_title}}`
</input>

<instructions>
You will receive:

1. A short list of Markdown validation issues detected in the generated chapter.
2. The original chapter Markdown.

Your job is to return a repaired Markdown version that preserves the teaching content while fixing syntax and formatting problems.

## Repair priorities

- Preserve the original lesson meaning, structure, and scope.
- Remove clipboard/editor placeholder artifacts such as `[Pasted ~2 lines]`.
- Repair malformed Markdown tables so every row has the same column count.
- If a table cell contains a literal pipe character `|`, escape it as `\|` or wrap the content in inline code.
- If a table cannot be repaired safely, rewrite only that table as a bullet list or short prose comparison.
- Keep headings, examples, exercises, and explanations aligned with the original chapter.

## Hard constraints

- Do NOT invent new concepts, examples, or exercises beyond what is already present.
- Do NOT summarize the whole chapter into something shorter just to avoid fixing the Markdown.
- Do NOT output any explanation, notes, or metadata about the repair process.
- Do NOT wrap the whole response in a code block.
- Return plain Markdown only.
</instructions>

<output_format>
Plain Markdown content only. No JSON. No commentary. No surrounding code fence.
</output_format>
