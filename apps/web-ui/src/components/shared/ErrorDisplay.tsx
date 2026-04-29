import { useSessionStore } from '../../state/sessionStore';
import { useCreateSession } from '../../hooks/query';

export function ErrorDisplay() {
  const reset = useSessionStore((s) => s.reset);
  const createSession = useCreateSession();

  return (
    <div className="error-display" role="alert">
      <h3>Something went wrong</h3>
      <p>An unexpected error occurred. Please try again.</p>
      <div className="error-actions">
        <button
          onClick={() => createSession.mutate()}
          disabled={createSession.isPending}
        >
          Retry
        </button>
        <button className="secondary" onClick={reset}>
          Start Over
        </button>
      </div>
    </div>
  );
}
