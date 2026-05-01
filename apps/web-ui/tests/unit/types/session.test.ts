import { describe, it, expect } from 'vitest';
import { parseSession } from '../../../src/types/session';
import { checkShape } from '../../../src/types/validate';

describe('parseSession', () => {
  it('parses a full session snapshot', () => {
    const raw = {
      session_id: 'abc-123',
      state: 'CHAPTER_LEARNING',
      goal: { description: 'Learn Rust', domain: 'programming' },
      feasibility_result: {
        feasible: true,
        reason: 'Good goal',
        suggestions: [],
        prerequisites: ['basic CS'],
      },
      profile: {
        experience_level: { domain_knowledge: 'beginner' },
        learning_style: { preferred_format: ['text'] },
        available_time: { hours_per_week: 10 },
      },
      profile_rounds: 3,
      curriculum: {
        title: 'Rust Fundamentals',
        chapters: [
          { id: 'ch1', title: 'Hello World', order: 1, objectives: ['Print hello'] },
        ],
        estimated_duration: '4 weeks',
      },
      current_chapter_id: 'ch1',
      chapter_contents: { ch1: 'Hello World content...' },
      messages: [
        { id: 'm1', role: 'user', content: 'Hi', timestamp: '2024-01-01T00:00:00Z' },
      ],
    };

    const result = parseSession(raw);
    expect(result.sessionId).toBe('abc-123');
    expect(result.state).toBe('CHAPTER_LEARNING');
    expect(result.goal?.description).toBe('Learn Rust');
    expect(result.feasibilityResult?.feasible).toBe(true);
    expect(result.profile?.experienceLevel?.domainKnowledge).toBe('beginner');
    expect(result.profileRounds).toBe(3);
    expect(result.curriculum?.chapters).toHaveLength(1);
    expect(result.currentChapterId).toBe('ch1');
    expect(result.messages).toHaveLength(1);
  });

  it('handles null fields gracefully', () => {
    const raw = {
      session_id: 'empty-session',
      state: 'IDLE',
      goal: null,
      feasibility_result: null,
      profile: null,
      profile_rounds: 0,
      curriculum: null,
      current_chapter_id: null,
      chapter_contents: {},
      messages: [],
    };

    const result = parseSession(raw);
    expect(result.sessionId).toBe('empty-session');
    expect(result.state).toBe('IDLE');
    expect(result.goal).toBeNull();
    expect(result.feasibilityResult).toBeNull();
    expect(result.profile).toBeNull();
    expect(result.curriculum).toBeNull();
    expect(result.messages).toHaveLength(0);
  });

  it('handles missing fields with defaults', () => {
    const result = parseSession({});
    expect(result.sessionId).toBe('');
    expect(result.state).toBe('IDLE');
    expect(result.goal).toBeNull();
    expect(result.profileRounds).toBe(0);
    expect(result.chapterContents).toEqual({});
    expect(result.messages).toEqual([]);
  });
});

// ── Cross-module contract tests: fixture data shapes ──

describe('contract shapes', () => {
  it('validates a full session fixture against checkShape', () => {
    const session = {
      sessionId: 'abc-123',
      state: 'CHAPTER_LEARNING',
      goal: { description: 'Learn Rust', domain: 'programming' },
      feasibilityResult: { feasible: true, reason: 'Good', suggestions: [], prerequisites: [] },
      profile: {
        experienceLevel: { domainKnowledge: 'beginner' },
        learningStyle: { preferredFormat: ['text'] },
        availableTime: { hoursPerWeek: 5 },
      },
      profileRounds: 3,
      curriculum: {
        title: 'Rust',
        chapters: [{ id: 'ch1', title: 'Intro', order: 1, objectives: ['a'] }],
        estimatedDuration: '4 weeks',
      },
      currentChapterId: 'ch1',
      chapterContents: {},
      messages: [],
    };

    const warnings = checkShape('session', session, {
      sessionId: { type: 'string' },
      state: { type: 'string' },
      goal: { type: 'object', optional: true },
      feasibilityResult: { type: 'object', optional: true },
      profile: { type: 'object', optional: true },
      profileRounds: { type: 'number' },
      curriculum: { type: 'object', optional: true },
      currentChapterId: { type: 'string', optional: true },
      chapterContents: { type: 'object' },
      messages: { type: 'array' },
    });

    expect(warnings).toEqual([]);
  });

  it('detects missing required fields', () => {
    const warnings = checkShape('session', {}, {
      sessionId: { type: 'string' },
      state: { type: 'string' },
      chapterContents: { type: 'object' },
      messages: { type: 'array' },
    });

    expect(warnings.length).toBeGreaterThan(0);
    expect(warnings.some((w) => w.includes('sessionId'))).toBe(true);
    expect(warnings.some((w) => w.includes('state'))).toBe(true);
  });
});
