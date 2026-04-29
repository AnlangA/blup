import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { GoalInput } from '../../../src/components/session/GoalInput';

// Mock the session store
vi.mock('../../../src/state/sessionStore', () => ({
  useSessionStore: vi.fn(() => ({
    submitGoal: vi.fn(),
  })),
}));

describe('GoalInput', () => {
  it('renders the form correctly', () => {
    render(<GoalInput />);
    
    expect(screen.getByText('What do you want to learn?')).toBeInTheDocument();
    expect(screen.getByLabelText('Learning Goal')).toBeInTheDocument();
    expect(screen.getByLabelText('Subject Domain')).toBeInTheDocument();
    expect(screen.getByLabelText('Context (optional)')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Start Learning' })).toBeInTheDocument();
  });

  it('has required fields', () => {
    render(<GoalInput />);
    
    const descriptionInput = screen.getByLabelText('Learning Goal');
    const domainInput = screen.getByLabelText('Subject Domain');
    
    expect(descriptionInput).toBeRequired();
    expect(domainInput).toBeRequired();
  });

  it('validates minimum length for description', () => {
    render(<GoalInput />);
    
    const descriptionInput = screen.getByLabelText('Learning Goal');
    expect(descriptionInput).toHaveAttribute('minLength', '10');
  });

  it('disables submit button when fields are empty', () => {
    render(<GoalInput />);
    
    const submitButton = screen.getByRole('button', { name: 'Start Learning' });
    expect(submitButton).toBeDisabled();
  });

  it('enables submit button when fields are filled', () => {
    render(<GoalInput />);
    
    const descriptionInput = screen.getByLabelText('Learning Goal');
    const domainInput = screen.getByLabelText('Subject Domain');
    
    fireEvent.change(descriptionInput, { target: { value: 'Learn Python for data analysis' } });
    fireEvent.change(domainInput, { target: { value: 'programming' } });
    
    const submitButton = screen.getByRole('button', { name: 'Start Learning' });
    expect(submitButton).not.toBeDisabled();
  });
});
