import { useState } from 'react';
import { useSessionStore } from '../../state/sessionStore';
import { api } from '../../api/client';

export function ProfileQuestion() {
  const sessionId = useSessionStore((s) => s.sessionId);
  const setState = useSessionStore((s) => s.setState);
  const [answer, setAnswer] = useState('');
  const [loading, setLoading] = useState(false);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!sessionId || !answer.trim()) return;
    setLoading(true);
    try {
      const result = await api.submitProfileAnswer(sessionId, {
        question_id: 'q1',
        answer: answer.trim(),
      });
      if (result.is_complete) {
        setState('CURRICULUM_PLANNING');
      }
      setAnswer('');
    } catch {
      // error handled by store
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="profile-question">
      <h2>Tell Us About Yourself</h2>
      <p className="prompt">
        What experience do you have with this subject?
      </p>
      <form onSubmit={handleSubmit}>
        <div className="options">
          {['No experience at all', 'Basic familiarity', 'Some practical experience', 'Advanced knowledge'].map((opt) => (
            <label key={opt} className="option-label">
              <input
                type="radio"
                name="experience"
                value={opt}
                checked={answer === opt}
                onChange={(e) => setAnswer(e.target.value)}
              />
              {opt}
            </label>
          ))}
        </div>
        <div className="form-group">
          <textarea
            value={answer}
            onChange={(e) => setAnswer(e.target.value)}
            placeholder="Or describe in your own words..."
            rows={3}
          />
        </div>
        <button type="submit" disabled={loading || !answer.trim()}>
          {loading ? 'Submitting...' : 'Continue'}
        </button>
      </form>
    </div>
  );
}
