import { create } from 'zustand';
import { api, ApiError } from '../api/client';

export type SessionState =
  | 'IDLE'
  | 'GOAL_INPUT'
  | 'FEASIBILITY_CHECK'
  | 'PROFILE_COLLECTION'
  | 'CURRICULUM_PLANNING'
  | 'CHAPTER_LEARNING'
  | 'COMPLETED'
  | 'ERROR';

export interface Chapter {
  id: string;
  title: string;
  order: number;
  objectives: string[];
}

export interface FeasibilityData {
  feasible: boolean;
  reason: string;
  suggestions: string[];
  estimated_duration?: string;
  prerequisites: string[];
}

interface SessionStore {
  sessionId: string | null;
  state: SessionState;
  error: ApiError | null;
  goal: { description: string; domain: string; context?: string } | null;
  feasibility: FeasibilityData | null;
  profile: Record<string, unknown> | null;
  chapters: Chapter[];
  currentChapterId: string | null;
  messages: Array<{ id: string; role: string; content: string; timestamp: string }>;

  createSession: () => Promise<void>;
  submitGoal: (goal: { description: string; domain: string; context?: string }) => Promise<void>;
  setState: (state: SessionState) => void;
  setChapter: (chapterId: string) => void;
  setChapters: (chapters: Chapter[]) => void;
  reset: () => void;
}

export const useSessionStore = create<SessionStore>((set, get) => ({
  sessionId: localStorage.getItem('blup_session_id'),
  state: 'IDLE',
  error: null,
  goal: null,
  feasibility: null,
  profile: null,
  chapters: [],
  currentChapterId: null,
  messages: [],

  createSession: async () => {
    try {
      set({ error: null });
      const resp = await api.createSession();
      localStorage.setItem('blup_session_id', resp.session_id);
      set({ sessionId: resp.session_id, state: 'IDLE', error: null });
    } catch (err: unknown) {
      console.error('Failed to create session:', err);
      localStorage.removeItem('blup_session_id');
      set({ sessionId: null, error: err as ApiError, state: 'ERROR' });
    }
  },

  submitGoal: async (goal) => {
    const { sessionId } = get();
    if (!sessionId) {
      console.error('No session ID, cannot submit goal');
      set({ error: { code: 'NO_SESSION', message: 'No active session. Please refresh the page.' }, state: 'ERROR' });
      return;
    }
    try {
      set({ goal, state: 'FEASIBILITY_CHECK', error: null });
      const result = await api.submitGoal(sessionId, goal);
      set({
        feasibility: result.feasibility as FeasibilityData,
        state: 'FEASIBILITY_CHECK',
      });
    } catch (err: unknown) {
      console.error('Failed to submit goal:', err);
      const apiError = err as ApiError;
      if (apiError.code === 'NOT_FOUND') {
        localStorage.removeItem('blup_session_id');
        set({ sessionId: null, error: apiError, state: 'ERROR' });
      } else {
        set({ error: apiError, state: 'ERROR' });
      }
    }
  },

  setState: (state) => set({ state }),

  setChapter: (chapterId) => set({ currentChapterId: chapterId }),

  setChapters: (chapters) => set({ chapters }),

  reset: () => {
    localStorage.removeItem('blup_session_id');
    set({
      sessionId: null,
      state: 'IDLE',
      error: null,
      goal: null,
      feasibility: null,
      profile: null,
      chapters: [],
      currentChapterId: null,
      messages: [],
    });
  },
}));
