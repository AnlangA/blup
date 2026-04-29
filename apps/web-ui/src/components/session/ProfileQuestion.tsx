import { useState } from 'react';
import { useSessionStore } from '../../state/sessionStore';
import { useSession, useSubmitProfile } from '../../hooks/query';

const QUESTIONS: Record<number, { prompt: string; options: string[] }> = {
  0: {
    prompt: 'What experience do you have with this subject?',
    options: [
      'No experience at all',
      'Basic familiarity',
      'Some practical experience',
      'Advanced knowledge',
    ],
  },
  1: {
    prompt: 'How do you prefer to learn new material?',
    options: [
      'Reading text and documentation',
      'Watching video tutorials',
      'Hands-on exercises and coding',
      'Interactive discussions',
    ],
  },
  2: {
    prompt: 'How much time can you dedicate each week?',
    options: [
      'Less than 2 hours',
      '2-5 hours',
      '5-10 hours',
      'More than 10 hours',
    ],
  },
};

export function ProfileQuestion() {
  const sessionId = useSessionStore((s) => s.sessionId);
  const { data: session } = useSession(sessionId);
  const submitProfile = useSubmitProfile(sessionId);
  const [answer, setAnswer] = useState('');

  const round = (session?.profile_rounds as number) ?? 0;
  const question = QUESTIONS[round] ?? QUESTIONS[0];
  const totalRounds = 3;

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (!sessionId || !answer.trim()) return;
    submitProfile.mutate(
      {
        question_id: `q${round}`,
        answer: answer.trim(),
      },
      { onSuccess: () => setAnswer('') },
    );
  };

  const loading = submitProfile.isPending;

  return (
    <div className="profile-question">
      <h2>Tell Us About Yourself</h2>
      <p className="round-indicator">
        Question {round + 1} of {totalRounds}
      </p>
      <p className="prompt">{question.prompt}</p>
      <form onSubmit={handleSubmit}>
        <div className="options">
          {question.options.map((opt) => (
            <label key={opt} className="option-label">
              <input
                type="radio"
                name="answer"
                value={opt}
                checked={answer === opt}
                onChange={(e) => setAnswer(e.target.value)}
              />
              {opt}
            </label>
          ))}
          <label className="option-label">
            <input
              type="radio"
              name="answer"
              value=""
              checked={
                answer !== '' &&
                !question.options.includes(answer)
              }
              onChange={() => setAnswer('')}
            />
            Other (type below)
          </label>
        </div>
        <div className="form-group">
          <textarea
            value={
              question.options.includes(answer) ? '' : answer
            }
            onChange={(e) => setAnswer(e.target.value)}
            placeholder="Or describe in your own words..."
            rows={3}
          />
        </div>
        {submitProfile.isError && (
          <p className="error-text">
            Failed to submit. Please try again.
          </p>
        )}
        <button type="submit" disabled={loading || !answer.trim()}>
          {loading ? 'Submitting...' : round === 2 ? 'Complete Profile' : 'Continue'}
        </button>
      </form>
    </div>
  );
}
