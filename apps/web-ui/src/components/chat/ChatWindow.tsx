import { useState, useRef, useEffect, useCallback } from "react";
import { MarkdownRenderer } from "../content/MarkdownRenderer";
import { useSessionStore } from "../../state/sessionStore";
import { useSession, useAskQuestion } from "../../hooks/query";

interface ChatMessage {
  id: string;
  role: string;
  content: string;
  timestamp: string;
  chapter_id?: string;
}

export function ChatWindow() {
  const sessionId = useSessionStore((s) => s.sessionId);
  const currentChapterId = useSessionStore((s) => s.currentChapterId);
  const { data: session } = useSession(sessionId);
  const askQuestion = useAskQuestion(sessionId, currentChapterId);

  const [input, setInput] = useState("");
  const bottomRef = useRef<HTMLDivElement>(null);
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  // Messages come from backend session snapshot, filtered by current chapter
  const messages: ChatMessage[] = (
    (session?.messages ?? []) as ChatMessage[]
  ).filter((m) => !currentChapterId || m.chapter_id === currentChapterId);

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [messages]);

  // Auto-resize textarea
  useEffect(() => {
    const ta = textareaRef.current;
    if (!ta) return;
    ta.style.height = "auto";
    ta.style.height = `${Math.min(ta.scrollHeight, 200)}px`;
  }, [input]);

  const handleSend = useCallback(() => {
    if (!sessionId || !currentChapterId || !input.trim()) return;
    askQuestion.mutate(input.trim());
    setInput("");
  }, [sessionId, currentChapterId, input, askQuestion]);

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key === "Enter" && !e.shiftKey) {
        e.preventDefault();
        handleSend();
      }
    },
    [handleSend],
  );

  return (
    <div className="chat-window">
      <div className="messages-container">
        {messages.map((msg) => (
          <div key={msg.id} className={`message ${msg.role}`}>
            <div className="message-avatar">
              {msg.role === "user" ? "👤" : "🤖"}
            </div>
            <div className="message-body">
              <MarkdownRenderer content={msg.content} />
            </div>
          </div>
        ))}
        {askQuestion.isPending && (
          <div className="message assistant">
            <div className="message-avatar">🤖</div>
            <div className="message-body typing-indicator">Thinking...</div>
          </div>
        )}
        <div ref={bottomRef} />
      </div>
      <form
        className="chat-input-form"
        onSubmit={(e) => {
          e.preventDefault();
          handleSend();
        }}
      >
        <textarea
          ref={textareaRef}
          rows={1}
          value={input}
          onChange={(e) => setInput(e.target.value)}
          onKeyDown={handleKeyDown}
          placeholder="Ask a question about this chapter..."
          disabled={askQuestion.isPending}
          aria-label="Question input"
        />
        <button type="submit" disabled={askQuestion.isPending || !input.trim()}>
          Send
        </button>
      </form>
    </div>
  );
}
