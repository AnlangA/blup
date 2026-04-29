import { useSessionStore } from '../state/sessionStore';
import { CurriculumSidebar } from './curriculum/CurriculumSidebar';
import { ChatWindow } from './chat/ChatWindow';
import { MarkdownRenderer } from './content/MarkdownRenderer';

export function LearningLayout() {
  const chapterContent = useSessionStore((s) => s.chapterContent);
  const currentChapterId = useSessionStore((s) => s.currentChapterId);

  return (
    <div className="learning-layout">
      <CurriculumSidebar />
      <main className="chapter-content">
        {currentChapterId ? (
          chapterContent ? (
            <MarkdownRenderer content={chapterContent} />
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
      <ChatWindow />
    </div>
  );
}
