# Curriculum Planning v1

## Purpose
Generate a personalized curriculum plan from the learning goal and user profile.

## Input Variables
| Variable | Type | Required | Description |
|----------|------|----------|-------------|
| `learning_goal` | string | yes | The confirmed goal |
| `feasibility_result` | object | yes | Feasibility assessment with prerequisites and suggestions |
| `user_profile` | object | yes | Complete learner profile |

## Output Format
Must produce valid JSON conforming to `schemas/curriculum_plan.v1.schema.json`.

## Safety and Privacy Rules
- Never fabricate computation results, citations, or execution output.
- Never include API keys, credentials, or private paths in output.
- Mark uncertainty explicitly.

## Instructions

Design a curriculum as a series of chapters. Each chapter should:
1. Build on previous chapters (clear prerequisites).
2. Have specific, measurable learning objectives.
3. Be completable in one learning session (15-60 minutes).
4. Balance theory, examples, and practice.

### Chapter Count Guidelines
- Very focused goal → 3-5 chapters
- Moderate scope → 6-10 chapters
- Broad goal → 11-20 chapters

### Personalization Based on Profile
- `experience_level: beginner` → more foundational chapters, slower progression
- `pace_preference: fast_paced` → more content per chapter, fewer introductory chapters
- `preferred_format: exercise-based` → more inline exercises per chapter
- `available_time: low` → shorter chapters (~15 min each)

## Examples

### Example 1: Beginner Python for Data
**Profile:** experience=beginner, pace=moderate, time=5hrs/week, style=exercise-based
**Goal:** "Learn Python for data analysis with pandas"

**Output:**
```json
{
  "title": "Python for Data Analysis: From Excel to pandas",
  "description": "A hands-on curriculum that bridges Excel knowledge to Python data analysis.",
  "chapters": [
    {
      "id": "python-basics",
      "title": "Python Basics for Data Work",
      "order": 1,
      "objectives": ["Install Python and Jupyter", "Understand variables and data types", "Write basic expressions"],
      "prerequisites": [],
      "estimated_minutes": 45
    },
    {
      "id": "data-structures",
      "title": "Lists, Dictionaries, and DataFrames",
      "order": 2,
      "objectives": ["Work with Python lists and dicts", "Create your first pandas DataFrame"],
      "prerequisites": ["python-basics"],
      "estimated_minutes": 50
    }
  ],
  "estimated_duration": "4-6 weeks (5 hrs/week)",
  "learning_objectives": [
    "Read CSV and Excel files into Python",
    "Clean and filter data with pandas",
    "Create summary statistics",
    "Build basic data visualizations"
  ]
}
```
