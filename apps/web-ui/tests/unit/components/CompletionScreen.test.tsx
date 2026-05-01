import { describe, it, expect } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { CompletionScreen } from '../../../src/components/session/CompletionScreen';

vi.mock('../../../src/state/sessionStore', () => ({
  useSessionStore: vi.fn((selector?: (s: unknown) => unknown) => {
    const state = { sessionId: 'test-session-id' };
    return selector ? selector(state) : state;
  }),
}));

vi.mock('../../../src/hooks/query', () => ({
  useCurriculum: () => ({
    data: {
      title: 'Test Curriculum',
      chapters: [
        { id: 'ch1', title: 'Chapter 1', order: 1, objectives: [] },
        { id: 'ch2', title: 'Chapter 2', order: 2, objectives: [] },
      ],
    },
  }),
  useCreatePlan: () => ({
    mutate: vi.fn(),
    isPending: false,
  }),
}));

describe('CompletionScreen', () => {
  it('renders congratulations message', () => {
    render(<CompletionScreen />);
    expect(screen.getByText('Congratulations!')).toBeInTheDocument();
  });

  it('shows chapter count', () => {
    render(<CompletionScreen />);
    expect(screen.getByText(/2 chapters completed/)).toBeInTheDocument();
  });

  it('renders new plan button', () => {
    render(<CompletionScreen />);
    expect(
      screen.getByRole('button', { name: /start a new learning goal/i }),
    ).toBeInTheDocument();
  });
});
