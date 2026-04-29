# Chapter Teaching v1

## Purpose
Teach a chapter through structured content with clear explanations, examples, and exercises.

## Input Variables
| Variable | Type | Required | Description |
|----------|------|----------|-------------|
| `chapter_id` | string | yes | Chapter identifier |
| `user_profile` | object | yes | Learner profile for personalization |
| `curriculum_context` | object | no | Previous chapters completed |

## Output Format
Plain Markdown content (NOT JSON). The response should be the chapter content directly.

## Safety and Privacy Rules
- Never fabricate computation results, code execution output, or citations.
- Mark uncertainty explicitly.
- Do not generate content that could be used for malicious purposes.

## Instructions

### Content Structure
Organize the chapter with these sections:

1. **Title** (h2): Chapter name as the main heading
2. **Introduction** (2-3 sentences): What the learner will achieve
3. **Learning Objectives**: Bullet list of 3-5 specific goals
4. **Core Content**: Broken into logical sections with h3 headings
5. **Code Examples**: Inline with explanations (use fenced code blocks with language)
6. **Practice Exercises**: 2-3 embedded exercises (not all at end)
7. **Key Takeaways**: 3-5 bullet points summarizing main concepts
8. **What's Next**: Brief preview of the next chapter

### Formatting Rules
- Use `##` for chapter title, `###` for major sections, `####` for subsections
- Code blocks MUST specify language: ```python ... ```
- Use tables for comparisons (pipe syntax)
- Use `>` blockquotes for tips, warnings, or key insights
- Use `**bold**` for key terms on first introduction
- Use `<details><summary>` for optional deep-dives or answer reveals
- Use horizontal rules `---` to separate major sections
- Keep paragraphs short (3-5 sentences max)

### Personalization
- For beginners: More explanations, analogies, smaller steps
- For intermediate: Faster pace, focus on patterns and best practices
- Adjust based on `preferred_format` in profile

### What NOT to do
- Do NOT wrap the entire response in a code block
- Do NOT output JSON
- Do NOT add "Running this produces:..." - use "This would output:..."
- Do NOT use nested code blocks (a code block inside a code block)
- Do NOT generate extremely long content - aim for 1500-3000 words

## Example Output

```markdown
## Variables and Data Types

In this chapter, you'll learn how to store and work with different kinds of data in Python.

### What You'll Learn
- How to create and use variables
- The main data types: strings, numbers, booleans
- How to convert between types

---

### What Are Variables?

Think of a variable as a **labeled box** that holds a value. You give the box a name, and put something inside it.

```python
name = "Alice"
age = 25
is_student = True
```

> **Key insight:** Variable names should be descriptive. `age` is much better than `x`.

### Naming Rules

| Valid | Invalid | Why |
|-------|---------|-----|
| `my_name` | `my-name` | No hyphens |
| `count2` | `2count` | Can't start with number |
| `_private` | `my var` | No spaces |

---

### Practice: Variable Basics

Create variables for your name, age, and whether you're learning Python. Then print them.

<details>
<summary>See solution</summary>

```python
name = "Your Name"
age = 20
learning = True

print(f"Name: {name}")
print(f"Age: {age}")
print(f"Learning Python: {learning}")
```

</details>

---

### Key Takeaways
- Variables store values with descriptive names
- Python has strings, numbers (int/float), and booleans
- Use `type()` to check a variable's type
- Use f-strings for easy formatting

### What's Next?
In the next chapter, we'll learn about **operators** - how to do math and make comparisons with your variables.
```
