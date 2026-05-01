import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import { PlanCard } from '../../../src/components/plan/PlanCard';
import type { PlanMeta } from '../../../src/state/sessionStore';

const basePlan: PlanMeta = {
  id: 'plan-1',
  title: 'Learn Python',
  domain: 'programming',
  state: 'CHAPTER_LEARNING',
  createdAt: new Date().toISOString(),
  updatedAt: new Date().toISOString(),
};

describe('PlanCard', () => {
  it('renders plan title', () => {
    render(
      <PlanCard
        plan={basePlan}
        isActive={false}
        onSelect={vi.fn()}
        onDelete={vi.fn()}
      />,
    );
    expect(screen.getByText('Learn Python')).toBeInTheDocument();
  });

  it('renders domain badge', () => {
    render(
      <PlanCard
        plan={basePlan}
        isActive={false}
        onSelect={vi.fn()}
        onDelete={vi.fn()}
      />,
    );
    expect(screen.getByText('programming')).toBeInTheDocument();
  });

  it('renders state label', () => {
    render(
      <PlanCard
        plan={basePlan}
        isActive={false}
        onSelect={vi.fn()}
        onDelete={vi.fn()}
      />,
    );
    expect(screen.getByText('Learning')).toBeInTheDocument();
  });

  it('applies active class when active', () => {
    const { container } = render(
      <PlanCard
        plan={basePlan}
        isActive={true}
        onSelect={vi.fn()}
        onDelete={vi.fn()}
      />,
    );
    expect(container.firstChild).toHaveClass('active');
  });

  it('calls onSelect on click', () => {
    const onSelect = vi.fn();
    render(
      <PlanCard
        plan={basePlan}
        isActive={false}
        onSelect={onSelect}
        onDelete={vi.fn()}
      />,
    );
    fireEvent.click(screen.getByText('Learn Python'));
    expect(onSelect).toHaveBeenCalled();
  });

  it('renders delete button', () => {
    render(
      <PlanCard
        plan={basePlan}
        isActive={false}
        onSelect={vi.fn()}
        onDelete={vi.fn()}
      />,
    );
    expect(screen.getByLabelText(/delete/i)).toBeInTheDocument();
  });
});

import { fireEvent } from '@testing-library/react';
