import { useState } from 'react';
import { useSessionStore } from '../../state/sessionStore';
import { useSubmitGoal } from '../../hooks/query';

export function GoalInput() {
  const sessionId = useSessionStore((s) => s.sessionId);
  const submitGoal = useSubmitGoal(sessionId);
  const [description, setDescription] = useState('');
  const [domain, setDomain] = useState('');
  const [context, setContext] = useState('');

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (!description || !domain) return;
    submitGoal.mutate({
      description,
      domain,
      context: context || undefined,
    });
  };

  const loading = submitGoal.isPending;

  return (
    <div className="goal-input-container">
      <h1>What do you want to learn?</h1>
      <form onSubmit={handleSubmit}>
        <div className="form-group">
          <label htmlFor="description">Learning Goal</label>
          <textarea
            id="description"
            value={description}
            onChange={(e) => setDescription(e.target.value)}
            placeholder="e.g., I want to learn Python for data analysis"
            required
            minLength={10}
          />
        </div>
        <div className="form-group">
          <label htmlFor="domain">Subject Domain</label>
          <input
            id="domain"
            type="text"
            value={domain}
            onChange={(e) => setDomain(e.target.value)}
            placeholder="e.g., programming, mathematics, physics"
            required
          />
        </div>
        <div className="form-group">
          <label htmlFor="context">Context (optional)</label>
          <textarea
            id="context"
            value={context}
            onChange={(e) => setContext(e.target.value)}
            placeholder="Any background about why you want to learn this"
          />
        </div>
        {submitGoal.isError && (
          <p className="error-text">
            {submitGoal.error?.message || 'Failed to submit goal'}
          </p>
        )}
        <button type="submit" disabled={loading || !description || !domain}>
          {loading ? 'Checking...' : 'Start Learning'}
        </button>
      </form>
    </div>
  );
}
