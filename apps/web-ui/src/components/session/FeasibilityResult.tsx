import { useSessionStore } from '../../state/sessionStore';
import { useSession } from '../../hooks/query';
import { parseSession } from '../../types/session';
import type { FeasibilityData } from '../../types/session';

export function FeasibilityResult() {
  const sessionId = useSessionStore((s) => s.sessionId);
  const { data: rawSession } = useSession(sessionId);
  const reset = useSessionStore((s) => s.reset);

  const session = rawSession ? parseSession(rawSession as unknown as Record<string, unknown>) : null;
  const state = session?.state ?? 'IDLE';
  const feasibility: FeasibilityData | null = session?.feasibilityResult ?? null;

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
          {feasibility.estimatedDuration && (
            <p className="meta">
              Estimated duration:{' '}
              <strong>{feasibility.estimatedDuration}</strong>
            </p>
          )}
          {feasibility.prerequisites?.length > 0 && (
            <div className="suggestions">
              <p>Prerequisites:</p>
              <ul>
                {feasibility.prerequisites.map((p, i) => (
                  <li key={i}>{p}</li>
                ))}
              </ul>
            </div>
          )}
        </div>
      ) : (
        <div className="result-card warning">
          <p className="verdict">Let's refine your goal</p>
          <p className="reason">{feasibility.reason}</p>
          {feasibility.suggestions?.length > 0 && (
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
