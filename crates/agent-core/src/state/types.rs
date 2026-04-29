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

#[derive(Debug, Clone, PartialEq, Eq)]
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
