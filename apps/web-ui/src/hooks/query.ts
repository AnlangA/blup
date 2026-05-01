import { useEffect, useReducer, useCallback, useRef } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { api } from "../api/client";
import { sseClient } from "../api/sse";
import type {
  LearningGoal,
  ProfileAnswer,
  SessionSnapshot,
  SessionListEntry,
  CurriculumPlan,
  ChapterContent,
} from "../api/client";
import { useSessionStore } from "../state/sessionStore";

// ── Plans ──

export function useCreatePlan() {
  const addPlan = useSessionStore((s) => s.addPlan);

  return useMutation({
    mutationFn: () => api.createSession(),
    onSuccess: (data) => {
      addPlan({
        id: data.session_id,
        title: "Untitled",
        domain: "",
        state: data.state,
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      });
    },
  });
}

export function useDeletePlan() {
  const removePlan = useSessionStore((s) => s.removePlan);
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (planId: string) => api.deleteSession(planId),
    onSuccess: (_data, planId) => {
      queryClient.removeQueries({ queryKey: ["session", planId] });
      queryClient.removeQueries({ queryKey: ["curriculum", planId] });
      removePlan(planId);
    },
  });
}

export function useSyncPlansFromServer() {
  const updatePlanMeta = useSessionStore((s) => s.updatePlanMeta);

  return useQuery<SessionListEntry[]>({
    queryKey: ["sessions"],
    queryFn: async () => {
      const entries = await api.listSessions();
      // Read plans from store directly (not from closure) to avoid stale data
      const currentPlans = useSessionStore.getState().plans;
      for (const entry of entries) {
        const existing = currentPlans.find((p) => p.id === entry.id);
        if (existing) {
          if (
            existing.state !== entry.state ||
            existing.title !== entry.goal_description ||
            existing.domain !== entry.domain
          ) {
            updatePlanMeta(entry.id, {
              state: entry.state,
              title: entry.goal_description || existing.title,
              domain: entry.domain || existing.domain,
              updatedAt: entry.updated_at,
            });
          }
        }
      }
      return entries;
    },
    refetchOnWindowFocus: false,
    staleTime: 60_000,
  });
}

// ── Session ──

export function useSession(sessionId: string | null) {
  return useQuery<SessionSnapshot>({
    queryKey: ["session", sessionId],
    queryFn: () => api.getSession(sessionId!),
    enabled: !!sessionId,
    staleTime: 30_000,
  });
}

/** Sync plan metadata from session data whenever session query updates. */
export function useSessionPlanSync(
  sessionId: string | null,
  session: SessionSnapshot | undefined,
) {
  const updatePlanMeta = useSessionStore((s) => s.updatePlanMeta);

  useEffect(() => {
    if (!sessionId || !session) return;
    const goalDesc =
      (session.goal as Record<string, unknown> | null)?.description;
    const goalDomain =
      (session.goal as Record<string, unknown> | null)?.domain;
    const newTitle = (goalDesc as string) || "Untitled";
    const newDomain = (goalDomain as string) || "";
    const current = useSessionStore
      .getState()
      .plans.find((p) => p.id === sessionId);
    if (
      current &&
      (current.state !== session.state ||
        current.title !== newTitle ||
        current.domain !== newDomain)
    ) {
      updatePlanMeta(sessionId, {
        state: session.state,
        title: newTitle,
        domain: newDomain,
      });
    }
  }, [sessionId, session, updatePlanMeta]);
}

// ── Goal ──

export function useSubmitGoal(sessionId: string | null) {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (goal: LearningGoal) => api.submitGoal(sessionId!, goal),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["session", sessionId] });
    },
  });
}

interface GoalStreamState {
  status: string | null;
  message: string | null;
  isStreaming: boolean;
  error: string | null;
  result: unknown | null;
}

type GoalStreamAction =
  | { type: "reset" }
  | { type: "status"; state: string; message: string }
  | { type: "done"; result: unknown }
  | { type: "error"; message: string };

function goalStreamReducer(
  state: GoalStreamState,
  action: GoalStreamAction,
): GoalStreamState {
  switch (action.type) {
    case "reset":
      return {
        status: null,
        message: null,
        isStreaming: true,
        error: null,
        result: null,
      };
    case "status":
      return { ...state, status: action.state, message: action.message };
    case "done":
      return { ...state, isStreaming: false, result: action.result };
    case "error":
      return { ...state, isStreaming: false, error: action.message };
  }
}

