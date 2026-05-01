<task>
Generate a personalized curriculum plan from the confirmed learning goal and learner profile.
</task>

<input>
- **learning_goal**: `{{learning_goal}}`
- **user_profile**: `{{user_profile}}`
</input>

<instructions>
Design the curriculum by thinking through these steps in order:

**Step 1 — Analyze the Learner**
Examine the user profile to determine:
- Starting point: What does the learner already know? (domain_knowledge level)
- Pace: How fast should content progress? (pace_preference + hours_per_week)
- Format: What content types to emphasize? (preferred_format)
- Challenge: How difficult should exercises be? (difficulty_bias)

**Step 2 — Decompose the Goal**
Break the learning goal into a logical sequence of knowledge milestones. Each milestone should:
- Be a self-contained skill or concept that can be taught in one session
- Build directly on the milestones before it
- Map to one chapter in the curriculum

**Step 3 — Determine Chapter Count**
Based on goal scope and learner profile:
- Focused goal (e.g., "learn one library"): 3–5 chapters
- Moderate scope (e.g., "learn a framework"): 6–10 chapters
- Broad goal (e.g., "learn a language"): 11–20 chapters

Adjust for experience level:
- `beginner`: Add 1–3 foundational chapters, keep progression gentle
- `intermediate`: Skip basics, start at conceptual depth
- `advanced`: Focus on patterns, best practices, and edge cases

Adjust for time constraints:
- Low hours_per_week (< 3): Shorter chapters (~15 min each)
- Moderate (3–7): Standard chapters (~30 min each)
- High (> 7): Longer chapters with more exercises (~45 min each)

**Step 4 — Write Each Chapter**
For each chapter, define:
- A unique `id` (kebab-case, descriptive: e.g., "python-basics", "data-cleaning")
- A clear, learner-facing `title`
- 3–5 specific, measurable `objectives` (use action verbs: "Write X", "Explain Y", "Build Z")
- `prerequisites` referencing chapter IDs that must come before
- Realistic `estimated_minutes` based on content density and learner level

**Step 5 — Review for Coherence**
Verify that:
- Every chapter's prerequisites are satisfied by earlier chapters
- No chapter introduces concepts not covered by itself or a prerequisite
- The progression feels natural — no sudden difficulty jumps
- The curriculum title and description accurately reflect the content
</instructions>

<output_format>
Return ONLY a JSON object (no markdown fences, no extra text):

```json
{
  "title": "string — descriptive curriculum title that appeals to the learner",
  "description": "string — 1-2 sentence overview of what the learner will achieve",
  "chapters": [
    {
      "id": "string — unique kebab-case identifier",
      "title": "string — clear chapter title",
      "order": 1,
      "objectives": ["string — specific, measurable learning objective"],
      "prerequisites": ["string — chapter IDs that must be completed first"],
      "estimated_minutes": 30
    }
  ],
  "estimated_duration": "string — e.g., '4-6 weeks (5 hrs/week)'",
  "learning_objectives": ["string — top-level outcomes the learner will achieve"]
}
```

Schema reference: `schemas/curriculum_plan.v1.schema.json`
</output_format>

<constraints>
- Every chapter `id` must be unique across the curriculum.
- Every entry in a chapter's `prerequisites` must reference an existing chapter `id` with a lower `order`.
- Chapter `order` values must be sequential starting from 1 with no gaps.
- `estimated_minutes` should be between 15 and 60 for any single chapter.
- `learning_objectives` should be 3–6 high-level outcomes, each distinct and measurable.
- `prerequisites` for the first chapter must be an empty array.
- Objectives must use concrete action verbs ("Write", "Build", "Explain", "Identify", "Debug"), not vague ones ("Understand", "Know", "Learn about").
</constraints>

<examples>

### Example: Beginner Python for Data Analysis

**Profile:** experience=beginner, pace=moderate, time=5hrs/week, format=exercise-based
**Goal:** "Learn Python for data analysis with pandas"

**Output:**
```json
{
  "title": "Python for Data Analysis: From Excel to pandas",
  "description": "A hands-on curriculum that takes you from zero Python experience to analyzing real datasets with pandas, leveraging your Excel background as a bridge.",
  "chapters": [
    {
      "id": "python-basics",
      "title": "Python Basics for Data Work",
      "order": 1,
      "objectives": [
        "Install Python and Jupyter Notebook",
        "Create variables and use basic data types (string, int, float, bool)",
        "Write arithmetic expressions and simple print statements"
      ],
      "prerequisites": [],
      "estimated_minutes": 45
    },
    {
      "id": "data-structures",
      "title": "Lists, Dictionaries, and DataFrames",
      "order": 2,
      "objectives": [
        "Create and manipulate Python lists and dictionaries",
        "Import pandas and create a DataFrame from a dictionary",
        "Select columns and rows from a DataFrame"
      ],
      "prerequisites": ["python-basics"],
      "estimated_minutes": 50
    },
    {
      "id": "reading-data",
      "title": "Reading and Inspecting Data Files",
      "order": 3,
      "objectives": [
        "Read CSV and Excel files into pandas DataFrames",
        "Use .head(), .info(), .describe() to inspect data",
        "Identify missing values and data type issues"
      ],
      "prerequisites": ["data-structures"],
      "estimated_minutes": 40
    }
  ],
  "estimated_duration": "4-6 weeks (5 hrs/week)",
  "learning_objectives": [
    "Read CSV and Excel files into Python using pandas",
    "Clean and filter messy datasets",
    "Compute summary statistics and group-by aggregations",
    "Build basic data visualizations with matplotlib"
  ]
}
```

</examples>
