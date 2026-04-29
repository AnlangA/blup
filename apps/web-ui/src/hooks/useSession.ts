import { useState, useCallback, useRef, useEffect } from 'react';
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

const SESSION_KEY = 'blup_session_id';

export function useSession() {
  const [sessionId, setSessionId] = useState<string | null>(() => localStorage.getItem(SESSION_KEY));
  const [state, setState] = useState<SessionState>('IDLE');
  const [error, setError] = useState<ApiError | null>(null);
  const initializingRef = useRef(false);

  const createSession = useCallback(async () => {
    if (initializingRef.current) return;
    initializingRef.current = true;
    try {
      const response = await api.createSession();
      setSessionId(response.session_id);
      setState(response.state as SessionState);
      setError(null);
      localStorage.setItem(SESSION_KEY, response.session_id);
    } catch (err: unknown) {
      setError(err as ApiError);
      setState('ERROR');
    } finally {
      initializingRef.current = false;
    }
  }, []);

  useEffect(() => {
    if (!sessionId && !initializingRef.current) {
      createSession();
    }
  }, [sessionId, createSession]);

  const submitGoal = useCallback(async (goal: { description: string; domain: string; context?: string }) => {
    if (!sessionId) return;
    try {
      setState('FEASIBILITY_CHECK');
      const result = await api.submitGoal(sessionId, goal);
      setState('PROFILE_COLLECTION');
      return result;
    } catch (err: unknown) {
      setError(err as ApiError);
      setState('ERROR');
    }
  }, [sessionId]);

  const answerProfile = useCallback(async (questionId: string, answer: string) => {
    if (!sessionId) return;
    try {
      const result = await api.submitProfileAnswer(sessionId, { question_id: questionId, answer });
      if (result.is_complete) {
        setState('CURRICULUM_PLANNING');
      }
      return result;
    } catch (err: unknown) {
      setError(err as ApiError);
      setState('ERROR');
    }
  }, [sessionId]);

  const getCurriculum = useCallback(async () => {
    if (!sessionId) return;
    try {
      const result = await api.getCurriculum(sessionId);
      setState('CHAPTER_LEARNING');
      return result;
    } catch (err: unknown) {
      setError(err as ApiError);
      setState('ERROR');
    }
  }, [sessionId]);

  return {
    sessionId,
    state,
    error,
    createSession,
    submitGoal,
    answerProfile,
    getCurriculum,
  };
}
