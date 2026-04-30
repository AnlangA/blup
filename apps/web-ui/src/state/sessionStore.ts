import { create } from "zustand";

export type CodeTheme = "github-dark" | "github-light";

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
  sessionId: localStorage.getItem("blup_session_id"),
  currentChapterId: localStorage.getItem("blup_current_chapter_id"),
  codeTheme:
    (localStorage.getItem("blup_code_theme") as CodeTheme) || "github-dark",

  setSession: (id) => {
    localStorage.setItem("blup_session_id", id);
    set({ sessionId: id });
  },

  setChapter: (chapterId) => {
    localStorage.setItem("blup_current_chapter_id", chapterId);
    set({ currentChapterId: chapterId });
  },

  setCodeTheme: (codeTheme) => {
    localStorage.setItem("blup_code_theme", codeTheme);
    document.documentElement.setAttribute("data-theme", codeTheme);
    set({ codeTheme });
  },

  reset: () => {
    localStorage.removeItem("blup_session_id");
    localStorage.removeItem("blup_current_chapter_id");
    localStorage.removeItem("blup_code_theme");
    set({ sessionId: null, currentChapterId: null, codeTheme: "github-dark" });
  },
}));

// Apply theme on initial load
const initialTheme =
  (localStorage.getItem("blup_code_theme") as CodeTheme) || "github-dark";
document.documentElement.setAttribute("data-theme", initialTheme);
