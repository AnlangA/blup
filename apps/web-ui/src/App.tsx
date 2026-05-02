import { useEffect, useRef } from "react";
import { useSessionStore } from "./state/sessionStore";
import {
  useCreatePlan,
  useSession,
  useSessionPlanSync,
  useSyncPlansFromServer,
} from "./hooks/query";
import { GoalInput } from "./components/session/GoalInput";
import { FeasibilityResult } from "./components/session/FeasibilityResult";
import { ProfileQuestion } from "./components/session/ProfileQuestion";
import { CompletionScreen } from "./components/session/CompletionScreen";
import { ErrorDisplay } from "./components/shared/ErrorDisplay";
import { LearningLayout } from "./components/LearningLayout";
import { PlanSwitcher } from "./components/plan/PlanSwitcher";

function MainContent() {
  const sessionId = useSessionStore((s) => s.sessionId);
  const currentChapterId = useSessionStore((s) => s.currentChapterId);
  const setChapter = useSessionStore((s) => s.setChapter);
  const removePlan = useSessionStore((s) => s.removePlan);
  const plans = useSessionStore((s) => s.plans);
  const setActivePlan = useSessionStore((s) => s.setActivePlan);

  const {
    data: session,
    isLoading: sessionLoading,
    isError: sessionError,
    error: sessionErr,
  } = useSession(sessionId);

  useSessionPlanSync(sessionId, session);

  useEffect(() => {
    if (session?.current_chapter_id && !currentChapterId) {
      setChapter(session.current_chapter_id);
    }
  }, [session?.current_chapter_id, currentChapterId, setChapter]);

  // When the active session no longer exists on the server, clean up the
  // local stale entry and auto-switch to the next available plan.
  useEffect(() => {
    if (!sessionError || !sessionId) return;
    const code = (sessionErr as { code?: string } | null)?.code;
    if (code === "NOT_FOUND") {
      removePlan(sessionId);
      const remaining = plans.filter((p) => p.id !== sessionId);
      if (remaining.length > 0) {
        setActivePlan(remaining[0].id);
      }
    }
  }, [sessionError, sessionId, sessionErr, removePlan, plans, setActivePlan]);

  // Show loading skeleton while session data is fetched for the first time.
  if (sessionLoading) {
    return (
      <div className="main-content-area">
        <div className="loading-screen">Loading...</div>
      </div>
    );
  }

  if (sessionError) {
    return (
      <div className="main-content-area">
        <ErrorDisplay />
      </div>
    );
  }

  const state = session?.state ?? "IDLE";

  if (state === "ERROR") {
    return (
      <div className="main-content-area">
        <ErrorDisplay />
      </div>
    );
  }

  return (
    <div className="main-content-area">
      {(() => {
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
      })()}
    </div>
  );
}

function App() {
  const plans = useSessionStore((s) => s.plans);
  const sessionId = useSessionStore((s) => s.sessionId);
  const createPlan = useCreatePlan();

  // Tracks whether the auto-create effect has fired at least once.
  // Prevents showing "Initializing..." forever when the user deletes
  // all plans (which also makes plans.length === 0).
  const hasAttemptedCreate = useRef(false);

  // Periodically sync all plan states from the server so that inactive
  // plans reflect state transitions made by backend operations (e.g.,
  // completing all chapters advances the plan to COMPLETED).
  useSyncPlansFromServer();

  // Auto-create first plan on initial load if none exist.
  // Guarded by hasAttemptedCreate ref so React 18 Strict Mode (which
  // double-invokes effects in dev) doesn't create two empty plans.
  useEffect(() => {
    if (plans.length === 0 && !createPlan.isPending && !hasAttemptedCreate.current) {
      hasAttemptedCreate.current = true;
      createPlan.mutate();
    }
  }, []); // eslint-disable-line react-hooks/exhaustive-deps

  // No plans yet — always show the empty-state shell with a "+ New Plan"
  // button. The auto-create effect fires in the background on initial load
  // so the user never gets stuck on a loading screen.
  if (plans.length === 0) {
    if (createPlan.isError && hasAttemptedCreate.current) {
      return <ErrorDisplay />;
    }
    return (
      <div className="app-shell">
        <PlanSwitcher />
        <div className="main-content-area">
          <div className="welcome-content">
            <h2>Welcome to Blup</h2>
            <p>Create a new plan to start learning.</p>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="app-shell">
      <PlanSwitcher />
      {sessionId ? (
        <MainContent />
      ) : (
        <div className="main-content-area">
          <div className="welcome-content">
            <h2>Welcome to Blup</h2>
            <p>Create a new plan to start learning.</p>
          </div>
        </div>
      )}
    </div>
  );
}

export default App;
