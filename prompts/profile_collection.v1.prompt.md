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
Analyze the learner's answer to infer their profile. When the answer is vague, incomplete, or contradictory, choose the closest reasonable default and note the uncertainty in your reasoning.

Apply these mapping rules:

**Experience Level Mapping:**
| User Signal | domain_knowledge |
|---|---|
| "nothing", "no experience", "starting from scratch", "complete beginner" | `none` |
| "basics", "basic understanding", "familiar with", "some exposure", "dabbled" | `beginner` |
| "comfortable", "practical experience", "use it at work", "intermediate" | `intermediate` |
| "advanced", "expert", "professional", "years of experience" | `advanced` |

**Pace Preference Mapping:**
| User Signal | pace_preference |
|---|---|
| "take my time", "thorough", "deep understanding" | `slow_thorough` |
| "balanced", "moderate", default when unclear | `moderate` |
| "fast", "quick", "efficient", "accelerated" | `fast_paced` |
| "self-paced", "my own schedule", "flexible" | `self_directed` |

**Default Values (use when the answer does not specify):**
- `preferred_format`: `["text", "exercise-based"]`
- `pace_preference`: `"moderate"`
- `hours_per_week`: `5`
- `language`: `"en"`
- `difficulty_bias`: `"standard"`
- `feedback_frequency`: `"end_of_section"`
</instructions>

<output_format>
When `is_final` is true, return ONLY a JSON object (no markdown fences, no explanation) with exactly these fields:

```json
{
  "experience_level": {
    "domain_knowledge": "<one of: none, beginner, intermediate, advanced>"
  },
  "learning_style": {
    "preferred_format": ["<one or more of: text, visual, interactive, audio, exercise-based, project-based>"],
    "pace_preference": "<one of: slow_thorough, moderate, fast_paced, self_directed>"
  },
  "available_time": {
    "hours_per_week": <number between 0.5 and 80>
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
- All four top-level keys are REQUIRED: experience_level, learning_style, available_time, preferences.
- Do NOT add any fields beyond those shown above.
- Return ONLY the raw JSON object — no ```json fences, no surrounding text, no explanation.
- If the user's answer is ambiguous, apply the closest mapping and use default values for unspecified fields.
</constraints>

<examples>

### Example 1: Beginner with clear signal
**Input:** learning_goal="Learn Python", answer="I have no programming experience but I use Excel a lot"

**Output:**
```json
{
  "experience_level": {
    "domain_knowledge": "none"
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

### Example 2: Intermediate learner with preferences
**Input:** learning_goal="Learn Rust systems programming", answer="I've been writing Python for 3 years professionally. I learn fast and prefer hands-on projects. I can dedicate about 10 hours a week."

**Output:**
```json
{
  "experience_level": {
    "domain_knowledge": "intermediate"
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
