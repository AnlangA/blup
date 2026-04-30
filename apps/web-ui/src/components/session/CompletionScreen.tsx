import { useSessionStore } from '../../state/sessionStore';
import { useCurriculum, useCreatePlan } from '../../hooks/query';

export function CompletionScreen() {
  const sessionId = useSessionStore((s) => s.sessionId);
  const { data: curriculum } = useCurriculum(sessionId);
  const createPlan = useCreatePlan();

  const chapterCount = curriculum?.chapters?.length ?? 0;

  return (
    <div className="completion-screen">
      <h1>Congratulations!</h1>
      <p>You've completed all chapters in this curriculum.</p>
      {chapterCount > 0 && (
        <p className="summary">
          {chapterCount} chapter{chapterCount !== 1 ? 's' : ''} completed
        </p>
      )}
      <div className="actions">
        <button
          onClick={() => createPlan.mutate()}
          disabled={createPlan.isPending}
        >
          {createPlan.isPending ? 'Creating...' : 'Start a New Learning Goal'}
        </button>
      </div>
    </div>
  );
}
