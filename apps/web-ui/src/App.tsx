import { useEffect } from 'react';
import { useSessionStore } from './state/sessionStore';
import { useCreateSession, useSession } from './hooks/query';
import { GoalInput } from './components/session/GoalInput';
import { FeasibilityResult } from './components/session/FeasibilityResult';
import { ProfileQuestion } from './components/session/ProfileQuestion';
import { CompletionScreen } from './components/session/CompletionScreen';
import { ErrorDisplay } from './components/shared/ErrorDisplay';
import { LearningLayout } from './components/LearningLayout';

function App() {
  const sessionId = useSessionStore((s) => s.sessionId);
  const createSession = useCreateSession();

  // Create session on first load if none exists
  useEffect(() => {
    if (!sessionId) {
      createSession.mutate();
    }
  }, []); // eslint-disable-line react-hooks/exhaustive-deps

  // Get server state
  const { data: session } = useSession(
    createSession.isSuccess ? sessionId : null,
  );

  // Derive state from backend
  const state = session?.state ?? 'IDLE';

  if (createSession.isPending) {
    return <div className="loading-screen">Initializing...</div>;
  }

  if (createSession.isError || state === 'ERROR') {
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
