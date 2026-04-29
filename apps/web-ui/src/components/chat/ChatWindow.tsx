import { useState, useRef, useEffect, useCallback } from 'react';
import { MarkdownRenderer } from '../content/MarkdownRenderer';
import { api } from '../../api/client';
import { useSessionStore } from '../../state/sessionStore';

interface Message {
  id: string;
  role: string;
  content: string;
  timestamp: string;
}

export function ChatWindow() {
  const sessionId = useSessionStore((s) => s.sessionId);
  const currentChapterId = useSessionStore((s) => s.currentChapterId);
  const [messages, setMessages] = useState<Message[]>([]);
  const [input, setInput] = useState('');
  const [loading, setLoading] = useState(false);
  const bottomRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [messages]);

  const handleSend = useCallback(async () => {
    if (!sessionId || !currentChapterId || !input.trim()) return;
    const question = input.trim();
    setInput('');
    setLoading(true);

    const userMsg: Message = {
      id: crypto.randomUUID(),
      role: 'user',
      content: question,
      timestamp: new Date().toISOString(),
    };
    setMessages((prev) => [...prev, userMsg]);

    try {
      const result = await api.askQuestion(sessionId, currentChapterId, question);
      const assistantMsg: Message = {
        id: (result.id as string) || crypto.randomUUID(),
        role: 'assistant',
        content: (result.content as string) || 'I couldn\'t process that question.',
        timestamp: (result.timestamp as string) || new Date().toISOString(),
      };
      setMessages((prev) => [...prev, assistantMsg]);
    } catch {
      // error handled by store
    } finally {
      setLoading(false);
    }
  }, [sessionId, currentChapterId, input]);

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
        <input
          type="text"
          value={input}
          onChange={(e) => setInput(e.target.value)}
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
