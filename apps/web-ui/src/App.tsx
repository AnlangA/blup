import { useEffect } from "react";
import { useSessionStore } from "./state/sessionStore";
import { useCreateSession, useSession } from "./hooks/query";
import { GoalInput } from "./components/session/GoalInput";
import { FeasibilityResult } from "./components/session/FeasibilityResult";
import { ProfileQuestion } from "./components/session/ProfileQuestion";
import { CompletionScreen } from "./components/session/CompletionScreen";
import { ErrorDisplay } from "./components/shared/ErrorDisplay";
import { LearningLayout } from "./components/LearningLayout";

function App() {
  const sessionId = useSessionStore((s) => s.sessionId);
  const currentChapterId = useSessionStore((s) => s.currentChapterId);
  const setChapter = useSessionStore((s) => s.setChapter);
  const reset = useSessionStore((s) => s.reset);
  const createSession = useCreateSession();

  // Create session on first load if none exists
  useEffect(() => {
    if (!sessionId) {
      createSession.mutate();
    }
  }, []); // eslint-disable-line react-hooks/exhaustive-deps

  // Always try to restore the session if we have an ID (from localStorage or
  // newly created).  enable only when we have a real session ID.
  const {
    data: session,
    isError: sessionError,
    error: sessionErr,
  } = useSession(sessionId);

  // Restore currentChapterId from server session snapshot if client lost it
  useEffect(() => {
    if (session?.current_chapter_id && !currentChapterId) {
      setChapter(session.current_chapter_id);
    }
  }, [session?.current_chapter_id, currentChapterId, setChapter]);

  // If the session query failed with NOT_FOUND (e.g. stale session after
  // backend restart), clear the stale ID and create a new session.
  useEffect(() => {
    const code = (sessionErr as { code?: string } | null)?.code;
    if (sessionError && code === "NOT_FOUND") {
      reset();
      createSession.mutate();
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [sessionError]);

  // Derive state from backend
  const state = session?.state ?? "IDLE";

  if (createSession.isPending) {
    return <div className="loading-screen">Initializing...</div>;
  }

  if (createSession.isError || state === "ERROR") {
    return <ErrorDisplay />;
  }

  switch (state) {
    case "IDLE":
    case "GOAL_INPUT":
      return <GoalInput />;
    case "FEASIBILITY_CHECK":
    case "PROFILE_COLLECTION":
      return (
        <>
          <FeasibilityResult />
          {state === "PROFILE_COLLECTION" && <ProfileQuestion />}
        </>
      );
    case "CURRICULUM_PLANNING":
    case "CHAPTER_LEARNING":
      return <LearningLayout />;
    case "COMPLETED":
      return <CompletionScreen />;
    default:
      return <GoalInput />;
  }
}

export default App;
