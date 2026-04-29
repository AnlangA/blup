import { useEffect } from 'react';
import { useSessionStore } from './state/sessionStore';
import { GoalInput } from './components/session/GoalInput';
import { FeasibilityResult } from './components/session/FeasibilityResult';
import { ProfileQuestion } from './components/session/ProfileQuestion';
import { CompletionScreen } from './components/session/CompletionScreen';
import { ErrorDisplay } from './components/shared/ErrorDisplay';
import { LearningLayout } from './components/LearningLayout';

function App() {
  const state = useSessionStore((s) => s.state);
  const sessionId = useSessionStore((s) => s.sessionId);
  const createSession = useSessionStore((s) => s.createSession);

  useEffect(() => {
    if (!sessionId) {
      createSession();
    }
  }, [sessionId, createSession]);

  if (state === 'ERROR') {
    return <ErrorDisplay />;
  }

  switch (state) {
    case 'IDLE':
    case 'GOAL_INPUT':
      return <GoalInput />;
    case 'FEASIBILITY_CHECK':
    case 'PROFILE_COLLECTION':
      return (
        <>
          <FeasibilityResult />
          {state === 'PROFILE_COLLECTION' && <ProfileQuestion />}
        </>
      );
    case 'CURRICULUM_PLANNING':
    case 'CHAPTER_LEARNING':
      return <LearningLayout />;
    case 'COMPLETED':
      return <CompletionScreen />;
    default:
      return <GoalInput />;
  }
}

export default App;
