<task>
Teach a single chapter through structured, progressive content with explanations, examples, and embedded exercises.
</task>

<input>
- **chapter_id**: `{{chapter_id}}`
- **user_profile**: `{{user_profile}}`
- **curriculum_context**: `{{curriculum_context}}`
</input>

<instructions>
Produce the full chapter content as Markdown. Follow the structure below exactly. Every section must be present.

## Scope Anchoring

- Use `chapter_id` and `curriculum_context` to infer the current chapter's exact scope, title, objectives, and neighboring chapters.
- Teach only the current chapter. Do not retell the whole curriculum or introduce later material as if it already belongs here.
- Keep all explanations, examples, and exercises aligned to the chapter objectives rather than to unrelated examples elsewhere in the prompt.

## Content Structure (use as your outline)

### 1. Chapter Title (`##`)
The chapter name as an `h2` heading.

### 2. Introduction (2–3 sentences)
What the learner will achieve by the end of this chapter. Connect to what they already know from previous chapters (if any).

### 3. Learning Objectives (bullet list)
3–5 specific, measurable goals. Use action verbs. These must match the objectives defined in the curriculum plan for this chapter.

### 4. Core Content (multiple `###` sections)
Break the main teaching into logical sections. Within each section:
- Explain one concept at a time
- Provide a concrete example immediately after each concept. Use code only when the chapter topic genuinely involves programming or executable tooling; otherwise use worked examples, formulas, scenarios, tables, or analogies.
- Use `> ` blockquotes for tips and warnings
- Keep paragraphs to 3–5 sentences max

### 5. Practice Exercises (embed 2–3 throughout, not all at the end)
Place exercises after the section that teaches the relevant concept. Each exercise:
- States a clear task
- Includes a `<details><summary>See hint</summary>` hint
- Includes a `<details><summary>See solution</summary>` solution with explanation
- Uses the chapter's teaching medium: code when code is central, worked steps for quantitative topics, and prose/scenario answers for non-code topics

### 6. Key Takeaways (bullet list)
3–5 bullets summarizing the chapter's main concepts. Each bullet should be a complete, self-contained statement.

### 7. What's Next (1–2 sentences)
Brief preview of the next chapter, creating continuity and motivation.

## Differentiation by Learner Level

**For beginners (domain_knowledge: none or beginner):**
- Use more analogies connecting to everyday experience
- Break complex operations into numbered step sequences
- Include more inline exercises (every 2–3 concepts)
- For code-centric chapters, provide complete, runnable examples — no partial snippets. For non-code chapters, provide complete worked examples with all steps shown.
- Target word count: 2000–3000 words

**For intermediate learners (domain_knowledge: intermediate):**
- Move faster through basics, spend more time on patterns and best practices
- Use partial examples with "fill in the rest" exercises when that suits the medium (partial code for programming, partial worked solutions for quantitative topics, partial analyses for conceptual topics)
- Reference prior knowledge explicitly ("You already know X, so Y works similarly")
- Target word count: 1500–2500 words

**For advanced learners (domain_knowledge: advanced):**
- Focus on edge cases, performance implications, and design trade-offs
- Use shorter explanations with deeper technical references
- Include challenge exercises that require combining multiple concepts
- Target word count: 1500–2000 words

## Match the Teaching Medium to the Subject

- If the chapter is about programming or developer tooling, use runnable code, commands, configs, or syntax in the specific language/tool named by the curriculum context.
- If the chapter is quantitative but not programming, prefer formulas, worked calculations, tables, and interpretation of results.
- If the chapter is conceptual or practical but non-programming, prefer scenarios, examples, comparisons, checklists, and short practice prompts.
- Never switch to Python by default. Only use Python when the learner is actually studying Python or when Python is explicitly part of the chapter objective.
</instructions>

<output_format>
Plain Markdown content. NOT JSON. NOT wrapped in a code block.

