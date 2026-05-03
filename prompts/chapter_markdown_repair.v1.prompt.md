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
- If the chapter contains code blocks, ensure each fenced code block has a correct language identifier from the supported list: python, javascript, typescript, rust, go, c, cpp, java, ruby, bash, text, sql, html, css, json, yaml, diff.
- If a code block is missing a language identifier or uses an unsupported one, correct it to the closest matching identifier. Do NOT add language identifiers to blocks that are clearly plain text, expected output, or transcripts — use `text` for those.
- Preserve an existing code fence language identifier when it is still correct. If it is missing or clearly wrong, choose the best-fit identifier (`bash` for shell commands, `text` for literal output/plain text, specific source language when clear).
- Keep headings, examples, exercises, and explanations aligned with the original chapter.

## Hard constraints

- Do NOT invent new concepts, examples, or exercises beyond what is already present.
- Do NOT summarize the whole chapter into something shorter just to avoid fixing the Markdown.
- Do NOT output any explanation, notes, or metadata about the repair process.
- Do NOT wrap the whole response in a code block.
- Do NOT nest fenced code blocks.
- Do NOT convert ordinary prose into code blocks unless the original content was clearly meant to be code, commands, config, or literal output.
- Return plain Markdown only.
</instructions>

<output_format>
Plain Markdown content only. No JSON. No commentary. No surrounding code fence. Internal code fences are allowed only when they are part of the repaired chapter content.
</output_format>
