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

**Step 0 — Ground the Goal**
- Identify the actual subject area, intended outcome, and any explicit constraints from `learning_goal` and `context`.
- Keep the assessment and suggestions inside that subject area. Do not drift to unrelated domains or default to software topics unless the learner is actually asking about software.

**Step 1 — Scope Assessment**
Is the goal well-defined? Evaluate whether it is:
- **Too broad**: e.g., "learn physics", "learn programming", "study math". These need narrowing to a specific sub-domain or project.
- **Too narrow**: e.g., "derive equation 3.7 from chapter 4", "what is the syntax for one operator". These do not warrant a full curriculum.
- **Well-scoped**: Covers a specific skill or knowledge area that can be decomposed into 3–20 learning chapters.

**Step 2 — Teachability Check**
Can this goal be taught effectively through:
- Structured text explanations?
- Worked examples, diagrams, formulas, critiques, or scenario-based practice?
- Code examples and exercises, but only if the goal itself involves programming or executable tools?
- Step-by-step problem solving or reflective exercises?
If the goal requires physical practice, hands-on lab work, or in-person supervision, decide whether a meaningful theory/planning/analysis subset is still teachable here and explain that boundary explicitly.

**Step 3 — Depth & Duration Estimation**
- Can the goal be broken into chapters, each completable in 15–60 minutes?
- Is the estimated total duration realistic? (A complex domain should take weeks, not "1 day".)
- Are the prerequisites concrete and specific? ("basic algebra" is good; "some math" is not.)

**Step 4 — Constructive Guidance**
If the goal is infeasible, you MUST provide specific, actionable suggestions for refining it. Never simply reject — always guide the learner toward a feasible version they would find valuable.
- Keep suggestions aligned with the learner's original domain. Do not pivot them to coding, Python, or developer tools unless they already asked for that direction.
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
- Keep the `reason`, `suggestions`, and `prerequisites` in the same subject domain as the learner's goal unless the learner explicitly asks to pivot domains.
- If a goal is only partially feasible, explain which portion is teachable on this platform and which portion requires external practice, tools, or supervision.
- Do not default to programming languages or coding projects unless the goal is actually about them.
</constraints>

<examples>

### Positive Example: Well-scoped goal

**Input:** learning_goal="I want to learn introductory statistics for psychology research", domain="statistics", context="I remember basic algebra but haven't used it in years"

**Output:**
```json
{
  "feasible": true,
  "reason": "Introductory statistics for psychology research is a focused goal with clear milestones such as descriptive statistics, probability basics, hypothesis testing, and interpretation of results. Your prior algebra exposure provides a workable starting point, even if it feels rusty.",
  "suggestions": [],
  "estimated_duration": "5-7 weeks (5 hrs/week)",
  "prerequisites": ["basic algebra with percentages and simple equations", "comfort reading tables and graphs"]
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
    "Pick one concrete outcome, such as building a small website, analyzing spreadsheet exports, or automating a repetitive task",
    "Choose one language or stack that fits that outcome instead of trying to cover all of programming at once",
    "Example refined goal: \"Learn SQL to answer business questions from spreadsheet exports\""
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
