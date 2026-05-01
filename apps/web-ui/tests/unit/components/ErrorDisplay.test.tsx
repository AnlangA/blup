import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import { ErrorDisplay } from '../../../src/components/shared/ErrorDisplay';

vi.mock('../../../src/hooks/query', () => ({
  useCreatePlan: () => ({
    mutate: vi.fn(),
    isPending: false,
  }),
}));

describe('ErrorDisplay', () => {
  it('renders error heading', () => {
    render(<ErrorDisplay />);
    expect(screen.getByText('Something went wrong')).toBeInTheDocument();
  });

  it('renders retry button', () => {
    render(<ErrorDisplay />);
    expect(screen.getByRole('button', { name: /retry/i })).toBeInTheDocument();
  });

  it('has alert role for accessibility', () => {
    render(<ErrorDisplay />);
    expect(screen.getByRole('alert')).toBeInTheDocument();
  });
});
