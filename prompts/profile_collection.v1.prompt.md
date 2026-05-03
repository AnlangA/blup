<task>
Generate a complete learner profile from the user's answers about their experience and preferences.
</task>

<input>
- **learning_goal**: `{{learning_goal}}`
- **domain**: `{{domain}}`
- **answer**: `{{answer}}`
- **round**: `{{round}}`
- **is_final**: `{{is_final}}`
</input>

<instructions>
Infer the learner's profile from the current answer plus the target `learning_goal` and `domain`. The current answer may cover only one part of the profile, so infer conservatively and use defaults for anything that is missing, vague, or only weakly implied.

**Goal and Domain Alignment**
- Assess `experience_level.domain_knowledge` relative to the target learning goal, not just general adjacent experience.
- When the learner has experience in a related but different domain, keep `domain_knowledge` conservative and, if supported by the schema, preserve the transfer signal with optional fields such as `related_domains` or `years_of_experience`.
- Do not invent optional fields like `goals`, `success_criteria`, `timezone`, or `notes` unless the answer clearly provides them.

Apply these mapping rules:

**Experience Level Mapping:**
| User Signal | domain_knowledge |
|---|---|
| "nothing", "no experience", "starting from scratch", "complete beginner" | `none` |
| "basics", "basic understanding", "familiar with", "some exposure", "dabbled" | `beginner` |
| "comfortable", "practical experience", "use it at work", "intermediate" | `intermediate` |
| "advanced", "expert", "professional", "years of experience" | `advanced` |

**Preferred Format Mapping:**
| User Signal | preferred_format |
|---|---|
| "reading", "docs", "text", "written explanations" | `text` |
| "visual", "diagrams", "charts", "see it" | `visual` |
| "interactive", "quiz me", "ask me questions" | `interactive` |
| "audio", "listen", "podcast" | `audio` |
| "practice", "hands-on", "drills", "try it myself" | `exercise-based` |
| "projects", "build something", "portfolio" | `project-based` |

**Pace Preference Mapping:**
| User Signal | pace_preference |
|---|---|
| "take my time", "thorough", "deep understanding" | `slow_thorough` |
| "balanced", "moderate", default when unclear | `moderate` |
| "fast", "quick", "efficient", "accelerated" | `fast_paced` |
| "self-paced", "my own schedule", "flexible" | `self_directed` |

**Difficulty Bias Mapping:**
| User Signal | difficulty_bias |
|---|---|
| "gentle", "easy", "build confidence", "not too hard" | `easier` |
| no clear preference | `standard` |
| "push me", "challenging", "stretch", "rigorous" | `challenging` |

**Feedback Frequency Mapping:**
| User Signal | feedback_frequency |
|---|---|
| "correct me right away", "immediate feedback" | `immediate` |
| no clear preference | `end_of_section` |
| "summaries", "review at the end", "end of chapter" | `end_of_chapter` |

**Time Extraction Rules:**
- If the learner gives a single weekly estimate, use it directly.
- If the learner gives a weekly range, choose a reasonable midpoint.
- If the learner gives a daily cadence, convert it to an approximate weekly total.
- If the learner gives session length but not weekly time, keep the default `hours_per_week` unless a clear weekly total can be inferred.

**Default Values (use when the answer does not specify):**
- `preferred_format`: `["text", "exercise-based"]`
- `pace_preference`: `"moderate"`
- `hours_per_week`: `5`
- `language`: `"en"`
- `difficulty_bias`: `"standard"`
- `feedback_frequency`: `"end_of_section"`
</instructions>

<output_format>
When `is_final` is true, return ONLY a JSON object (no markdown fences, no explanation) using only fields from `schemas/user_profile.v1.schema.json`.

Include the required fields below. Optional fields may be included only when they are explicitly stated or strongly implied:

```json
{
  "experience_level": {
    "domain_knowledge": "<one of: none, beginner, intermediate, advanced>",
    "related_domains": ["<optional related domain>"],
    "years_of_experience": 0
  },
  "learning_style": {
    "preferred_format": ["<one or more of: text, visual, interactive, audio, exercise-based, project-based>"],
    "pace_preference": "<one of: slow_thorough, moderate, fast_paced, self_directed>",
    "notes": "<optional short note>"
  },
  "available_time": {
    "hours_per_week": <number between 0.5 and 80>,
    "preferred_session_length_minutes": 45,
    "timezone": "<optional timezone>"
  },
  "goals": {
    "primary_goal": "<optional string>",
    "secondary_goals": ["<optional string>"],
    "success_criteria": "<optional string>"
  },
  "preferences": {
    "language": "<language code or name>",
    "difficulty_bias": "<one of: easier, standard, challenging>",
    "feedback_frequency": "<one of: immediate, end_of_section, end_of_chapter>"
  }
}
```

Schema reference: `schemas/user_profile.v1.schema.json`
</output_format>

<constraints>
- `experience_level.domain_knowledge` MUST be exactly one of: "none", "beginner", "intermediate", "advanced". No other values.
- `learning_style.preferred_format` MUST be a non-empty array. Each value MUST be from the allowed list.
- `available_time.hours_per_week` MUST be a number between 0.5 and 80.
- The top-level keys `experience_level`, `learning_style`, `available_time`, and `preferences` MUST always be present.
- Do NOT add any fields that are not allowed by `schemas/user_profile.v1.schema.json`.
- Return ONLY the raw JSON object — no ```json fences, no surrounding text, no explanation.
- If the user's answer is ambiguous, apply the closest mapping and use default values for unspecified fields.
- Do not set `domain_knowledge` above `beginner` solely because the learner has adjacent but different-domain experience. Use optional transfer fields instead when supported by the answer.
- If you include any optional field, it must be grounded in the answer rather than guessed.
</constraints>

<examples>

### Example 1: Beginner with clear signal
**Input:** learning_goal="Learn bookkeeping basics", answer="I've never studied accounting before. Visual explanations help me, and I can spend about 3 hours a week."

**Output:**
```json
{
  "experience_level": {
    "domain_knowledge": "none"
  },
  "learning_style": {
    "preferred_format": ["visual", "text"],
    "pace_preference": "moderate"
  },
  "available_time": {
    "hours_per_week": 3
  },
  "preferences": {
    "language": "en",
    "difficulty_bias": "standard",
    "feedback_frequency": "end_of_section"
  }
}
```

### Example 2: Adjacent experience, new target domain
**Input:** learning_goal="Learn Rust systems programming", answer="I've been writing Python for 3 years professionally. I learn fast and prefer hands-on projects. I can dedicate about 10 hours a week."

**Output:**
```json
{
  "experience_level": {
    "domain_knowledge": "beginner",
    "related_domains": ["Python"],
    "years_of_experience": 3
  },
  "learning_style": {
    "preferred_format": ["project-based", "exercise-based"],
    "pace_preference": "fast_paced"
  },
  "available_time": {
    "hours_per_week": 10
  },
  "preferences": {
    "language": "en",
    "difficulty_bias": "challenging",
    "feedback_frequency": "end_of_section"
  }
}
```

### Example 3: Vague answer with defaults applied
**Input:** learning_goal="Learn machine learning", answer="I'm not sure, I've read some articles"

**Output:**
```json
{
  "experience_level": {
    "domain_knowledge": "beginner"
  },
  "learning_style": {
    "preferred_format": ["text", "exercise-based"],
    "pace_preference": "moderate"
  },
  "available_time": {
    "hours_per_week": 5
  },
  "preferences": {
    "language": "en",
    "difficulty_bias": "standard",
    "feedback_frequency": "end_of_section"
  }
}
```

</examples>
