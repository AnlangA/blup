import { useEffect, useReducer, useCallback, useRef } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { api, downloadBlob } from "../api/client";
import { sseClient } from "../api/sse";
import type {
  LearningGoal,
  ProfileAnswer,
  SessionSnapshot,
  SessionListEntry,
  CurriculumPlan,
  ChapterContent,
  ExportResult,
  SandboxExecuteRequest,
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
      queryClient.removeQueries({ queryKey: ["sessions"] });
      removePlan(planId);
    },
    // If the backend cannot find the session (404), it's already gone —
    // clean up local state so the sidebar doesn't show a stale entry.
    onError: (_err, planId) => {
      queryClient.removeQueries({ queryKey: ["session", planId] });
      queryClient.removeQueries({ queryKey: ["curriculum", planId] });
      queryClient.removeQueries({ queryKey: ["sessions"] });
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
    refetchOnWindowFocus: true,
    refetchInterval: 30_000,
    staleTime: 15_000,
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
      queryClient.invalidateQueries({ queryKey: ["sessions"] });
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
          queryClient.invalidateQueries({ queryKey: ["sessions"] });
        },
        onError: (_code, message) => {
          dispatch({ type: "error", message });
          queryClient.invalidateQueries({ queryKey: ["session", sessionId] });
          queryClient.invalidateQueries({ queryKey: ["sessions"] });
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
      queryClient.invalidateQueries({ queryKey: ["sessions"] });
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
      queryClient.invalidateQueries({ queryKey: ["sessions"] });
    },
  });
}

// ── Export (Typst - sync mutation) ──

export function useExportChapterTypst(
  sessionId: string | null,
  chapterId: string | null,
) {
  return useMutation({
    mutationFn: () => api.exportChapterTypst(sessionId!, chapterId!),
    onSuccess: (data: ExportResult) => {
      if (data.typst_source) {
        void downloadBlob(
          data.typst_source,
          data.filename || "chapter.typ",
          "text/plain",
        );
      }
    },
  });
}

export function useExportCurriculumTypst(sessionId: string | null) {
  return useMutation({
    mutationFn: () => api.exportCurriculumTypst(sessionId!),
    onSuccess: (data: ExportResult) => {
      if (data.typst_source) {
        void downloadBlob(
          data.typst_source,
          data.filename || "curriculum.typ",
          "text/plain",
        );
      }
    },
  });
}

// ── Export (PDF - SSE streaming) ──

interface PdfExportState {
  status: string | null;
  message: string | null;
  isExporting: boolean;
  error: string | null;
  result: ExportResult | null;
}

type PdfExportAction =
  | { type: "reset" }
  | { type: "status"; state: string; message: string }
  | { type: "done"; result: ExportResult }
  | { type: "error"; message: string };

function pdfExportReducer(
  state: PdfExportState,
  action: PdfExportAction,
): PdfExportState {
  switch (action.type) {
    case "reset":
      return {
        status: null,
        message: null,
        isExporting: true,
        error: null,
        result: null,
      };
    case "status":
      return { ...state, status: action.state, message: action.message };
    case "done":
      return { ...state, isExporting: false, result: action.result };
    case "error":
      return { ...state, isExporting: false, error: action.message };
  }
}

export function useExportChapterPdf(
  sessionId: string | null,
  chapterId: string | null,
) {
  const [state, dispatch] = useReducer(pdfExportReducer, {
    status: null,
    message: null,
    isExporting: false,
    error: null,
    result: null,
  });
  const sseRef = useRef(sseClient);

  const exportPdf = useCallback(() => {
    if (!sessionId || !chapterId) return;

    sseRef.current.close();
    dispatch({ type: "reset" });

    const url = `/api/session/${sessionId}/export/chapter/${chapterId}/pdf`;
    sseRef.current.connectPost(url, {}, {
      onStatus: (st, msg) =>
        dispatch({ type: "status", state: st, message: msg }),
      onDone: (result) => {
        const r = result as ExportResult;
        dispatch({ type: "done", result: r });
        if (r.pdf_base64) {
          const byteChars = atob(r.pdf_base64);
          const byteNums = new Array(byteChars.length);
          for (let i = 0; i < byteChars.length; i++) {
            byteNums[i] = byteChars.charCodeAt(i);
          }
          const pdfBytes = new Uint8Array(byteNums);
          void downloadBlob(
            new Blob([pdfBytes], { type: "application/pdf" }),
            r.filename || "chapter.pdf",
            "application/pdf",
          );
        }
      },
      onError: (_code, message) => {
        dispatch({ type: "error", message });
      },
    });
  }, [sessionId, chapterId]);

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

  return { ...state, exportPdf, reset };
}

