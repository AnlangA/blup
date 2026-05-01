use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::types::{SessionState, StateError, Transition};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransitionRecord {
    pub from: SessionState,
    pub to: SessionState,
    pub transition: Transition,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct StateMachine {
    current_state: SessionState,
    previous_state: Option<SessionState>,
    transition_history: Vec<TransitionRecord>,
}

impl StateMachine {
    pub fn new() -> Self {
        Self {
            current_state: SessionState::Idle,
            previous_state: None,
            transition_history: Vec::new(),
        }
    }

    pub fn current_state(&self) -> SessionState {
        self.current_state
    }

    /// Create a new state machine starting from a given state.
    pub fn with_state(state: SessionState) -> Self {
        Self {
            current_state: state,
            previous_state: None,
            transition_history: Vec::new(),
        }
    }

    pub fn previous_state(&self) -> Option<SessionState> {
        self.previous_state
    }

    pub fn set_previous_state(&mut self, state: SessionState) {
        self.previous_state = Some(state);
    }

    pub fn transition(&mut self, transition: Transition) -> Result<SessionState, StateError> {
        let next_state = self.validate_transition(&transition)?;

        let record = TransitionRecord {
            from: self.current_state,
            to: next_state,
            transition,
            timestamp: Utc::now(),
        };

        if next_state == SessionState::Error {
            self.previous_state = Some(self.current_state);
        }

        self.current_state = next_state;
        self.transition_history.push(record);

        Ok(next_state)
    }

    fn validate_transition(&self, transition: &Transition) -> Result<SessionState, StateError> {
        use SessionState::*;
        use Transition::*;

        let next = match (&self.current_state, transition) {
            (Idle, SubmitGoal) => GoalInput,
            (GoalInput, SubmitGoal) => FeasibilityCheck,
            (FeasibilityCheck, GoalFeasible) => ProfileCollection,
            (FeasibilityCheck, GoalInfeasible) => GoalInput,
            (ProfileCollection, ProfileContinue) => ProfileCollection,
            (ProfileCollection, ProfileComplete) => CurriculumPlanning,
            (CurriculumPlanning, CurriculumReady) => ChapterLearning,
            (ChapterLearning, ChapterComplete) => ChapterLearning,
            (ChapterLearning, AllChaptersDone) => Completed,
            (Completed, Reset) => Idle,

            (_, ErrorOccurred) => Error,
            (Error, Retry) => self.previous_state.unwrap_or(Idle),
            (Error, Reset) => Idle,

            _ => {
                return Err(StateError::InvalidTransition {
                    from: self.current_state,
                    transition: transition.clone(),
                });
            }
        };

        Ok(next)
    }

    pub fn history(&self) -> &[TransitionRecord] {
        &self.transition_history
    }

    /// Replay a historical transition record without re-validating.
    /// Used when reconstructing state machine from persisted snapshots.
    pub fn replay_record(&mut self, record: &TransitionRecord) {
        self.previous_state = Some(record.from);
        self.current_state = record.to;
        self.transition_history.push(record.clone());
    }
}

impl Default for StateMachine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initial_state_is_idle() {
        let sm = StateMachine::new();
        assert_eq!(sm.current_state(), SessionState::Idle);
    }

    #[test]
    fn test_idle_to_goal_input() {
        let mut sm = StateMachine::new();
        let result = sm.transition(Transition::SubmitGoal);
        assert!(result.is_ok());
        assert_eq!(sm.current_state(), SessionState::GoalInput);
    }

    #[test]
    fn test_goal_input_to_feasibility_check() {
        let mut sm = StateMachine::new();
        sm.transition(Transition::SubmitGoal).unwrap();
        let result = sm.transition(Transition::SubmitGoal);
        assert!(result.is_ok());
        assert_eq!(sm.current_state(), SessionState::FeasibilityCheck);
    }

    #[test]
    fn test_feasibility_feasible_to_profile() {
        let mut sm = StateMachine::new();
        sm.transition(Transition::SubmitGoal).unwrap();
        sm.transition(Transition::SubmitGoal).unwrap();
        let result = sm.transition(Transition::GoalFeasible);
        assert!(result.is_ok());
        assert_eq!(sm.current_state(), SessionState::ProfileCollection);
    }

    #[test]
    fn test_feasibility_infeasible_to_goal_input() {
        let mut sm = StateMachine::new();
        sm.transition(Transition::SubmitGoal).unwrap();
        sm.transition(Transition::SubmitGoal).unwrap();
        let result = sm.transition(Transition::GoalInfeasible);
        assert!(result.is_ok());
        assert_eq!(sm.current_state(), SessionState::GoalInput);
    }

    #[test]
    fn test_profile_to_curriculum() {
        let mut sm = StateMachine::new();
        sm.transition(Transition::SubmitGoal).unwrap();
        sm.transition(Transition::SubmitGoal).unwrap();
        sm.transition(Transition::GoalFeasible).unwrap();
        let result = sm.transition(Transition::ProfileComplete);
        assert!(result.is_ok());
        assert_eq!(sm.current_state(), SessionState::CurriculumPlanning);
    }

    #[test]
    fn test_curriculum_to_chapter_learning() {
        let mut sm = StateMachine::new();
        sm.transition(Transition::SubmitGoal).unwrap();
        sm.transition(Transition::SubmitGoal).unwrap();
        sm.transition(Transition::GoalFeasible).unwrap();
        sm.transition(Transition::ProfileComplete).unwrap();
        let result = sm.transition(Transition::CurriculumReady);
        assert!(result.is_ok());
        assert_eq!(sm.current_state(), SessionState::ChapterLearning);
    }

    #[test]
    fn test_chapter_learning_to_completed() {
        let mut sm = StateMachine::new();
        sm.transition(Transition::SubmitGoal).unwrap();
        sm.transition(Transition::SubmitGoal).unwrap();
        sm.transition(Transition::GoalFeasible).unwrap();
        sm.transition(Transition::ProfileComplete).unwrap();
        sm.transition(Transition::CurriculumReady).unwrap();
        let result = sm.transition(Transition::AllChaptersDone);
        assert!(result.is_ok());
        assert_eq!(sm.current_state(), SessionState::Completed);
    }

    #[test]
    fn test_completed_to_idle_reset() {
        let mut sm = StateMachine::new();
        sm.transition(Transition::SubmitGoal).unwrap();
        sm.transition(Transition::SubmitGoal).unwrap();
        sm.transition(Transition::GoalFeasible).unwrap();
        sm.transition(Transition::ProfileComplete).unwrap();
        sm.transition(Transition::CurriculumReady).unwrap();
        sm.transition(Transition::AllChaptersDone).unwrap();
        let result = sm.transition(Transition::Reset);
        assert!(result.is_ok());
        assert_eq!(sm.current_state(), SessionState::Idle);
    }

    #[test]
    fn test_invalid_idle_to_feasibility() {
        let mut sm = StateMachine::new();
        let result = sm.transition(Transition::GoalFeasible);
        assert!(result.is_err());
    }

    #[test]
    fn test_error_from_any_state() {
        let transitions_list: Vec<Vec<Transition>> = vec![
            vec![Transition::SubmitGoal],
            vec![Transition::SubmitGoal, Transition::SubmitGoal],
            vec![
                Transition::SubmitGoal,
                Transition::SubmitGoal,
                Transition::GoalFeasible,
            ],
            vec![
                Transition::SubmitGoal,
                Transition::SubmitGoal,
                Transition::GoalFeasible,
                Transition::ProfileComplete,
            ],
        ];

        for transitions in transitions_list {
            let mut sm = StateMachine::new();
            for t in transitions {
                sm.transition(t).unwrap();
            }
            let result = sm.transition(Transition::ErrorOccurred);
            assert!(result.is_ok());
            assert_eq!(sm.current_state(), SessionState::Error);
        }
    }

    #[test]
    fn test_error_retry_returns_to_previous() {
        let mut sm = StateMachine::new();
        sm.transition(Transition::SubmitGoal).unwrap();
        sm.transition(Transition::SubmitGoal).unwrap();
        sm.transition(Transition::GoalFeasible).unwrap();
        sm.transition(Transition::ErrorOccurred).unwrap();
        assert_eq!(sm.current_state(), SessionState::Error);
        let result = sm.transition(Transition::Retry);
        assert!(result.is_ok());
        assert_eq!(sm.current_state(), SessionState::ProfileCollection);
    }

    #[test]
    fn test_error_reset_returns_to_idle() {
        let mut sm = StateMachine::new();
        sm.transition(Transition::SubmitGoal).unwrap();
        sm.transition(Transition::ErrorOccurred).unwrap();
        let result = sm.transition(Transition::Reset);
        assert!(result.is_ok());
        assert_eq!(sm.current_state(), SessionState::Idle);
    }

    #[test]
    fn test_error_retry_without_previous_state() {
        let mut sm = StateMachine::new();
        sm.transition(Transition::ErrorOccurred).unwrap();
        let result = sm.transition(Transition::Retry);
        assert!(result.is_ok());
        assert_eq!(sm.current_state(), SessionState::Idle);
    }

    #[test]
    fn test_history_records_transitions() {
        let mut sm = StateMachine::new();
        sm.transition(Transition::SubmitGoal).unwrap();
        sm.transition(Transition::SubmitGoal).unwrap();
        assert_eq!(sm.history().len(), 2);
        assert_eq!(sm.history()[0].from, SessionState::Idle);
        assert_eq!(sm.history()[0].to, SessionState::GoalInput);
    }

    #[test]
    fn test_default_trait() {
        let sm = StateMachine::default();
        assert_eq!(sm.current_state(), SessionState::Idle);
    }
}
