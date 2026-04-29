import { useState, useRef, useCallback } from 'react';
import { useSessionStore } from '../state/sessionStore';
import { useChapter } from '../hooks/query';
import { useStreamChapter } from '../hooks/streaming';
import { CurriculumSidebar } from './curriculum/CurriculumSidebar';
import { ChatWindow } from './chat/ChatWindow';
import { MarkdownRenderer } from './content/MarkdownRenderer';

const SIDEBAR_DEFAULT = 260;
const SIDEBAR_MIN = 180;
const SIDEBAR_MAX = 500;
const CHAT_DEFAULT = 380;
const CHAT_MIN = 280;
const CHAT_MAX = 600;

export function LearningLayout() {
  const sessionId = useSessionStore((s) => s.sessionId);
  const currentChapterId = useSessionStore((s) => s.currentChapterId);

  // Use cached chapter data as initial content, streaming for live updates
  const { data: cachedChapter, isLoading: cacheLoading } = useChapter(
    sessionId,
    currentChapterId,
  );
  const streamState = useStreamChapter(sessionId, currentChapterId);

  // Prefer streamed content, fall back to cached
  const chapterContent =
    streamState.content ?? cachedChapter?.content ?? null;
  const isLoading =
    (cacheLoading || streamState.isStreaming) && !chapterContent;

  const [sidebarWidth, setSidebarWidth] = useState(SIDEBAR_DEFAULT);
  const [chatWidth, setChatWidth] = useState(CHAT_DEFAULT);
  const dragging = useRef<'sidebar' | 'chat' | null>(null);

  const onResizeStart = useCallback(
    (panel: 'sidebar' | 'chat') => (e: React.MouseEvent) => {
      e.preventDefault();
      dragging.current = panel;
      let lastX = e.clientX;

      const onMove = (ev: MouseEvent) => {
        const delta = ev.clientX - lastX;
        lastX = ev.clientX;

        if (dragging.current === 'sidebar') {
          setSidebarWidth((w) =>
            Math.min(SIDEBAR_MAX, Math.max(SIDEBAR_MIN, w + delta)),
          );
        } else {
          setChatWidth((w) =>
            Math.min(CHAT_MAX, Math.max(CHAT_MIN, w - delta)),
          );
        }
      };

      const onUp = () => {
        dragging.current = null;
        document.removeEventListener('mousemove', onMove);
        document.removeEventListener('mouseup', onUp);
        document.body.style.userSelect = '';
        document.body.style.cursor = '';
      };

      document.body.style.userSelect = 'none';
      document.body.style.cursor = 'col-resize';
      document.addEventListener('mousemove', onMove);
      document.addEventListener('mouseup', onUp);
    },
    [],
  );

  return (
    <div
      className="learning-layout"
      style={{
        gridTemplateColumns: `${sidebarWidth}px 4px 1fr 4px ${chatWidth}px`,
      }}
    >
      <CurriculumSidebar />
      <div
        className="resize-handle"
        onMouseDown={onResizeStart('sidebar')}
      />
      <main className="chapter-content">
        {currentChapterId ? (
          isLoading ? (
            <div className="welcome-content">
              <p>Loading chapter content...</p>
            </div>
          ) : streamState.error ? (
            <div className="welcome-content">
              <p style={{ color: 'var(--color-error)' }}>
                Failed to load chapter: {streamState.error}
              </p>
            </div>
          ) : chapterContent ? (
            <>
              {streamState.isStreaming && (
                <div className="streaming-indicator">Streaming...</div>
              )}
              <MarkdownRenderer content={chapterContent} />
            </>
          ) : (
            <div className="welcome-content">
              <p>Loading chapter content...</p>
            </div>
          )
        ) : (
          <div className="welcome-content">
            <h2>Welcome to Your Learning Journey</h2>
            <p>Select a chapter from the sidebar to begin learning.</p>
          </div>
        )}
      </main>
      <div className="resize-handle" onMouseDown={onResizeStart('chat')} />
      <ChatWindow />
    </div>
  );
}
