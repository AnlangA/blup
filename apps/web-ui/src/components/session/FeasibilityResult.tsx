import { useSessionStore } from '../../state/sessionStore';

export function FeasibilityResult() {
  const feasibility = useSessionStore((s) => s.feasibility);
  const state = useSessionStore((s) => s.state);
  const reset = useSessionStore((s) => s.reset);
  const setState = useSessionStore((s) => s.setState);

  if (state !== 'FEASIBILITY_CHECK' && state !== 'PROFILE_COLLECTION') {
    return null;
  }

  if (!feasibility) {
    return (
      <div className="feasibility-result">
        <h2>Goal Analysis</h2>
        <div className="result-card">
          <p className="loading-text">Analyzing your learning goal...</p>
        </div>
      </div>
    );
  }

  return (
    <div className="feasibility-result">
      <h2>Goal Analysis</h2>
      {feasibility.feasible ? (
        <div className="result-card success">
          <p className="verdict">Your learning goal looks great!</p>
          <p className="reason">{feasibility.reason}</p>
          {feasibility.estimated_duration && (
            <p className="meta">
              Estimated duration: <strong>{feasibility.estimated_duration}</strong>
            </p>
          )}
          {feasibility.prerequisites.length > 0 && (
            <div className="suggestions">
              <p>Prerequisites:</p>
              <ul>
                {feasibility.prerequisites.map((p, i) => (
                  <li key={i}>{p}</li>
                ))}
              </ul>
            </div>
          )}
          <button onClick={() => setState('PROFILE_COLLECTION')}>
            Continue to Profile Setup
          </button>
        </div>
      ) : (
        <div className="result-card warning">
          <p className="verdict">Let's refine your goal</p>
          <p className="reason">{feasibility.reason}</p>
          {feasibility.suggestions.length > 0 && (
            <div className="suggestions">
              <p>Suggestions:</p>
              <ul>
                {feasibility.suggestions.map((s, i) => (
                  <li key={i}>{s}</li>
                ))}
              </ul>
            </div>
          )}
          <button onClick={reset}>Try a Different Goal</button>
        </div>
      )}
    </div>
  );
}
