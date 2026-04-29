import { useState, useRef, useEffect, useCallback, useMemo } from 'react';
import { MarkdownRenderer } from '../content/MarkdownRenderer';
import { api } from '../../api/client';
import { useSessionStore } from '../../state/sessionStore';

export function ChatWindow() {
  const sessionId = useSessionStore((s) => s.sessionId);
  const currentChapterId = useSessionStore((s) => s.currentChapterId);
  const chapterChatMessages = useSessionStore((s) => s.chapterChatMessages);
  const addChatMessage = useSessionStore((s) => s.addChatMessage);
  const [input, setInput] = useState('');
  const [loading, setLoading] = useState(false);
  const bottomRef = useRef<HTMLDivElement>(null);
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  // Get messages for current chapter
  const messages = useMemo(() => {
    return currentChapterId ? (chapterChatMessages[currentChapterId] || []) : [];
  }, [currentChapterId, chapterChatMessages]);

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [messages]);

  // Auto-resize textarea
  useEffect(() => {
    const ta = textareaRef.current;
    if (!ta) return;
    ta.style.height = 'auto';
    ta.style.height = `${Math.min(ta.scrollHeight, 200)}px`;
  }, [input]);

  const handleSend = useCallback(async () => {
    if (!sessionId || !currentChapterId || !input.trim()) return;
    const question = input.trim();
    setInput('');
    setLoading(true);

    const userMsg = {
      id: crypto.randomUUID(),
      role: 'user' as const,
      content: question,
      timestamp: new Date().toISOString(),
    };

    // Add user message to chapter chat
    addChatMessage(currentChapterId, userMsg);

    try {
      const result = await api.askQuestion(sessionId, currentChapterId, question);
      const assistantMsg = {
        id: (result.id as string) || crypto.randomUUID(),
        role: 'assistant' as const,
        content: (result.content as string) || 'I couldn\'t process that question.',
        timestamp: (result.timestamp as string) || new Date().toISOString(),
      };
      // Add assistant message to chapter chat
      addChatMessage(currentChapterId, assistantMsg);
    } catch (err) {
      console.error('Failed to get answer:', err);
      const errorMsg = {
        id: crypto.randomUUID(),
        role: 'assistant' as const,
        content: 'Sorry, I encountered an error processing your question. Please try again.',
        timestamp: new Date().toISOString(),
      };
      // Add error message to chapter chat
      addChatMessage(currentChapterId, errorMsg);
    } finally {
      setLoading(false);
    }
  }, [sessionId, currentChapterId, input, addChatMessage]);

  const handleKeyDown = useCallback((e: React.KeyboardEvent) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleSend();
    }
  }, [handleSend]);

  return (
    <div className="chat-window">
      <div className="messages-container">
        {messages.map((msg) => (
          <div key={msg.id} className={`message ${msg.role}`}>
            <div className="message-avatar">
              {msg.role === 'user' ? '👤' : '🤖'}
            </div>
            <div className="message-body">
              <MarkdownRenderer content={msg.content} />
            </div>
          </div>
        ))}
        {loading && (
          <div className="message assistant">
            <div className="message-avatar">🤖</div>
            <div className="message-body typing-indicator">Thinking...</div>
          </div>
        )}
        <div ref={bottomRef} />
      </div>
      <form
        className="chat-input-form"
        onSubmit={(e) => { e.preventDefault(); handleSend(); }}
      >
        <textarea
          ref={textareaRef}
          rows={1}
          value={input}
          onChange={(e) => setInput(e.target.value)}
          onKeyDown={handleKeyDown}
          placeholder="Ask a question about this chapter..."
          disabled={loading}
          aria-label="Question input"
        />
        <button type="submit" disabled={loading || !input.trim()}>
          Send
        </button>
      </form>
    </div>
  );
}
