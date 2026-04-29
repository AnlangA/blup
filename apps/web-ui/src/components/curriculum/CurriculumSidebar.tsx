import { useEffect } from 'react';
import { useSessionStore, Chapter } from '../../state/sessionStore';
import { api } from '../../api/client';

export function CurriculumSidebar() {
  const sessionId = useSessionStore((s) => s.sessionId);
  const currentChapterId = useSessionStore((s) => s.currentChapterId);
  const state = useSessionStore((s) => s.state);
  const chapters = useSessionStore((s) => s.chapters);
  const setChapters = useSessionStore((s) => s.setChapters);

  useEffect(() => {
    if (!sessionId || chapters.length > 0) return;
    (async () => {
      try {
        const result = await api.getCurriculum(sessionId);
        const list = (result.chapters as Chapter[]) || [];
        setChapters(list);
      } catch {
        // handled by store
      }
    })();
  }, [sessionId, chapters.length, setChapters]);

  const handleChapterClick = async (ch: Chapter) => {
    if (!sessionId) return;
    useSessionStore.setState({
      currentChapterId: ch.id,
      state: 'CHAPTER_LEARNING',
    });
    try {
      const result = await api.startChapter(sessionId, ch.id);
      const content = result.content as string;
      if (content) {
        const msgs = useSessionStore.getState().messages;
        useSessionStore.setState({
          messages: [...msgs, {
            id: crypto.randomUUID(),
            role: 'assistant',
            content,
            timestamp: new Date().toISOString(),
          }],
        });
      }
    } catch {
      // handled by store
    }
  };

  if (state !== 'CURRICULUM_PLANNING' && state !== 'CHAPTER_LEARNING') {
    return null;
  }

  return (
    <aside className="curriculum-sidebar">
      <h2>Curriculum</h2>
      {chapters.length === 0 ? (
        <p className="loading-text">Loading curriculum...</p>
      ) : (
        <ul>
          {chapters
            .slice()
            .sort((a, b) => a.order - b.order)
            .map((ch) => (
              <li
                key={ch.id}
                className={currentChapterId === ch.id ? 'active' : ''}
                onClick={() => handleChapterClick(ch)}
                role="button"
                tabIndex={0}
                onKeyDown={(e) => e.key === 'Enter' && handleChapterClick(ch)}
              >
                <span className="chapter-order">{ch.order}.</span>
                <span className="chapter-title">{ch.title}</span>
              </li>
            ))}
        </ul>
      )}
    </aside>
  );
}
