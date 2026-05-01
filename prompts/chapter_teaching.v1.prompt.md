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
- Provide a concrete code example or analogy immediately after each concept
- Use `> ` blockquotes for tips and warnings
- Keep paragraphs to 3–5 sentences max

### 5. Practice Exercises (embed 2–3 throughout, not all at the end)
Place exercises after the section that teaches the relevant concept. Each exercise:
- States a clear task
- Includes a `<details><summary>See hint</summary>` hint
- Includes a `<details><summary>See solution</summary>` solution with explanation

### 6. Key Takeaways (bullet list)
3–5 bullets summarizing the chapter's main concepts. Each bullet should be a complete, self-contained statement.

### 7. What's Next (1–2 sentences)
Brief preview of the next chapter, creating continuity and motivation.

## Differentiation by Learner Level

**For beginners (domain_knowledge: none or beginner):**
- Use more analogies connecting to everyday experience
- Break complex operations into numbered step sequences
- Include more inline exercises (every 2–3 concepts)
- Provide complete, runnable code examples — no partial snippets
- Target word count: 2000–3000 words

**For intermediate learners (domain_knowledge: intermediate):**
- Move faster through basics, spend more time on patterns and best practices
- Use partial code examples with "fill in the rest" exercises
- Reference prior knowledge explicitly ("You already know X, so Y works similarly")
- Target word count: 1500–2500 words

**For advanced learners (domain_knowledge: advanced):**
- Focus on edge cases, performance implications, and design trade-offs
- Use shorter explanations with deeper technical references
- Include challenge exercises that require combining multiple concepts
- Target word count: 1500–2000 words
</instructions>

<output_format>
Plain Markdown content. NOT JSON. NOT wrapped in a code block.

Formatting rules:
- `##` for chapter title, `###` for major sections, `####` for subsections
- Code blocks MUST specify language: ` ```python ... ``` `
- Tables for comparisons: `| Col1 | Col2 |`
- `> ` blockquotes for tips, warnings, key insights
- `**bold**` for key terms on FIRST introduction only
- `<details><summary>` for exercise hints and solutions
- `---` horizontal rules to separate major sections
</output_format>

<constraints>
- Do NOT wrap the entire response in a code block.
- Do NOT output JSON.
- Do NOT write "Running this produces:..." — use "This would output:..." or "Expected output:".
- Do NOT nest code blocks inside code blocks.
- Target 1500–3000 words depending on learner level (not counting code blocks).
- Every code example must be syntactically correct for the stated language.
- Do not introduce concepts that belong in later chapters unless explicitly connecting to them in "What's Next".
</constraints>

<examples>

### Structure Example

```markdown
## Variables and Data Types

In this chapter, you'll learn how to store and work with different kinds of data in Python — the foundation for everything that follows.

### What You'll Learn
- Create and use variables in Python
- Distinguish the main data types: strings, numbers, and booleans
- Convert between types using built-in functions

---

### What Are Variables?

Think of a variable as a **labeled box** that holds a value. You give the box a name, and put something inside it.

```python
name = "Alice"
age = 25
is_student = True
```

> **Tip:** Variable names should be descriptive. `age` is much better than `x` because it tells you what the value means.

### Naming Rules

| Valid | Invalid | Why |
|-------|---------|-----|
| `my_name` | `my-name` | Hyphens are not allowed |
| `count2` | `2count` | Cannot start with a number |
| `_private` | `my var` | Spaces are not allowed |

---

### Practice: Create Your Own Variables

Create variables for your name, age, and whether you're learning Python. Then print them.

<details>
<summary>See hint</summary>

Use `=` to assign a value. Use `print()` to display it. Remember: strings need quotes, numbers don't.

</details>

<details>
<summary>See solution</summary>

```python
name = "Your Name"
age = 20
learning_python = True

print(f"Name: {name}")
print(f"Age: {age}")
print(f"Learning Python: {learning_python}")
```

The `f"..."` syntax is called an **f-string** — it lets you embed variables directly inside a string using `{}`.

</details>

---

### Key Takeaways
- Variables store values with descriptive names
- Python has three basic data types: strings (text), numbers (int/float), and booleans (True/False)
- Use `type()` to check a variable's data type
- f-strings (`f"..."`) format values into text cleanly

### What's Next?
In the next chapter, we'll learn about **operators** — how to perform calculations and make comparisons with your variables.
```

</examples>
