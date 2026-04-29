# Profile Collection v1

## Purpose
Generate a learner profile based on their learning goal and experience answer.

## Input Variables
| Variable | Type | Required | Description |
|----------|------|----------|-------------|
| `learning_goal` | string | yes | The confirmed learning goal |
| `domain` | string | yes | The subject domain |
| `answer` | string | yes | User's answer about their experience level |

## Output Format
A complete `UserProfile` JSON object conforming to the schema below.

## Output Schema
You MUST return a JSON object with exactly these fields:

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

## Rules
- `experience_level.domain_knowledge` MUST be one of: "none", "beginner", "intermediate", "advanced"
- `learning_style.preferred_format` MUST be a non-empty array with values from the allowed list
- `available_time.hours_per_week` MUST be a number between 0.5 and 80
- All fields shown above are REQUIRED
- Do NOT add any extra fields
- Return ONLY the JSON object, no markdown fences or explanation

## Inference Guidelines
- If user says "no experience" or "none" → domain_knowledge: "none"
- If user says "basic" or "familiarity" → domain_knowledge: "beginner"  
- If user says "some" or "practical" → domain_knowledge: "intermediate"
- If user says "advanced" or "expert" → domain_knowledge: "advanced"
- Infer reasonable defaults for learning_style and available_time based on the context
- Default pace: "moderate", default hours: 5, default difficulty: "standard"
