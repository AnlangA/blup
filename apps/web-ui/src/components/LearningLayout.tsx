import { useSessionStore } from '../state/sessionStore';
import { CurriculumSidebar } from './curriculum/CurriculumSidebar';
import { ChatWindow } from './chat/ChatWindow';
import { MarkdownRenderer } from './content/MarkdownRenderer';

export function LearningLayout() {
  const messages = useSessionStore((s) => s.messages);
  const currentChapterId = useSessionStore((s) => s.currentChapterId);

  const lastAssistantMsg = [...messages]
    .reverse()
    .find((m) => m.role === 'assistant');

  return (
    <div className="learning-layout">
      <CurriculumSidebar />
      <main className="chapter-content">
        {lastAssistantMsg && currentChapterId ? (
          <MarkdownRenderer content={lastAssistantMsg.content} />
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
