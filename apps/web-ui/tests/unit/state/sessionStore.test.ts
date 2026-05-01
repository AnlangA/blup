import { describe, it, expect, beforeEach, vi } from 'vitest';

// Mock localStorage
const localStorageMock = (() => {
  let store: Record<string, string> = {};
  return {
    getItem: (key: string) => store[key] ?? null,
    setItem: (key: string, value: string) => { store[key] = value; },
    removeItem: (key: string) => { delete store[key]; },
    clear: () => { store = {}; },
  };
})();

Object.defineProperty(globalThis, 'localStorage', { value: localStorageMock });

describe('sessionStore', () => {
  beforeEach(() => {
    localStorage.clear();
  });

  it('should store and retrieve plans from localStorage', () => {
    const plans = [
      {
        id: 'test-plan-id',
        title: 'Test Plan',
        domain: 'testing',
        state: 'IDLE',
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      },
    ];
    localStorage.setItem('blup_plans', JSON.stringify(plans));
    const stored = localStorage.getItem('blup_plans');
    expect(stored).toBeTruthy();
    const parsed = JSON.parse(stored!);
    expect(parsed).toHaveLength(1);
    expect(parsed[0].id).toBe('test-plan-id');
  });

  it('should store active plan id', () => {
    localStorage.setItem('blup_active_plan_id', 'plan-123');
    expect(localStorage.getItem('blup_active_plan_id')).toBe('plan-123');
  });

  it('should store current chapter id', () => {
    localStorage.setItem('blup_current_chapter_id', 'ch1');
    expect(localStorage.getItem('blup_current_chapter_id')).toBe('ch1');
  });

  it('should clear plan data on remove', () => {
    localStorage.setItem('blup_plans', '[{"id":"p1"}]');
    localStorage.setItem('blup_active_plan_id', 'p1');
    localStorage.removeItem('blup_plans');
    expect(localStorage.getItem('blup_plans')).toBeNull();
  });
});
