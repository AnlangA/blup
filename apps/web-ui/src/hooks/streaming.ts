import { useReducer, useState, useEffect, useCallback } from 'react';
import { sseClient } from '../api/sse';

interface StreamState {
  content: string | null;
  isStreaming: boolean;
  error: string | null;
}

type StreamAction =
  | { type: 'reset' }
  | { type: 'chunk'; text: string }
  | { type: 'done' }
  | { type: 'error'; message: string };

function streamReducer(state: StreamState, action: StreamAction): StreamState {
  switch (action.type) {
    case 'reset':
      return { content: null, isStreaming: true, error: null };
    case 'chunk':
      return { ...state, content: (state.content ?? '') + action.text };
    case 'done':
      return { ...state, isStreaming: false };
    case 'error':
      return { content: null, isStreaming: false, error: action.message };
  }
}

/**
 * Stream chapter content via SSE from the backend.
 */
export function useStreamChapter(
  sessionId: string | null,
  chapterId: string | null,
): StreamState {
  const [state, dispatch] = useReducer(streamReducer, {
    content: null,
    isStreaming: false,
    error: null,
  });

  useEffect(() => {
    if (!sessionId || !chapterId) return;

    dispatch({ type: 'reset' });
    const url = `/api/session/${sessionId}/chapter/${chapterId}/stream`;

    sseClient.connectGet(url, {
      onChunk: (text) => dispatch({ type: 'chunk', text }),
      onDone: () => {
        sseClient.close();
        dispatch({ type: 'done' });
      },
      onError: (_code, message) => {
        sseClient.close();
        dispatch({ type: 'error', message });
      },
    });

    return () => {
      sseClient.close();
    };
  }, [sessionId, chapterId]);

  return state;
}

/**
 * Hook to manually trigger a streaming chapter fetch.
 */
export function useStreamChapterOnDemand(
  sessionId: string | null,
): StreamState & { streamChapter: (chapterId: string) => void } {
  const [chapterId, setChapterId] = useState<string | null>(null);
  const streamState = useStreamChapter(sessionId, chapterId);

  const streamChapter = useCallback(
    (chId: string) => {
      sseClient.close();
      setChapterId(chId);
    },
    [],
  );

  return { ...streamState, streamChapter };
}
