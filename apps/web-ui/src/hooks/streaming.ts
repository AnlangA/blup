import { useReducer, useState, useEffect, useCallback, useRef } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { SSEClient } from '../api/sse';
import type { ChapterContent } from '../api/client';

interface StreamState {
  content: string | null;
  isStreaming: boolean;
  error: string | null;
}

type StreamAction =
  | { type: 'reset' }
  | { type: 'chunk'; text: string }
  | { type: 'done'; content?: string | null }
  | { type: 'error'; message: string };

function streamReducer(state: StreamState, action: StreamAction): StreamState {
  switch (action.type) {
    case 'reset':
      return { content: null, isStreaming: true, error: null };
    case 'chunk':
      return { ...state, content: (state.content ?? '') + action.text };
    case 'done':
      return {
        ...state,
        content: action.content ?? state.content,
        isStreaming: false,
      };
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
  options?: { enabled?: boolean },
): StreamState {
  const queryClient = useQueryClient();
  const [state, dispatch] = useReducer(streamReducer, {
    content: null,
    isStreaming: false,
    error: null,
  });

  const sseRef = useRef(new SSEClient());

  useEffect(() => {
    if (!sessionId || !chapterId || options?.enabled === false) return;

    dispatch({ type: 'reset' });
    const url = `/api/session/${sessionId}/chapter/${chapterId}/stream`;

    const client = sseRef.current!;
    client.connectGet(url, {
      onChunk: (text) => dispatch({ type: 'chunk', text }),
      onDone: (result) => {
        const content =
          result &&
          typeof result === 'object' &&
          'content' in result &&
          typeof (result as { content?: unknown }).content === 'string'
            ? (result as { content: string }).content
            : null;
        if (content) {
          queryClient.setQueryData<ChapterContent>(
            ['chapter', sessionId, chapterId],
            {
              id: chapterId,
              role: 'assistant',
              content,
              timestamp: new Date().toISOString(),
            },
          );
        }
        client.close();
        dispatch({ type: 'done', content });
      },
      onError: (_code, message) => {
        client.close();
        dispatch({ type: 'error', message });
      },
    });

    return () => {
      client.close();
    };
  }, [sessionId, chapterId, queryClient, options?.enabled]);

  return state;
}

/**
 * Hook to manually trigger a streaming chapter fetch.
 */
export function useStreamChapterOnDemand(
  sessionId: string | null,
): StreamState & { streamChapter: (chapterId: string) => void } {
  const [chapterId, setChapterId] = useState<string | null>(null);
  const sseRef = useRef(new SSEClient());
  const streamState = useStreamChapterWithRef(sessionId, chapterId, sseRef);

  const streamChapter = useCallback(
    (chId: string) => {
      sseRef.current!.close();
      setChapterId(chId);
    },
    [],
  );

  return { ...streamState, streamChapter };
}

function useStreamChapterWithRef(
  sessionId: string | null,
  chapterId: string | null,
  sseRef: React.RefObject<SSEClient>,
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

    const client = sseRef.current!;
    client.connectGet(url, {
      onChunk: (text) => dispatch({ type: 'chunk', text }),
      onDone: (result) => {
        const content =
          result &&
          typeof result === 'object' &&
          'content' in result &&
          typeof (result as { content?: unknown }).content === 'string'
            ? (result as { content: string }).content
            : null;
        client.close();
        dispatch({ type: 'done', content });
      },
      onError: (_code, message) => {
        client.close();
        dispatch({ type: 'error', message });
      },
    });

    return () => {
      client.close();
    };
  }, [sessionId, chapterId, sseRef]);

  return state;
}
