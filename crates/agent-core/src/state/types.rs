use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionState {
    Idle,
    GoalInput,
    FeasibilityCheck,
    ProfileCollection,
    CurriculumPlanning,
    ChapterLearning,
    Completed,
    Error,
}

impl std::fmt::Display for SessionState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SessionState::Idle => write!(f, "IDLE"),
            SessionState::GoalInput => write!(f, "GOAL_INPUT"),
            SessionState::FeasibilityCheck => write!(f, "FEASIBILITY_CHECK"),
            SessionState::ProfileCollection => write!(f, "PROFILE_COLLECTION"),
            SessionState::CurriculumPlanning => write!(f, "CURRICULUM_PLANNING"),
            SessionState::ChapterLearning => write!(f, "CHAPTER_LEARNING"),
            SessionState::Completed => write!(f, "COMPLETED"),
            SessionState::Error => write!(f, "ERROR"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Transition {
    SubmitGoal,
    GoalFeasible,
    GoalInfeasible,
    ProfileContinue,
    ProfileComplete,
    CurriculumReady,
    ChapterComplete,
    AllChaptersDone,
    ErrorOccurred,
    Retry,
    Reset,
}

#[derive(Debug, Error)]
pub enum StateError {
    #[error("Invalid transition: cannot {transition:?} from {from:?}")]
    InvalidTransition {
        from: SessionState,
        transition: Transition,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_state_display() {
        assert_eq!(SessionState::Idle.to_string(), "IDLE");
        assert_eq!(SessionState::GoalInput.to_string(), "GOAL_INPUT");
        assert_eq!(
            SessionState::ChapterLearning.to_string(),
            "CHAPTER_LEARNING"
        );
        assert_eq!(SessionState::Completed.to_string(), "COMPLETED");
        assert_eq!(SessionState::Error.to_string(), "ERROR");
    }
}
