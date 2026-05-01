import { useState } from 'react';
import { useSessionStore } from '../../state/sessionStore';
import { useSubmitGoalStream } from '../../hooks/query';

export function GoalInput() {
  const sessionId = useSessionStore((s) => s.sessionId);
  const { submit, reset, isStreaming, message, error } =
    useSubmitGoalStream(sessionId);
  const [description, setDescription] = useState('');
  const [domain, setDomain] = useState('');
  const [context, setContext] = useState('');

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (!description || !domain) return;
    submit({
      description,
      domain,
      context: context || undefined,
    });
  };

  const handleRetry = () => {
    reset();
  };

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
            disabled={isStreaming}
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
            disabled={isStreaming}
          />
        </div>
        <div className="form-group">
          <label htmlFor="context">Context (optional)</label>
          <textarea
            id="context"
            value={context}
            onChange={(e) => setContext(e.target.value)}
            placeholder="Any background about why you want to learn this"
            disabled={isStreaming}
          />
        </div>
        {isStreaming && (
          <div className="goal-stream-status">
            <div className="spinner" />
            <span>{message || 'Checking feasibility...'}</span>
          </div>
        )}
        {error && (
          <div className="goal-stream-error">
            <p className="error-text">{error}</p>
            <button type="button" className="retry-btn" onClick={handleRetry}>
              Try Again
            </button>
          </div>
        )}
        <button
          type="submit"
          disabled={isStreaming || !description || !domain}
        >
          {isStreaming ? 'Checking...' : 'Start Learning'}
        </button>
      </form>
    </div>
  );
}
