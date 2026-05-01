//! Typed domain structs mirroring the JSON Schema definitions.
//!
//! These replace `serde_json::Value` for session fields so the compiler
//! catches key typos and shape mismatches at build time instead of runtime.

use serde::{Deserialize, Serialize};

// ── LearningGoal (learning_goal.v1.schema.json) ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningGoal {
    pub description: String,
    pub domain: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_level: Option<String>,
}

// ── FeasibilityResult (feasibility_result.v1.schema.json) ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeasibilityResult {
    pub feasible: bool,
    pub reason: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub suggestions: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub estimated_duration: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub prerequisites: Vec<String>,
}

// ── UserProfile (user_profile.v1.schema.json) ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserProfile {
    pub experience_level: ExperienceLevel,
    pub learning_style: LearningStyle,
    pub available_time: AvailableTime,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub goals: Option<ProfileGoals>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preferences: Option<Preferences>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperienceLevel {
    pub domain_knowledge: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub related_domains: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub years_of_experience: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningStyle {
    pub preferred_format: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pace_preference: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AvailableTime {
    pub hours_per_week: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preferred_session_length_minutes: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timezone: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileGoals {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub primary_goal: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub secondary_goals: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub success_criteria: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Preferences {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub difficulty_bias: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub feedback_frequency: Option<String>,
}

// ── CurriculumPlan (curriculum_plan.v1.schema.json) ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurriculumPlan {
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub chapters: Vec<ChapterData>,
    pub estimated_duration: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub prerequisites_summary: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub learning_objectives: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChapterData {
    pub id: String,
    pub title: String,
    pub order: u32,
    pub objectives: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub prerequisites: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub estimated_minutes: Option<u32>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub key_concepts: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub exercises: Vec<Exercise>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Exercise {
    pub question: String,
    #[serde(rename = "type")]
    pub exercise_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub difficulty: Option<String>,
}

// ── SessionMessage (message.v1.schema.json + chapter_id extension) ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMessage {
    pub id: String,
    pub role: String,
    pub content: String,
    pub timestamp: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chapter_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

// ── ChapterProgress (chapter_progress.v1.schema.json) ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChapterProgress {
    pub chapter_id: String,
    pub status: String,
    pub completion: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub time_spent_minutes: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exercises_completed: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exercises_total: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_accessed: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub difficulty_rating: Option<u32>,
}
