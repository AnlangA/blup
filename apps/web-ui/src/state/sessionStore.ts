import { create } from 'zustand';

export type CodeTheme = 'github-dark' | 'github-light';

interface SessionStore {
  sessionId: string | null;
  currentChapterId: string | null;
  codeTheme: CodeTheme;

  setSession: (id: string) => void;
  setChapter: (chapterId: string) => void;
  setCodeTheme: (theme: CodeTheme) => void;
  reset: () => void;
}

export const useSessionStore = create<SessionStore>((set) => ({
  sessionId: localStorage.getItem('blup_session_id'),
  currentChapterId: null,
  codeTheme: 'github-dark',

  setSession: (id) => {
    localStorage.setItem('blup_session_id', id);
    set({ sessionId: id });
  },

  setChapter: (chapterId) => set({ currentChapterId: chapterId }),

  setCodeTheme: (codeTheme) => {
    document.documentElement.setAttribute('data-theme', codeTheme);
    set({ codeTheme });
  },

  reset: () => {
    localStorage.removeItem('blup_session_id');
    set({ sessionId: null, currentChapterId: null, codeTheme: 'github-dark' });
  },
}));