export function useExportCurriculumPdf(sessionId: string | null) {
  const [state, dispatch] = useReducer(pdfExportReducer, {
    status: null,
    message: null,
    isExporting: false,
    error: null,
    result: null,
  });
  const sseRef = useRef(sseClient);

  const exportPdf = useCallback(() => {
    if (!sessionId) return;

    sseRef.current.close();
    dispatch({ type: "reset" });

    const url = `/api/session/${sessionId}/export/curriculum/pdf`;
    sseRef.current.connectPost(url, {}, {
      onStatus: (st, msg) =>
        dispatch({ type: "status", state: st, message: msg }),
      onDone: (result) => {
        const r = result as ExportResult;
        dispatch({ type: "done", result: r });
        if (r.pdf_base64) {
          const byteChars = atob(r.pdf_base64);
          const byteNums = new Array(byteChars.length);
          for (let i = 0; i < byteChars.length; i++) {
            byteNums[i] = byteChars.charCodeAt(i);
          }
          const pdfBytes = new Uint8Array(byteNums);
          void downloadBlob(
            new Blob([pdfBytes], { type: "application/pdf" }),
            r.filename || "curriculum.pdf",
            "application/pdf",
          );
        }
      },
      onError: (_code, message) => {
        dispatch({ type: "error", message });
      },
    });
  }, [sessionId]);

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

  return { ...state, exportPdf, reset };
}

// ── Sandbox Execution ──

interface SandboxState {
  stdout: string;
  stderr: string;
  status: string | null;
  message: string | null;
  isRunning: boolean;
  error: string | null;
  exitCode: number | null;
  durationMs: number | null;
}

type SandboxAction =
  | { type: "reset" }
  | { type: "executing" }
  | { type: "status"; state: string; message: string }
  | { type: "stdout"; content: string }
  | { type: "stderr"; content: string }
  | { type: "done"; exitCode: number | null; durationMs: number | null }
  | { type: "error"; message: string };

function sandboxReducer(
  state: SandboxState,
  action: SandboxAction,
): SandboxState {
  switch (action.type) {
    case "reset":
      return {
        stdout: "",
        stderr: "",
        status: null,
        message: null,
        isRunning: false,
        error: null,
        exitCode: null,
        durationMs: null,
      };
    case "executing":
      return {
        stdout: "",
        stderr: "",
        status: null,
        message: null,
        isRunning: true,
        error: null,
        exitCode: null,
        durationMs: null,
      };
    case "status":
      return { ...state, status: action.state, message: action.message };
    case "stdout":
      return { ...state, stdout: state.stdout + action.content };
    case "stderr":
      return { ...state, stderr: state.stderr + action.content };
    case "done":
      return {
        ...state,
        isRunning: false,
        exitCode: action.exitCode,
        durationMs: action.durationMs,
      };
    case "error":
      return { ...state, isRunning: false, error: action.message };
  }
}

export function useSandboxExecute() {
  const [state, dispatch] = useReducer(sandboxReducer, {
    stdout: "",
    stderr: "",
    status: null,
    message: null,
    isRunning: false,
    error: null,
    exitCode: null,
    durationMs: null,
  });
  const sseRef = useRef(sseClient);

  const execute = useCallback((req: SandboxExecuteRequest) => {
    sseRef.current.close();
    dispatch({ type: "executing" });

    sseRef.current.connectPost("/api/sandbox/execute", req, {
      onStatus: (st, msg) =>
        dispatch({ type: "status", state: st, message: msg }),
      onStdout: (content) => dispatch({ type: "stdout", content }),
      onStderr: (content) => dispatch({ type: "stderr", content }),
      onDone: (result) => {
        const r = result as { exit_code: number | null; duration_ms: number | null };
        dispatch({
          type: "done",
          exitCode: r.exit_code,
          durationMs: r.duration_ms,
        });
      },
      onError: (_code, message) => {
        dispatch({ type: "error", message });
      },
    });
  }, []);

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

  return { ...state, execute, reset };
}
