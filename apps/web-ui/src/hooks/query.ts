import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { api } from '../api/client';
import type {
  LearningGoal,
  ProfileAnswer,
  SessionSnapshot,
  CurriculumPlan,
  ChapterContent,
} from '../api/client';
import { useSessionStore } from '../state/sessionStore';

// ── Session ──

export function useCreateSession() {
  const setSession = useSessionStore((s) => s.setSession);

  return useMutation({
    mutationFn: () => api.createSession(),
    onSuccess: (data) => {
      localStorage.setItem('blup_session_id', data.session_id);
      setSession(data.session_id);
    },
  });
}

export function useSession(sessionId: string | null) {
  return useQuery<SessionSnapshot>({
    queryKey: ['session', sessionId],
    queryFn: () => api.getSession(sessionId!),
    enabled: !!sessionId,
    staleTime: 30_000,
  });
}

// ── Goal ──

export function useSubmitGoal(sessionId: string | null) {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (goal: LearningGoal) => api.submitGoal(sessionId!, goal),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['session', sessionId] });
    },
  });
}

// ── Profile ──

export function useSubmitProfile(sessionId: string | null) {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (answer: ProfileAnswer) =>
      api.submitProfileAnswer(sessionId!, answer),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['session', sessionId] });
    },
  });
}

// ── Curriculum ──

export function useCurriculum(sessionId: string | null) {
  return useQuery<CurriculumPlan>({
    queryKey: ['curriculum', sessionId],
    queryFn: () => api.getCurriculum(sessionId!),
    enabled: !!sessionId,
    staleTime: Infinity,
  });
}

// ── Chapter ──

export function useChapter(
  sessionId: string | null,
  chapterId: string | null,
) {
  return useQuery<ChapterContent>({
    queryKey: ['chapter', sessionId, chapterId],
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
          queryKey: ['chapter', sessionId, chId],
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
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['session', sessionId] });
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
      queryClient.invalidateQueries({ queryKey: ['session', sessionId] });
    },
  });
}
