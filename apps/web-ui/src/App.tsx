import { useEffect } from "react";
import { useSessionStore } from "./state/sessionStore";
import { useCreatePlan, useSession, useSyncPlanFromSession } from "./hooks/query";
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

  const {
    data: session,
    isError: sessionError,
    error: sessionErr,
  } = useSession(sessionId);

  useSyncPlanFromSession(sessionId, session);

  useEffect(() => {
    if (session?.current_chapter_id && !currentChapterId) {
      setChapter(session.current_chapter_id);
    }
  }, [session?.current_chapter_id, currentChapterId, setChapter]);

  const state = session?.state ?? "IDLE";

  if (sessionError) {
    const code = (sessionErr as { code?: string } | null)?.code;
    if (code === "NOT_FOUND") {
      return (
        <div className="main-content-area">
          <div className="welcome-content">
            <p>This session no longer exists. Please select another plan.</p>
          </div>
        </div>
      );
    }
    return (
      <div className="main-content-area">
        <ErrorDisplay />
      </div>
    );
  }

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

  // Auto-create first plan on initial load if none exist
  useEffect(() => {
    if (plans.length === 0 && !createPlan.isPending) {
      createPlan.mutate();
    }
  }, []); // eslint-disable-line react-hooks/exhaustive-deps

  if (createPlan.isPending && plans.length === 0) {
    return <div className="loading-screen">Initializing...</div>;
  }

  if (createPlan.isError && plans.length === 0) {
    return <ErrorDisplay />;
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
