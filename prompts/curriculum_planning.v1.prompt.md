<task>
Generate a personalized curriculum plan from the confirmed learning goal and learner profile.
</task>

<input>
- **learning_goal**: `{{learning_goal}}`
- **user_profile**: `{{user_profile}}`
</input>

<instructions>
Design the curriculum by thinking through these steps in order:

**Step 0 — Identify the Actual Subject Domain**
- Infer the real subject domain from the learner's goal itself, not from examples elsewhere in this prompt.
- Keep every chapter aligned to that domain and to the learner's stated outcome.
- Do NOT introduce Python, programming, notebooks, libraries, or code-first activities unless the goal explicitly requires them.
- For non-programming goals, use subject-appropriate chapter types such as concepts, worked examples, case studies, drills, critiques, labs, or practice scenarios.

**Step 1 — Analyze the Learner**
Examine the user profile to determine:
- Starting point: What does the learner already know? (domain_knowledge level)
- Pace: How fast should content progress? (pace_preference + hours_per_week)
- Format: What content types to emphasize? (preferred_format). Use those preferences to bias the curriculum, but do not force a medium that does not fit the subject.
- Challenge: How difficult should exercises be? (difficulty_bias)

**Step 2 — Decompose the Goal**
Break the learning goal into a logical sequence of knowledge milestones. Each milestone should:
- Be a self-contained skill or concept that can be taught in one session
- Build directly on the milestones before it
- Map to one chapter in the curriculum
- Stay within the learner's stated subject area rather than drifting into unrelated tools or disciplines

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
- A unique `id` (kebab-case, descriptive: e.g., "linear-equations-basics", "spanish-greetings", "sql-joins")
- A clear, learner-facing `title`
- 3–5 specific, measurable `objectives` using action verbs that fit the subject (e.g., "Solve", "Explain", "Interpret", "Build", "Photograph", "Critique")
- `prerequisites` referencing chapter IDs that must come before
- Realistic `estimated_minutes` based on content density and learner level

**Step 5 — Review for Coherence**
Verify that:
- Every chapter's prerequisites are satisfied by earlier chapters
- No chapter introduces concepts not covered by itself or a prerequisite
- The progression feels natural — no sudden difficulty jumps
- The curriculum title and description accurately reflect the content
- No chapter title, objective, or prerequisite imports an unrelated domain, tool, or language
- `estimated_duration` roughly matches the chapter count, chapter lengths, and stated `hours_per_week`
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
- Objectives must use concrete action verbs ("Solve", "Build", "Explain", "Interpret", "Identify", "Critique"), not vague ones ("Understand", "Know", "Learn about").
- Keep the curriculum in the same subject domain as the learning goal. Do not insert programming languages, Python libraries, notebooks, or coding tasks unless they are explicitly part of the goal or a clearly necessary prerequisite named by the goal.
- `estimated_duration` should be plausible given the number of chapters, their `estimated_minutes`, and the learner's available time.
</constraints>

<examples>

### Example: Beginner Product Photography for Online Shops

**Profile:** experience=beginner, pace=moderate, time=5hrs/week, format=visual + practice-based
**Goal:** "Learn product photography for my online store"

**Output:**
```json
{
  "title": "Product Photography for Online Sellers",
  "description": "A practical curriculum that helps you create clear, consistent product photos for listings using simple lighting, composition, and editing habits.",
  "chapters": [
    {
      "id": "camera-phone-setup",
      "title": "Set Up a Reliable Shooting Space",
      "order": 1,
      "objectives": [
        "Choose a stable background and shooting surface",
        "Position lights to reduce harsh shadows",
        "Prepare a repeatable phone or camera setup"
      ],
      "prerequisites": [],
      "estimated_minutes": 45
    },
    {
      "id": "composition-and-angles",
      "title": "Frame Products Clearly and Consistently",
      "order": 2,
      "objectives": [
        "Compose centered and detail-focused product shots",
        "Choose angles that communicate size and features",
        "Create a shot checklist for consistent listings"
      ],
      "prerequisites": ["camera-phone-setup"],
      "estimated_minutes": 50
    },
    {
      "id": "editing-for-listings",
      "title": "Edit Photos for Clean, Trustworthy Listings",
      "order": 3,
      "objectives": [
        "Crop and straighten product photos",
        "Adjust exposure and white balance for consistency",
        "Export web-ready images for product pages"
      ],
      "prerequisites": ["composition-and-angles"],
      "estimated_minutes": 40
    }
  ],
  "estimated_duration": "4-6 weeks (5 hrs/week)",
  "learning_objectives": [
    "Create a repeatable low-cost product photo setup",
    "Capture clear images that show product details honestly",
    "Edit photos into a consistent listing-ready style",
    "Build a simple workflow for photographing new inventory"
  ]
}
```

Use the same structure for programming goals, but only mention the specific language, library, or tool that the learner actually asked to study.

</examples>
