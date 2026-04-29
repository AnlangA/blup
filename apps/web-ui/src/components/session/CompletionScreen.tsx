import { useSessionStore } from '../../state/sessionStore';
import { useCurriculum } from '../../hooks/query';

export function CompletionScreen() {
  const sessionId = useSessionStore((s) => s.sessionId);
  const reset = useSessionStore((s) => s.reset);
  const { data: curriculum } = useCurriculum(sessionId);

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
        <button onClick={reset}>Start a New Learning Goal</button>
      </div>
    </div>
  );
}