Formatting rules:
- `##` for chapter title, `###` for major sections, `####` for subsections
- Use inline code for short syntax, identifiers, operators, filenames, and short outputs; reserve fenced code blocks for multi-line material.
- If you include a code block, it MUST use the language identifier that matches the subject matter: ` ```rust `, ` ```sql `, ` ```bash `, ` ```text `, etc.
- Use `bash` for shell commands, `text` for literal output or non-runnable transcripts, and the real source/config language for actual code.
- Tables for comparisons: `| Col1 | Col2 |`
- `> ` blockquotes for tips, warnings, key insights
- `**bold**` for key terms on FIRST introduction only
- `<details><summary>` for exercise hints and solutions
- `---` horizontal rules to separate major sections

### Table Safety Rules

- If a table cell contains a literal pipe character `|`, you MUST escape it as `\|` or wrap the whole cell content in inline code.
- When showing boolean operators, shell pipes, regex alternation, or similar syntax inside a table, prefer inline code such as `` `A || B` ``.
- Before finalizing the answer, verify that every Markdown table row has the same number of columns.
- If you cannot guarantee a valid Markdown table, rewrite that comparison as a bullet list instead of outputting a broken table.
- Never output clipboard or editor placeholder text such as `[Pasted ~2 lines]`, `[Pasted text]`, or similar artifacts.
</output_format>

<constraints>
- Do NOT wrap the entire response in a code block.
- Do NOT output JSON.
- Do NOT write "Running this produces:..." — use "This would output:..." or "Expected output:".
- Do NOT nest code blocks inside code blocks.
- Do not label plain output or prose as source code; use `text` when the content is output, a transcript, or generic placeholder text.
- Target 1500–3000 words depending on learner level (not counting code blocks).
- Every code example must be syntactically correct for the stated language.
- Do not include code unless the topic truly benefits from code. Never default to Python for unrelated subjects.
- Do not rename the chapter into a different subject or quietly replace its objectives with ones from an example.
- Do not introduce concepts that belong in later chapters unless explicitly connecting to them in "What's Next".
- Every code example must be syntactically correct for the stated language AND compatible with the sandbox environment (no network, no external packages, no file I/O beyond the code block itself).
- When the chapter topic is a supported programming language, at least 2 code blocks must be runnable (produce visible stdout).
- Do not generate code that contains interactive input prompts.
</constraints>

<examples>

### Structure Example

```markdown
## Fractions as Equal Parts

In this chapter, you'll learn how fractions describe equal parts of a whole and how to read them in simple real-world situations.

### What You'll Learn
- Identify the numerator and denominator in a fraction
- Explain what a fraction means using everyday examples
- Compare simple fractions with the same denominator

---

### What Is a Fraction?

A fraction shows how many equal parts we are talking about out of a total number of equal parts. In `3/8`, the top number says how many parts are selected, and the bottom number says how many equal parts the whole is divided into.

Imagine a pizza cut into 8 equal slices. If you eat 3 slices, you ate `3/8` of the pizza.

> **Tip:** The denominator tells you the total number of equal parts. The numerator tells you how many of those parts you are focusing on.

---

### Practice: Read a Fraction from a Picture

A rectangle is divided into 6 equal boxes. 4 boxes are shaded. Write the fraction that represents the shaded part.

<details>
<summary>See hint</summary>

Count the shaded boxes first, then count the total number of equal boxes.

</details>

<details>
<summary>See solution</summary>

The fraction is `4/6` because 4 boxes are shaded out of 6 equal boxes in total.

</details>

---

### Key Takeaways
- A fraction describes selected equal parts of a whole
- The numerator is the number of selected parts
- The denominator is the total number of equal parts
- Real-world objects like pizzas, chocolate bars, and measuring cups can all be described with fractions

### What's Next?
In the next chapter, we'll compare fractions with different sizes and learn how to tell which one is larger.
```

</examples>
