import { useSessionStore } from '../../state/sessionStore';

export function ErrorDisplay() {
  const error = useSessionStore((s) => s.error);
  const reset = useSessionStore((s) => s.reset);
  const createSession = useSessionStore((s) => s.createSession);

  return (
    <div className="error-display" role="alert">
      <h3>Something went wrong</h3>
      <p>{error?.message || 'An unexpected error occurred'}</p>
      {error?.code && <code className="error-code">Code: {error.code}</code>}
      <div className="error-actions">
        <button onClick={() => createSession()}>Retry</button>
        <button className="secondary" onClick={reset}>Start Over</button>
      </div>
    </div>
  );
}
