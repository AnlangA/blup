import { useEffect } from 'react';
import { useSessionStore, CodeTheme } from '../../state/sessionStore';
import {
  useCurriculum,
  useChapter,
  usePrefetchChapters,
} from '../../hooks/query';
import type { Chapter } from '../../api/client';
import { ExportButton } from '../export/ExportButton';

export function CurriculumSidebar() {
  const sessionId = useSessionStore((s) => s.sessionId);
  const currentChapterId = useSessionStore((s) => s.currentChapterId);
  const codeTheme = useSessionStore((s) => s.codeTheme);
  const setChapter = useSessionStore((s) => s.setChapter);
  const setCodeTheme = useSessionStore((s) => s.setCodeTheme);

  const {
    data: curriculum,
    isLoading,
    isError,
  } = useCurriculum(sessionId);

  const chapters = curriculum?.chapters ?? [];
  const chapterIds = chapters.map((c) => c.id);
  const { prefetchAll } = usePrefetchChapters(sessionId, chapterIds);

  // Prefetch all chapter content once curriculum is loaded
  useEffect(() => {
    if (chapterIds.length > 0) {
      prefetchAll();
    }
  }, [chapterIds, prefetchAll]);

  const handleChapterClick = (ch: Chapter) => {
    setChapter(ch.id);
  };

  if (isLoading) {
    return (
      <aside className="curriculum-sidebar">
        <h2>Curriculum</h2>
        <p className="loading-text">Loading curriculum...</p>
      </aside>
    );
  }

  if (isError) {
    return (
      <aside className="curriculum-sidebar">
        <h2>Curriculum</h2>
        <p className="loading-text">Failed to load curriculum.</p>
      </aside>
    );
  }

  return (
    <aside className="curriculum-sidebar">
      <div className="sidebar-title-row">
        <h2>Curriculum</h2>
        <div className="sidebar-title-actions">
          <ExportButton />
          <select
            className="theme-selector"
            value={codeTheme}
            onChange={(e) => setCodeTheme(e.target.value as CodeTheme)}
          >
            <option value="github-dark">Dark</option>
            <option value="github-light">Light</option>
          </select>
        </div>
      </div>
      <ul>
        {chapters
          .slice()
          .sort((a, b) => a.order - b.order)
          .map((ch) => (
            <ChapterItem
              key={ch.id}
              chapter={ch}
              sessionId={sessionId!}
              isActive={currentChapterId === ch.id}
              onClick={() => handleChapterClick(ch)}
            />
          ))}
      </ul>
    </aside>
  );
}

function ChapterItem({
  chapter,
  sessionId,
  isActive,
  onClick,
}: {
  chapter: Chapter;
  sessionId: string;
  isActive: boolean;
  onClick: () => void;
}) {
  const { isFetching, data } = useChapter(sessionId, chapter.id);

  return (
    <li
      className={isActive ? 'active' : ''}
      onClick={onClick}
      role="button"
      tabIndex={0}
      onKeyDown={(e) => e.key === 'Enter' && onClick()}
    >
      <span className="chapter-order">{chapter.order}.</span>
      <span className="chapter-title">
        {chapter.title}
        {isFetching && !data && (
          <span className="chapter-loading-dot"> ⏳</span>
        )}
      </span>
    </li>
  );
}