export function useSubmitGoalStream(sessionId: string | null) {
  const queryClient = useQueryClient();
  const [state, dispatch] = useReducer(goalStreamReducer, {
    status: null,
    message: null,
    isStreaming: false,
    error: null,
    result: null,
  });
  const sseRef = useRef(sseClient);

  const submit = useCallback(
    (goal: LearningGoal) => {
      if (!sessionId) return;

      sseRef.current.close();
      dispatch({ type: "reset" });

      const url = `/api/session/${sessionId}/goal/stream`;
      sseRef.current.connectPost(url, goal, {
        onStatus: (st, msg) => dispatch({ type: "status", state: st, message: msg }),
        onDone: (result) => {
          dispatch({ type: "done", result });
          queryClient.invalidateQueries({ queryKey: ["session", sessionId] });
        },
        onError: (_code, message) => {
          dispatch({ type: "error", message });
          queryClient.invalidateQueries({ queryKey: ["session", sessionId] });
        },
      });
    },
    [sessionId, queryClient],
  );

  const reset = useCallback(() => {
    sseRef.current.close();
    dispatch({ type: "reset" });
  }, []);

  useEffect(() => {
    const client = sseRef.current;
    return () => {
      client.close();
    };
  }, []);

  return { ...state, submit, reset };
}

// ── Profile ──

export function useSubmitProfile(sessionId: string | null) {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (answer: ProfileAnswer) =>
      api.submitProfileAnswer(sessionId!, answer),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["session", sessionId] });
    },
  });
}

// ── Curriculum ──

export function useCurriculum(sessionId: string | null) {
  return useQuery<CurriculumPlan>({
    queryKey: ["curriculum", sessionId],
    queryFn: () => api.getCurriculum(sessionId!),
    enabled: !!sessionId,
    staleTime: Infinity,
  });
}

// ── Chapter ──

export function useChapter(sessionId: string | null, chapterId: string | null) {
  return useQuery<ChapterContent>({
    queryKey: ["chapter", sessionId, chapterId],
    queryFn: () => api.startChapter(sessionId!, chapterId!),
    enabled: !!sessionId && !!chapterId,
    staleTime: Infinity,
  });
}

export function usePrefetchChapters(
  sessionId: string | null,
  chapterIds: string[],
) {
  const queryClient = useQueryClient();

  return {
    prefetchAll: () => {
      for (const chId of chapterIds) {
        queryClient.prefetchQuery({
          queryKey: ["chapter", sessionId, chId],
          queryFn: () => api.startChapter(sessionId!, chId),
          staleTime: Infinity,
        });
      }
    },
  };
}

// ── Q&A ──

export function useAskQuestion(
  sessionId: string | null,
  chapterId: string | null,
) {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (question: string) =>
      api.askQuestion(sessionId!, chapterId!, question),
    onMutate: async (question) => {
      // Optimistically add the user message to the cached session
      await queryClient.cancelQueries({ queryKey: ["session", sessionId] });
      const previous = queryClient.getQueryData<SessionSnapshot>([
        "session",
        sessionId,
      ]);
      if (previous) {
        const optimisticMessage = {
          id: `optimistic-${Date.now()}`,
          role: "user",
          content: question,
          timestamp: new Date().toISOString(),
          chapter_id: chapterId ?? undefined,
        };
        queryClient.setQueryData<SessionSnapshot>(["session", sessionId], {
          ...previous,
          messages: [...previous.messages, optimisticMessage],
        });
      }
      return { previous };
    },
    onError: (_err, _question, context) => {
      // Roll back to the previous state on error
      if (context?.previous) {
        queryClient.setQueryData(["session", sessionId], context.previous);
      }
    },
    onSettled: () => {
      queryClient.invalidateQueries({ queryKey: ["session", sessionId] });
    },
  });
}

// ── Chapter Complete ──

export function useCompleteChapter(sessionId: string | null) {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (chapterId: string) =>
      api.completeChapter(sessionId!, chapterId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["session", sessionId] });
    },
  });
}
