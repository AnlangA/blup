<task>
Evaluate whether a learning goal is feasible to teach on this platform.
</task>

<input>
- **learning_goal**: `{{learning_goal}}`
- **domain**: `{{domain}}`
- **context**: `{{context}}`
</input>

<instructions>
Think through this evaluation step by step:

**Step 1 — Scope Assessment**
Is the goal well-defined? Evaluate whether it is:
- **Too broad**: e.g., "learn physics", "learn programming", "study math". These need narrowing to a specific sub-domain or project.
- **Too narrow**: e.g., "derive equation 3.7 from chapter 4", "what is the syntax for one operator". These do not warrant a full curriculum.
- **Well-scoped**: Covers a specific skill or knowledge area that can be decomposed into 3–20 learning chapters.

**Step 2 — Teachability Check**
Can this goal be taught effectively through:
- Structured text explanations?
- Code examples and exercises?
- Step-by-step problem solving?
If the goal requires physical practice, hands-on lab work, or in-person supervision, it is partially or fully infeasible for this platform.

**Step 3 — Depth & Duration Estimation**
- Can the goal be broken into chapters, each completable in 15–60 minutes?
- Is the estimated total duration realistic? (A complex domain should take weeks, not "1 day".)
- Are the prerequisites concrete and specific? ("basic algebra" is good; "some math" is not.)

**Step 4 — Constructive Guidance**
If the goal is infeasible, you MUST provide specific, actionable suggestions for refining it. Never simply reject — always guide the learner toward a feasible version they would find valuable.
</instructions>

<output_format>
Return a single JSON object with this exact structure (no markdown fences, no extra text):

```json
{
  "feasible": true | false,
  "reason": "string — clear explanation understandable to the learner",
  "suggestions": ["string — specific, actionable refinements (empty array if feasible)"],
  "estimated_duration": "string — e.g., '3-4 weeks (5 hrs/week)' or 'N/A — refine goal first'",
  "prerequisites": ["string — specific prerequisites (empty array if none)"]
}
```

Schema reference: `schemas/feasibility_result.v1.schema.json`
</output_format>

<constraints>
- The `reason` field must be written for the learner, not as an internal diagnostic.
- `estimated_duration` must be realistic for the goal's complexity.
- Each `suggestion` must be a concrete action the learner can take, not vague advice.
- `prerequisites` must use specific skill names, not generic phrases.
- If the goal is ambiguous, include clarifying questions in `suggestions`.
</constraints>

<examples>

### Positive Example: Well-scoped goal

**Input:** learning_goal="I want to learn Python for data analysis with pandas", domain="programming", context="I work with Excel spreadsheets daily"

**Output:**
```json
{
  "feasible": true,
  "reason": "Python data analysis with pandas is a well-scoped goal with clear learning milestones. Your Excel experience provides a practical bridge for understanding tabular data operations.",
  "suggestions": [],
  "estimated_duration": "4-6 weeks (5 hrs/week)",
  "prerequisites": ["basic computer literacy", "understanding of tabular data (rows, columns, cells)"]
}
```

### Negative Example: Too broad

**Input:** learning_goal="I want to learn programming", domain="programming", context=""

**Output:**
```json
{
  "feasible": false,
  "reason": "\"Learn programming\" spans dozens of languages, paradigms, and application domains — too broad for a focused curriculum.",
  "suggestions": [
    "Pick a specific language: Python (recommended for beginners), JavaScript (for web), or SQL (for data)",
    "Choose an application: web development, data analysis, automation, or game development",
    "Example refined goal: \"Learn Python to automate repetitive spreadsheet tasks\""
  ],
  "estimated_duration": "N/A — refine goal first",
  "prerequisites": []
}
```

### Edge Case Example: Partially feasible

**Input:** learning_goal="I want to learn to play the piano", domain="music", context=""

**Output:**
```json
{
  "feasible": false,
  "reason": "Playing piano requires physical practice and real-time feedback on hand position and technique that this platform cannot provide.",
  "suggestions": [
    "Focus on music theory: \"Learn music theory fundamentals for keyboard instruments\"",
    "Focus on reading music: \"Learn to read sheet music and understand rhythm notation\"",
    "Combine with external practice: use this platform for theory and a local teacher for physical technique"
  ],
  "estimated_duration": "N/A — goal requires physical practice",
  "prerequisites": []
}
```

</examples>
