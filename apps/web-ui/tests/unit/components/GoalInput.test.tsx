import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { GoalInput } from '../../../src/components/session/GoalInput';

// Mock the session store
vi.mock('../../../src/state/sessionStore', () => ({
  useSessionStore: vi.fn((selector?: (s: unknown) => unknown) => {
    const state = { sessionId: 'test-session-id' };
    return selector ? selector(state) : state;
  }),
}));

const renderWithProviders = (ui: React.ReactElement) => {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  return render(
    <QueryClientProvider client={queryClient}>{ui}</QueryClientProvider>,
  );
};

describe('GoalInput', () => {
  it('renders the form correctly', () => {
    renderWithProviders(<GoalInput />);

    expect(screen.getByText('What do you want to learn?')).toBeInTheDocument();
    expect(screen.getByLabelText('Learning Goal')).toBeInTheDocument();
    expect(screen.getByLabelText('Subject Domain')).toBeInTheDocument();
    expect(screen.getByLabelText('Context (optional)')).toBeInTheDocument();
    expect(
      screen.getByRole('button', { name: 'Start Learning' }),
    ).toBeInTheDocument();
  });

  it('has required fields', () => {
    renderWithProviders(<GoalInput />);

    expect(screen.getByLabelText('Learning Goal')).toBeRequired();
    expect(screen.getByLabelText('Subject Domain')).toBeRequired();
  });

  it('validates minimum length for description', () => {
    renderWithProviders(<GoalInput />);

    expect(screen.getByLabelText('Learning Goal')).toHaveAttribute(
      'minLength',
      '10',
    );
  });

  it('disables submit button when fields are empty', () => {
    renderWithProviders(<GoalInput />);

    expect(
      screen.getByRole('button', { name: 'Start Learning' }),
    ).toBeDisabled();
  });

  it('enables submit button when fields are filled', () => {
    renderWithProviders(<GoalInput />);

    fireEvent.change(screen.getByLabelText('Learning Goal'), {
      target: { value: 'Learn Python for data analysis' },
    });
    fireEvent.change(screen.getByLabelText('Subject Domain'), {
      target: { value: 'programming' },
    });

    expect(
      screen.getByRole('button', { name: 'Start Learning' }),
    ).not.toBeDisabled();
  });
});
