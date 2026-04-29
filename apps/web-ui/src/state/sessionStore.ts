import { create } from "zustand";
import { api, ApiError } from "../api/client";

export type SessionState =
  | "IDLE"
  | "GOAL_INPUT"
  | "FEASIBILITY_CHECK"
  | "PROFILE_COLLECTION"
  | "CURRICULUM_PLANNING"
  | "CHAPTER_LEARNING"
  | "COMPLETED"
  | "ERROR";

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

export type CodeTheme = "github-dark" | "github-light";

interface SessionStore {
  sessionId: string | null;
  state: SessionState;
  error: ApiError | null;
  goal: { description: string; domain: string; context?: string } | null;
  feasibility: FeasibilityData | null;
  profile: Record<string, unknown> | null;
  chapters: Chapter[];
  currentChapterId: string | null;
  chapterContent: string | null;
  chapterCache: Record<string, string>;
  chapterLoading: Record<string, boolean>;
  chapterChatMessages: Record<
    string,
    Array<{ id: string; role: string; content: string; timestamp: string }>
  >;
  messages: Array<{
    id: string;
    role: string;
    content: string;
    timestamp: string;
  }>;
  codeTheme: CodeTheme;

  createSession: () => Promise<void>;
  restoreSession: () => Promise<boolean>;
  submitGoal: (goal: {
    description: string;
    domain: string;
    context?: string;
  }) => Promise<void>;
  setState: (state: SessionState) => void;
  setChapter: (chapterId: string) => void;
  setChapterContent: (content: string | null) => void;
  setChapterCache: (chapterId: string, content: string) => void;
  setChapterLoading: (chapterId: string, loading: boolean) => void;
  setChapters: (chapters: Chapter[]) => void;
  addChatMessage: (
    chapterId: string,
    message: { id: string; role: string; content: string; timestamp: string },
  ) => void;
  setCodeTheme: (theme: CodeTheme) => void;
  reset: () => void;
}

export const useSessionStore = create<SessionStore>((set, get) => ({
  sessionId: localStorage.getItem("blup_session_id"),
  state: "IDLE",
  error: null,
  goal: null,
  feasibility: null,
  profile: null,
  chapters: [],
  currentChapterId: null,
  chapterContent: null,
  chapterCache: {},
  chapterLoading: {},
  chapterChatMessages: {},
  messages: [],
  codeTheme: "github-dark",

  createSession: async () => {
    try {
      set({ error: null });
      const resp = await api.createSession();
      localStorage.setItem("blup_session_id", resp.session_id);
      set({ sessionId: resp.session_id, state: "IDLE", error: null });
    } catch (err: unknown) {
      console.error("Failed to create session:", err);
      localStorage.removeItem("blup_session_id");
      set({ sessionId: null, error: err as ApiError, state: "ERROR" });
    }
  },

  restoreSession: async () => {
    const { sessionId } = get();
    if (!sessionId) return false;
    try {
      const snapshot = await api.getSession(sessionId);
      // Extract chapters from curriculum if available
      const chapters = (snapshot.curriculum?.chapters || []) as Chapter[];
      set({
        state: snapshot.state as SessionState,
        goal: snapshot.goal as {
          description: string;
          domain: string;
          context?: string;
        } | null,
        feasibility:
          (snapshot.feasibility_result as unknown as FeasibilityData) || null,
        profile: snapshot.profile || null,
        chapters,
        currentChapterId: snapshot.current_chapter_id,
        chapterContent: snapshot.current_chapter_id
          ? snapshot.chapter_contents[snapshot.current_chapter_id] || null
          : null,
        chapterCache: snapshot.chapter_contents || {},
        messages: snapshot.messages || [],
        error: null,
      });
      return true;
    } catch (err: unknown) {
      console.error("[restoreSession] Failed to restore session:", err);
      const apiError = err as ApiError;
      // Session not found on backend - clear stale localStorage
      if (apiError.code === "NOT_FOUND") {
        localStorage.removeItem("blup_session_id");
        set({ sessionId: null, state: "IDLE" as SessionState });
      }
      return false;
    }
  },

  submitGoal: async (goal) => {
    const { sessionId } = get();
    console.log("[submitGoal] Starting, sessionId:", sessionId);
    if (!sessionId) {
      console.error("No session ID, cannot submit goal");
      set({
        error: {
          code: "NO_SESSION",
          message: "No active session. Please refresh the page.",
        },
        state: "ERROR",
      });
      return;
    }
    try {
      console.log("[submitGoal] Setting state to FEASIBILITY_CHECK");
      set({ goal, state: "FEASIBILITY_CHECK", error: null });
      console.log("[submitGoal] Calling API...");
      const result = await api.submitGoal(sessionId, goal);
      console.log("[submitGoal] API response:", result);
      set({
        feasibility: result.feasibility as FeasibilityData,
        state: "FEASIBILITY_CHECK",
      });
      console.log("[submitGoal] State updated successfully");
    } catch (err: unknown) {
      console.error("[submitGoal] Error:", err);
      const apiError = err as ApiError;
      if (apiError.code === "NOT_FOUND") {
        localStorage.removeItem("blup_session_id");
        set({ sessionId: null, error: apiError, state: "ERROR" });
      } else {
        set({ error: apiError, state: "ERROR" });
      }
    }
  },

  setState: (state) => set({ state }),

  setChapter: (chapterId) => set({ currentChapterId: chapterId }),

  setChapterContent: (content) => set({ chapterContent: content }),

  setChapterCache: (chapterId, content) => {
    const { chapterCache } = get();
    set({ chapterCache: { ...chapterCache, [chapterId]: content } });
  },

  setChapterLoading: (chapterId, loading) => {
    const { chapterLoading } = get();
    set({ chapterLoading: { ...chapterLoading, [chapterId]: loading } });
  },

  setChapters: (chapters) => set({ chapters }),

  addChatMessage: (chapterId, message) => {
    const { chapterChatMessages } = get();
    const existing = chapterChatMessages[chapterId] || [];
    set({
      chapterChatMessages: {
        ...chapterChatMessages,
        [chapterId]: [...existing, message],
      },
    });
  },

  setCodeTheme: (codeTheme) => {
    document.documentElement.setAttribute("data-theme", codeTheme);
    set({ codeTheme });
  },

  reset: () => {
    localStorage.removeItem("blup_session_id");
    set({
      sessionId: null,
      state: "IDLE",
      error: null,
      goal: null,
      feasibility: null,
      profile: null,
      chapters: [],
      currentChapterId: null,
      chapterContent: null,
      chapterCache: {},
      chapterLoading: {},
      chapterChatMessages: {},
      messages: [],
      codeTheme: "github-dark",
    });
  },
}));
