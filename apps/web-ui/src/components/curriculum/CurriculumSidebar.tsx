import { useEffect, useMemo } from 'react';
import { useSessionStore, Chapter } from '../../state/sessionStore';
import { api } from '../../api/client';

const sleep = (ms: number) => new Promise(resolve => setTimeout(resolve, ms));

// Extract fetch logic outside component
async function fetchChapterWithRetry(
  sid: string,
  chapterId: string,
  setChapterCache: (id: string, content: string) => void,
  setChapterLoading: (id: string, loading: boolean) => void,
  retryCount = 0,
): Promise<void> {
  const MAX_RETRIES = 3;
  try {
    const result = await api.startChapter(sid, chapterId);
    const content = result.content as string;
    if (content) {
      setChapterCache(chapterId, content);
      setChapterLoading(chapterId, false);
    }
  } catch {
    if (retryCount < MAX_RETRIES) {
      const delay = Math.random() * 10000;
      console.log(`[CurriculumSidebar] Chapter ${chapterId} failed, retry ${retryCount + 1} after ${Math.round(delay)}ms`);
      await sleep(delay);
      await fetchChapterWithRetry(sid, chapterId, setChapterCache, setChapterLoading, retryCount + 1);
      return;
    }
    console.error(`[CurriculumSidebar] Chapter ${chapterId} failed after ${MAX_RETRIES} retries`);
    setChapterLoading(chapterId, false);
  }
}

export function CurriculumSidebar() {
  const sessionId = useSessionStore((s) => s.sessionId);
  const currentChapterId = useSessionStore((s) => s.currentChapterId);
  const state = useSessionStore((s) => s.state);
  const chapters = useSessionStore((s) => s.chapters);
  const chapterCache = useSessionStore((s) => s.chapterCache);
  const chapterLoading = useSessionStore((s) => s.chapterLoading);
  const setChapters = useSessionStore((s) => s.setChapters);
  const setChapterContent = useSessionStore((s) => s.setChapterContent);
  const setChapterCache = useSessionStore((s) => s.setChapterCache);
  const setChapterLoading = useSessionStore((s) => s.setChapterLoading);

  // Load curriculum and fetch all chapters concurrently
  useEffect(() => {
    if (!sessionId || chapters.length > 0) return;

    const loadAndFetch = async () => {
      try {
        const result = await api.getCurriculum(sessionId);
        const list = (result.chapters as Chapter[]) || [];
        setChapters(list);

        // Mark all as loading
        const loadingState: Record<string, boolean> = {};
        list.forEach(ch => { loadingState[ch.id] = true; });
        useSessionStore.setState({ chapterLoading: loadingState });

        // Fetch all concurrently
        await Promise.allSettled(
          list.map(ch => fetchChapterWithRetry(sessionId, ch.id, setChapterCache, setChapterLoading))
        );
      } catch (err) {
        console.error('[CurriculumSidebar] Failed to load curriculum:', err);
      }
    };

    loadAndFetch();
  }, [sessionId, chapters.length, setChapters, setChapterCache, setChapterLoading]);

  const handleChapterClick = useMemo(() => {
    return async (ch: Chapter) => {
      if (!sessionId) return;

      const getCache = () => useSessionStore.getState().chapterCache;
      const getLoading = () => useSessionStore.getState().chapterLoading;

      // Check cache first
      const cached = getCache()[ch.id];
      if (cached) {
        useSessionStore.setState({
          currentChapterId: ch.id,
          state: 'CHAPTER_LEARNING',
          chapterContent: cached,
        });
        return;
      }

      // Show loading
      useSessionStore.setState({
        currentChapterId: ch.id,
        state: 'CHAPTER_LEARNING',
        chapterContent: null,
      });

      // Wait for concurrent fetch to finish (max 60s)
      if (getLoading()[ch.id]) {
        let waited = 0;
        while (getLoading()[ch.id] && waited < 60000) {
          await sleep(200);
          waited += 200;
        }
        const cachedAfterWait = getCache()[ch.id];
        if (cachedAfterWait) {
          setChapterContent(cachedAfterWait);
          return;
        }
      }

      // Fetch directly
      try {
        const result = await api.startChapter(sessionId, ch.id);
        const content = result.content as string;
        if (content) {
          setChapterContent(content);
          setChapterCache(ch.id, content);
        }
      } catch (err) {
        console.error('[CurriculumSidebar] Failed to start chapter:', err);
        setChapterContent('Sorry, I encountered an error loading this chapter. Please try again.');
      }
    };
  }, [sessionId, setChapterContent, setChapterCache]);

  if (state !== 'CURRICULUM_PLANNING' && state !== 'CHAPTER_LEARNING') {
    return null;
  }

  const loadedCount = Object.keys(chapterCache).length;

  return (
    <aside className="curriculum-sidebar">
      <h2>Curriculum</h2>
      {chapters.length === 0 ? (
        <p className="loading-text">Loading curriculum...</p>
      ) : (
        <>
          {loadedCount < chapters.length && (
            <p className="loading-text">Loading chapters... ({loadedCount}/{chapters.length})</p>
          )}
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
                  <span className="chapter-title">
                    {ch.title}
                    {chapterLoading[ch.id] && !chapterCache[ch.id] && (
                      <span className="chapter-loading-dot"> ⏳</span>
                    )}
                  </span>
                </li>
              ))}
          </ul>
        </>
      )}
    </aside>
  );
}
