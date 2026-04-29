import { useSessionStore } from '../../state/sessionStore';

export function CompletionScreen() {
  const reset = useSessionStore((s) => s.reset);
  const chapters = useSessionStore((s) => s.chapters);

  return (
    <div className="completion-screen">
      <h1>Congratulations!</h1>
      <p>You've completed all chapters in this curriculum.</p>
      {chapters.length > 0 && (
        <p className="summary">
          {chapters.length} chapter{chapters.length !== 1 ? 's' : ''} completed
        </p>
      )}
      <div className="actions">
        <button onClick={reset}>Start a New Learning Goal</button>
      </div>
    </div>
  );
}
