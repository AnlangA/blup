# Apps Module — Phase 1: Web UI (SPA)

## Module Overview

`apps/web-ui` is the Phase 1 single-page application — the primary user interface for the Blup learning assistant. It renders chat, curriculum navigation, chapter content (Markdown + KaTeX + code), and SSE-streamed assistant responses.

**Technology decision:** TBD between React 18+ with Vite and Svelte 5 with Vite. Both evaluated with TypeScript. This plan uses React as the reference implementation; Svelte equivalents should follow the same component architecture and API contract.

## Phase 1 Scope

| Deliverable | Description | Status |
|-------------|-------------|--------|
| Chat window | Scrollable message list with user input, streaming message display, error states | Planned |
| Curriculum sidebar | Chapter list with progress indicators, current chapter highlight, navigation | Planned |
| Chapter content area | Markdown rendering with KaTeX math and CodeMirror 6 code blocks | Planned |
| State routing | Mirror Agent Core session state in UI; handle reconnect | Planned |
| Error display | Structured error messages, retry actions, loading states | Planned |

### Explicit Exclusions

- No Tauri desktop shell (Phase 2.5).
- No Bevy viewer embedding (Phase 3).
- No file import UI (Phase 2.5).
- No direct LLM API calls from the browser.
- No code execution in the browser.
- No offline mode (requires backend).

## File Structure

```
apps/web-ui/
├── package.json
├── tsconfig.json
├── vite.config.ts
├── index.html
├── public/
│   └── favicon.svg
├── src/
│   ├── main.tsx                    # Entry point, mount App
│   ├── App.tsx                     # Root component, session provider, router
│   ├── vite-env.d.ts
│   │
│   ├── api/
│   │   ├── client.ts              # HTTP client (fetch wrapper, error handling)
│   │   ├── sse.ts                  # SSE EventSource wrapper with reconnect
│   │   ├── session.ts             # Session API functions
│   │   └── types.ts               # API request/response types (generated from schemas)
│   │
│   ├── components/
│   │   ├── chat/
│   │   │   ├── ChatWindow.tsx      # Main chat container
│   │   │   ├── MessageList.tsx     # Scrollable message list
│   │   │   ├── MessageBubble.tsx   # Single message (user or assistant)
│   │   │   ├── StreamingMessage.tsx # Message being streamed via SSE
│   │   │   ├── ChatInput.tsx       # Text input + send button
│   │   │   └── TypingIndicator.tsx # "Assistant is typing..." animation
│   │   │
│   │   ├── curriculum/
│   │   │   ├── CurriculumSidebar.tsx   # Chapter list sidebar
│   │   │   ├── ChapterListItem.tsx     # Single chapter in list
│   │   │   └── ProgressIndicator.tsx   # Progress bar or percentage
│   │   │
│   │   ├── content/
│   │   │   ├── ChapterContent.tsx      # Main content area
│   │   │   ├── MarkdownRenderer.tsx    # Markdown → HTML with KaTeX
│   │   │   ├── CodeBlock.tsx           # Code block with syntax highlighting
│   │   │   ├── MathBlock.tsx           # KaTeX math rendering
│   │   │   └── ExerciseCard.tsx        # Inline exercise display
│   │   │
│   │   ├── session/
│   │   │   ├── GoalInput.tsx           # Initial goal input form
│   │   │   ├── FeasibilityResult.tsx   # Feasibility display with adjust/confirm
│   │   │   ├── ProfileQuestion.tsx     # Profile collection Q&A
│   │   │   └── CompletionScreen.tsx    # All chapters done screen
│   │   │
│   │   └── shared/
│   │       ├── ErrorDisplay.tsx        # Structured error with retry button
│   │       ├── LoadingSpinner.tsx      # Loading indicator
│   │       ├── StatusBadge.tsx         # State badge (feasible, error, etc.)
│   │       └── Button.tsx              # Styled button component
│   │
│   ├── hooks/
│   │   ├── useSession.ts          # Session state management
│   │   ├── useSSE.ts              # SSE connection hook
│   │   ├── useChat.ts             # Chat message state
│   │   └── useChapterNavigation.ts # Chapter selection and progress
│   │
│   ├── state/
│   │   ├── sessionStore.ts        # Zustand or Context-based session state
│   │   ├── chatStore.ts           # Chat message state
│   │   └── curriculumStore.ts     # Curriculum and progress state
│   │
│   ├── types/
│   │   ├── schema-types.ts        # TypeScript types mirroring Phase 1 schemas
│   │   └── ui-types.ts            # UI-specific types (ViewState, etc.)
│   │
│   └── styles/
│       ├── global.css             # Global styles, CSS variables, theme
│       ├── chat.css
│       ├── curriculum.css
│       ├── content.css
│       └── components.css
│
├── tests/
│   ├── components/
│   │   ├── ChatWindow.test.tsx
│   │   ├── MessageList.test.tsx
│   │   ├── CurriculumSidebar.test.tsx
│   │   └── ChapterContent.test.tsx
│   ├── hooks/
│   │   ├── useSession.test.ts
│   │   └── useSSE.test.ts
│   └── integration/
│       ├── full_flow.test.tsx      # Full user journey test
│       └── sse_streaming.test.tsx  # SSE mock tests
│
└── e2e/
    └── learning-flow.spec.ts       # Playwright E2E test
```

## Component Architecture

### Component Tree

```
<App>
  <SessionProvider>              ← manages session_id, state machine state
    <GoalInput />                ← shown when state is GOAL_INPUT
    <FeasibilityResult />        ← shown when state is FEASIBILITY_CHECK
    <ProfileQuestion />          ← shown when state is PROFILE_COLLECTION

    <div class="learning-layout"> ← shown when state ≥ CURRICULUM_PLANNING
      <CurriculumSidebar>
        <ChapterListItem />      ← one per chapter
        <ProgressIndicator />
      </CurriculumSidebar>

      <main>
        <ChapterContent>
          <MarkdownRenderer>
            <MathBlock />
            <CodeBlock />
          </MarkdownRenderer>
          <ExerciseCard />
        </ChapterContent>

        <ChatWindow>
          <MessageList>
            <MessageBubble />    ← completed messages
            <StreamingMessage /> ← current SSE stream
          </MessageList>
          <TypingIndicator />
          <ChatInput />
        </ChatWindow>
      </main>
    </div>

    <CompletionScreen />         ← shown when state is COMPLETED
    <ErrorDisplay />             ← shown on error (overlay or inline)
  </SessionProvider>
</App>
```

### State-Driven Rendering

The root `App` renders different views based on `session.state`:

```typescript
type SessionState =
  | 'IDLE'
  | 'GOAL_INPUT'
  | 'FEASIBILITY_CHECK'
  | 'PROFILE_COLLECTION'
  | 'CURRICULUM_PLANNING'
  | 'CHAPTER_LEARNING'
  | 'COMPLETED'
  | 'ERROR';

function App() {
  const { state, error } = useSession();

  switch (state) {
    case 'IDLE':
    case 'GOAL_INPUT':
      return <GoalInput />;
    case 'FEASIBILITY_CHECK':
      return <FeasibilityResult />;
    case 'PROFILE_COLLECTION':
      return <ProfileQuestion />;
    case 'CURRICULUM_PLANNING':
    case 'CHAPTER_LEARNING':
      return <LearningLayout />;
    case 'COMPLETED':
      return <CompletionScreen />;
    case 'ERROR':
      return <ErrorDisplay error={error} onRetry={...} onReset={...} />;
  }
}
```

### Session Provider

The `SessionProvider` is the top-level state manager. It:
1. Creates a session via `POST /api/session` on mount.
2. Stores `session_id` and current `state`.
3. Exposes action functions (`submitGoal`, `answerProfile`, `selectChapter`, etc.).
4. Manages SSE connections (one active stream at a time).
5. Handles reconnect via `Last-Event-ID`.

```typescript
// hooks/useSession.ts (conceptual)
function useSession() {
  const [sessionId, setSessionId] = useState<string | null>(null);
  const [state, setState] = useState<SessionState>('IDLE');
  const [error, setError] = useState<ApiError | null>(null);

  // Create session on mount
  useEffect(() => {
    api.createSession().then(({ session_id, state }) => {
      setSessionId(session_id);
      setState(state);
    });
  }, []);

  // Action: submit learning goal
  async function submitGoal(goal: LearningGoal) {
    setState('FEASIBILITY_CHECK');
    const stream = api.submitGoal(sessionId!, goal);
    // SSE handling: chunk → display, done → transition state
  }

  // ... other actions

  return { sessionId, state, error, submitGoal, answerProfile, selectChapter, ... };
}
```

### SSE Client

Two SSE implementations are needed: `EventSource` for GET endpoints (chapter teaching, curriculum streaming), and `fetch` + `ReadableStream` for POST endpoints (goal submission, profile answers, question asking).

#### GET-based SSE (EventSource with Reconnection)

```typescript
// api/sse.ts
type SSEEventHandler = (data: any) => void;

interface SSEHandlers {
  onChunk?: SSEEventHandler;
  onStatus?: SSEEventHandler;
  onDone?: SSEEventHandler;
  onError?: SSEEventHandler;
  onPing?: () => void;
}

class SSEClient {
  private eventSource: EventSource | null = null;
  private lastEventId: string | null = null;
  private replayBuffer: SSEEvent[] = [];
  private reconnectAttempt = 0;
  private maxReconnectAttempts = 5;
  private baseReconnectDelayMs = 1000;
  private maxReconnectDelayMs = 30000;
  private reconnectTimer: number | null = null;
  private url = '';
  private handlers: SSEHandlers | null = null;
  private isDisposed = false;

  connect(url: string, handlers: SSEHandlers): void {
    this.url = url;
    this.handlers = handlers;
    this.isDisposed = false;
    this.doConnect();
  }

  private doConnect(): void {
    // Append Last-Event-ID for replay
    const url = this.lastEventId
      ? `${this.url}${this.url.includes('?') ? '&' : '?'}_lastEventId=${this.lastEventId}`
      : this.url;

    this.eventSource = new EventSource(url);

    this.eventSource.addEventListener('chunk', (e: MessageEvent) => {
      const data = JSON.parse(e.data);
      this.handlers?.onChunk?.(data);
      this.replayBuffer.push({ type: 'chunk', data });
      this.lastEventId = (e as any).lastEventId;
      this.reconnectAttempt = 0; // Reset on successful message
    });

    this.eventSource.addEventListener('status', (e: MessageEvent) => {
      this.handlers?.onStatus?.(JSON.parse(e.data));
    });

    this.eventSource.addEventListener('done', (e: MessageEvent) => {
      this.handlers?.onDone?.(JSON.parse(e.data));
      this.close();
    });

    this.eventSource.addEventListener('error', (e: MessageEvent) => {
      try {
        const data = JSON.parse(e.data);
        this.handlers?.onError?.(data);
      } catch {
        // Connection error (not a structured error event)
        this.handleConnectionError();
      }
    });

    this.eventSource.addEventListener('ping', () => {
      this.handlers?.onPing?.();
    });

    // Handle connection loss (browser EventSource error)
    this.eventSource.onerror = () => {
      this.handleConnectionError();
    };
  }

  private handleConnectionError(): void {
    if (this.isDisposed) return;

    this.eventSource?.close();
    this.eventSource = null;
    this.reconnectAttempt++;

    if (this.reconnectAttempt <= this.maxReconnectAttempts) {
      // Exponential backoff with jitter: 1s, 2s, 4s, 8s, 16s
      const delay = Math.min(
        this.baseReconnectDelayMs * Math.pow(2, this.reconnectAttempt - 1),
        this.maxReconnectDelayMs
      ) + Math.random() * 1000;

      console.warn(`SSE reconnect attempt ${this.reconnectAttempt}/${this.maxReconnectAttempts} in ${Math.round(delay)}ms`);

      this.reconnectTimer = window.setTimeout(() => {
        this.doConnect();
      }, delay);
    } else {
      this.handlers?.onError?.({
        code: 'SSE_RECONNECT_FAILED',
        message: `Failed to reconnect after ${this.maxReconnectAttempts} attempts. Please refresh.`,
      });
    }
  }

  close(): void {
    this.isDisposed = true;
    if (this.reconnectTimer !== null) {
      clearTimeout(this.reconnectTimer);
      this.reconnectTimer = null;
    }
    this.eventSource?.close();
    this.eventSource = null;
  }

  /** Return events since a given event ID for state recovery. */
  getEventsSince(eventId: string): SSEEvent[] {
    const idx = this.replayBuffer.findIndex(e => e.type === 'chunk' && e.data.index === eventId);
    return idx >= 0 ? this.replayBuffer.slice(idx) : [];
  }
}
```

#### POST-based SSE (fetch + ReadableStream)

```typescript
// api/sse.ts (continued)
interface PostSSEOptions {
  url: string;
  body: unknown;
  handlers: SSEHandlers;
  signal?: AbortSignal;
}

async function postSSE(options: PostSSEOptions): Promise<void> {
  const { url, body, handlers, signal } = options;

  const response = await fetch(url, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json', 'Accept': 'text/event-stream' },
    body: JSON.stringify(body),
    signal,
  });

  if (!response.ok) {
    // Non-streaming error response
    const errorBody = await response.json().catch(() => ({}));
    handlers.onError?.(errorBody.error || { code: 'HTTP_ERROR', message: `HTTP ${response.status}` });
    return;
  }

  if (!response.body) {
    handlers.onError?.({ code: 'NO_BODY', message: 'Response has no body' });
    return;
  }

  const reader = response.body.getReader();
  const decoder = new TextDecoder();
  let buffer = '';
  let currentEvent = '';
  let currentData = '';

  try {
    while (true) {
      const { done, value } = await reader.read();
      if (done) break;

      buffer += decoder.decode(value, { stream: true });
      const lines = buffer.split('\n');
      buffer = lines.pop() || ''; // Keep incomplete line in buffer

      for (const line of lines) {
        if (line.startsWith('event: ')) {
          currentEvent = line.slice(7).trim();
        } else if (line.startsWith('data: ')) {
          currentData = line.slice(6);
        } else if (line === '') {
          // Empty line = end of event → dispatch
          if (currentEvent && currentData) {
            dispatchSSEEvent(currentEvent, currentData, handlers);
          }
          currentEvent = '';
          currentData = '';
        }
      }
    }

    // Process any remaining data in buffer
    if (currentEvent && currentData) {
      dispatchSSEEvent(currentEvent, currentData, handlers);
    }
  } catch (err: any) {
    if (err.name === 'AbortError') return; // Intentional abort
    handlers.onError?.({ code: 'STREAM_ERROR', message: err.message });
  } finally {
    reader.releaseLock();
  }
}

function dispatchSSEEvent(event: string, data: string, handlers: SSEHandlers): void {
  try {
    const parsed = JSON.parse(data);
    switch (event) {
      case 'chunk': handlers.onChunk?.(parsed); break;
      case 'status': handlers.onStatus?.(parsed); break;
      case 'done': handlers.onDone?.(parsed); break;
      case 'error': handlers.onError?.(parsed); break;
      case 'ping': handlers.onPing?.(); break;
    }
  } catch {
    // Non-JSON data (shouldn't happen with our API, but be defensive)
    console.warn('SSE: Failed to parse event data', event, data.slice(0, 100));
  }
}
```

### Session Persistence (localStorage)

The Web UI stores the `session_id` in `localStorage` so the learner can refresh the page or close the browser without losing their session:

```typescript
// hooks/useSession.ts
const SESSION_STORAGE_KEY = 'blup_session';

function useSession() {
  const [sessionId, setSessionId] = useState<string | null>(() => {
    // Restore from localStorage on mount
    return localStorage.getItem(SESSION_STORAGE_KEY);
  });
  const [state, setState] = useState<SessionState>('IDLE');
  const [error, setError] = useState<ApiError | null>(null);

  // Sync to localStorage whenever sessionId changes
  useEffect(() => {
    if (sessionId) {
      localStorage.setItem(SESSION_STORAGE_KEY, sessionId);
    } else {
      localStorage.removeItem(SESSION_STORAGE_KEY);
    }
  }, [sessionId]);

  // On mount: if we have a stored sessionId, try to resume
  useEffect(() => {
    if (sessionId) {
      resumeSession(sessionId);
    } else {
      createNewSession();
    }
  }, []);

  async function createNewSession() {
    const response = await api.createSession();
    setSessionId(response.session_id);
    setState('IDLE');
    setError(null);
  }

  async function resumeSession(id: string) {
    try {
      // GET /api/session/{id} — returns current state
      const session = await api.getSession(id);
      setSessionId(session.id);
      setState(session.state);
      setError(null);
    } catch (err) {
      // Session expired or server restarted → create new
      localStorage.removeItem(SESSION_STORAGE_KEY);
      setSessionId(null);
      createNewSession();
    }
  }

  // ... submitGoal, answerProfile, etc.
}
```

### StreamingMessage Component

The `StreamingMessage` component renders an assistant message as chunks arrive via SSE:

```typescript
// components/chat/StreamingMessage.tsx
function StreamingMessage({ chunks, isComplete }: StreamingMessageProps) {
  const contentRef = useRef<HTMLDivElement>(null);
  const [renderedContent, setRenderedContent] = useState('');

  // Accumulate chunks into rendered Markdown
  useEffect(() => {
    const fullText = chunks.map(c => c.content).join('');
    setRenderedContent(fullText);
  }, [chunks]);

  // Auto-scroll as content grows
  useEffect(() => {
    if (contentRef.current) {
      contentRef.current.scrollIntoView({ behavior: 'smooth', block: 'end' });
    }
  }, [renderedContent]);

  return (
    <div className={`message assistant-message ${isComplete ? 'complete' : 'streaming'}`}
         role="article"
         aria-label="Assistant response">
      <div className="message-avatar" aria-hidden="true">🤖</div>
      <div className="message-body" ref={contentRef}>
        {isComplete ? (
          <MarkdownRenderer content={renderedContent} />
        ) : (
          <>
            <MarkdownRenderer content={renderedContent} />
            <span className="cursor-blink" aria-hidden="true">▌</span>
          </>
        )}
      </div>
    </div>
  );
}
```

### ChatWindow Auto-Scroll

```typescript
// components/chat/ChatWindow.tsx
function ChatWindow({ messages, streamingMessage }: ChatWindowProps) {
  const bottomRef = useRef<HTMLDivElement>(null);
  const [userScrolledUp, setUserScrolledUp] = useState(false);
  const containerRef = useRef<HTMLDivElement>(null);

  // Auto-scroll to bottom unless user scrolled up manually
  const handleScroll = useCallback(() => {
    if (!containerRef.current) return;
    const { scrollTop, scrollHeight, clientHeight } = containerRef.current;
    const isAtBottom = scrollHeight - scrollTop - clientHeight < 50;
    setUserScrolledUp(!isAtBottom);
  }, []);

  // Scroll to bottom on new messages (unless user scrolled up)
  useEffect(() => {
    if (!userScrolledUp) {
      bottomRef.current?.scrollIntoView({ behavior: 'smooth' });
    }
  }, [messages.length, streamingMessage?.chunks.length, userScrolledUp]);

  // Show "scroll to bottom" button when user scrolled up and new content arrives
  const showScrollButton = userScrolledUp &&
    (messages.length > 0 || (streamingMessage?.chunks.length ?? 0) > 0);

  return (
    <div className="chat-window" ref={containerRef} onScroll={handleScroll}>
      <MessageList messages={messages} />
      {streamingMessage && <StreamingMessage {...streamingMessage} />}
      <div ref={bottomRef} />

      {showScrollButton && (
        <button
          className="scroll-to-bottom"
          onClick={() => {
            bottomRef.current?.scrollIntoView({ behavior: 'smooth' });
            setUserScrolledUp(false);
          }}
          aria-label="Scroll to latest message"
        >
          ↓
        </button>
      )}
    </div>
  );
}
```

### MarkdownRenderer with Syntax Highlighting

```typescript
// components/content/MarkdownRenderer.tsx
function MarkdownRenderer({ content }: { content: string }) {
  return (
    <ReactMarkdown
      remarkPlugins={[remarkMath, remarkGfm]}
      rehypePlugins={[rehypeKatex, rehypeSanitize]}
      components={{
        // Code blocks: use CodeMirror 6 for read-only syntax highlighting
        code({ node, inline, className, children, ...props }) {
          const match = /language-(\w+)/.exec(className || '');
          const language = match ? match[1] : '';
          const codeString = String(children).replace(/\n$/, '');

          if (!inline && language) {
            return <CodeBlock language={language} code={codeString} />;
          }

          // Inline code
          return (
            <code className="inline-code" {...props}>
              {children}
            </code>
          );
        },

        // Tables: add responsive wrapper
        table({ children }) {
          return (
            <div className="table-wrapper">
              <table>{children}</table>
            </div>
          );
        },

        // Links: external → new tab with security attributes
        a({ href, children }) {
          const isExternal = href?.startsWith('http');
          return (
            <a
              href={href}
              target={isExternal ? '_blank' : undefined}
              rel={isExternal ? 'noopener noreferrer' : undefined}
            >
              {children}
            </a>
          );
        },

        // Images: only allow from assets/ directory
        img({ src, alt }) {
          if (!src?.startsWith('/assets/') && !src?.startsWith('data:')) {
            return null; // Block external images
          }
          return <img src={src} alt={alt || ''} loading="lazy" />;
        },
      }}
    >
      {content}
    </ReactMarkdown>
  );
}

// CodeBlock with CodeMirror 6 (read-only)
function CodeBlock({ language, code }: { language: string; code: string }) {
  const editorRef = useRef<HTMLDivElement>(null);
  const viewRef = useRef<EditorView | null>(null);

  useEffect(() => {
    if (!editorRef.current) return;

    // Determine language extension
    const langExt = getLanguageExtension(language);

    const state = EditorState.create({
      doc: code,
      extensions: [
        EditorView.editable.of(false), // Read-only
        langExt ? langExt() : [],
        minimalSetup,
        EditorView.theme({
          '&': { maxHeight: '400px', overflow: 'auto' },
          '.cm-content': { fontFamily: "'JetBrains Mono', monospace", fontSize: '13px' },
        }),
      ],
    });

    const view = new EditorView({
      state,
      parent: editorRef.current,
    });

    viewRef.current = view;

    return () => view.destroy();
  }, [language, code]);

  return (
    <div className="code-block" role="region" aria-label={`${language} code block`}>
      <div className="code-block-header">
        <span className="language-label">{language}</span>
        <CopyButton text={code} />
      </div>
      <div ref={editorRef} />
    </div>
  );
}

// Lazy-load language support
function getLanguageExtension(lang: string): (() => Extension) | null {
  const langMap: Record<string, () => Promise<Extension>> = {
    python: () => import('@codemirror/lang-python').then(m => m.python()),
    javascript: () => import('@codemirror/lang-javascript').then(m => m.javascript()),
    typescript: () => import('@codemirror/lang-javascript').then(m => m.javascript({ typescript: true })),
    rust: () => import('@codemirror/lang-rust').then(m => m.rust()),
    sql: () => import('@codemirror/lang-sql').then(m => m.sql()),
    html: () => import('@codemirror/lang-html').then(m => m.html()),
    css: () => import('@codemirror/lang-css').then(m => m.css()),
    json: () => import('@codemirror/lang-json').then(m => m.json()),
  };
  return langMap[lang] || null;
}
```

### Package Dependencies

```json
{
  "name": "blup-web-ui",
  "private": true,
  "version": "0.1.0",
  "type": "module",
  "scripts": {
    "dev": "vite",
    "build": "tsc && vite build",
    "preview": "vite preview",
    "lint": "eslint src/",
    "typecheck": "tsc --noEmit",
    "test": "vitest run",
    "test:watch": "vitest",
    "e2e": "playwright test"
  },
  "dependencies": {
    "react": "^18.3.0",
    "react-dom": "^18.3.0",
    "react-markdown": "^9.0.0",
    "remark-math": "^6.0.0",
    "rehype-katex": "^7.0.0",
    "katex": "^0.16.0",
    "@codemirror/view": "^6.30.0",
    "@codemirror/state": "^6.4.0",
    "@codemirror/lang-python": "^6.1.0",
    "@codemirror/lang-javascript": "^6.2.0",
    "@codemirror/lang-rust": "^6.0.0",
    "zustand": "^4.5.0"
  },
  "devDependencies": {
    "typescript": "^5.5.0",
    "vite": "^5.4.0",
    "@vitejs/plugin-react": "^4.3.0",
    "vitest": "^2.0.0",
    "@testing-library/react": "^16.0.0",
    "@testing-library/jest-dom": "^6.5.0",
    "playwright": "^1.45.0",
    "eslint": "^9.0.0",
    "@types/react": "^18.3.0",
    "@types/react-dom": "^18.3.0",
    "@types/katex": "^0.16.0"
  }
}
```

### State Management

Use **Zustand** for lightweight state management (no Redux overhead for Phase 1):

```typescript
// state/sessionStore.ts (conceptual)
interface SessionStore {
  sessionId: string | null;
  state: SessionState;
  error: ApiError | null;

  // Actions
  createSession: () => Promise<void>;
  submitGoal: (goal: LearningGoal) => Promise<void>;
  answerProfile: (answer: ProfileAnswer) => Promise<void>;
  selectChapter: (chapterId: string) => Promise<void>;
  askQuestion: (question: string) => Promise<void>;
  completeChapter: () => Promise<void>;
  resetSession: () => void;
}
```

### API Client

```typescript
// api/client.ts (conceptual)
const BASE_URL = import.meta.env.VITE_API_URL || 'http://localhost:3000';

class ApiClient {
  private async request<T>(method: string, path: string, body?: unknown): Promise<T> {
    const res = await fetch(`${BASE_URL}${path}`, {
      method,
      headers: body ? { 'Content-Type': 'application/json' } : undefined,
      body: body ? JSON.stringify(body) : undefined,
    });

    if (!res.ok) {
      const error = await res.json();
      throw new ApiError(error.error.code, error.error.message, res.status);
    }

    return res.json();
  }

  async createSession(): Promise<{ session_id: string; state: string }> {
    return this.request('POST', '/api/session');
  }

  submitGoal(sessionId: string, goal: LearningGoal): SSEClient {
    const client = new SSEClient();
    client.connect(`${BASE_URL}/api/session/${sessionId}/goal`, {
      onChunk: (data) => { /* update streaming message */ },
      onStatus: (data) => { /* update session state */ },
      onDone: (data) => { /* set feasibility result */ },
      onError: (data) => { /* show error */ },
    });
    // Actually send the POST body... SSE + POST body requires fetch API, not EventSource.
    // See SSE + POST section below.
    return client;
  }
}
```

### SSE + POST Challenge

`EventSource` only supports GET. For SSE endpoints that need a POST body (goal submission, profile answers, question asking), use the **Fetch API with ReadableStream**:

```typescript
// api/sse.ts — POST-based SSE using fetch
async function postSSE(
  url: string,
  body: unknown,
  handlers: SSEHandlers
): Promise<AbortController> {
  const controller = new AbortController();

  const response = await fetch(url, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(body),
    signal: controller.signal,
  });

  const reader = response.body!.getReader();
  const decoder = new TextDecoder();
  let buffer = '';

  while (true) {
    const { done, value } = await reader.read();
    if (done) break;

    buffer += decoder.decode(value, { stream: true });
    const lines = buffer.split('\n');
    buffer = lines.pop() || '';

    for (const line of lines) {
      if (line.startsWith('event: ')) {
        const eventType = line.slice(7).trim();
        // Next line should be "data: ..."
        // Parse and dispatch to handlers
      }
    }
  }

  return controller; // Caller can abort
}
```

### Markdown Rendering

The `MarkdownRenderer` component handles:
- **CommonMark** via `react-markdown`.
- **Math** via `remark-math` + `rehype-katex` — inline `$...$` and block `$$...$$`.
- **Code blocks** via a custom `code` component that renders fenced code blocks with CodeMirror 6 for read-only syntax highlighting (no editor needed). Inline code uses a simple `<code>` tag.
- **Images** — disabled by default (security); allow-listed only from `assets/`.
- **Links** — rendered but with `rel="noopener noreferrer"` and `target="_blank"`.
- **HTML in Markdown** — stripped (`rehype-sanitize` or `allowDangerousHtml: false`).

```typescript
// components/content/MarkdownRenderer.tsx (conceptual)
function MarkdownRenderer({ content }: { content: string }) {
  return (
    <ReactMarkdown
      remarkPlugins={[remarkMath]}
      rehypePlugins={[rehypeKatex, rehypeSanitize]}
      components={{
        code({ node, inline, className, children, ...props }) {
          if (inline) {
            return <code className="inline-code" {...props}>{children}</code>;
          }
          const language = className?.replace('language-', '') || '';
          return <CodeBlock language={language} code={String(children)} />;
        },
        img({ src, alt }) {
          // Only allow images from assets/ directory
          if (!src?.startsWith('/assets/')) return null;
          return <img src={src} alt={alt} loading="lazy" />;
        },
      }}
    >
      {content}
    </ReactMarkdown>
  );
}
```

### Error Handling

The UI must handle these error categories:

| Error | UI Treatment |
|-------|-------------|
| Network error (fetch failed) | "Connection lost" with retry button |
| HTTP 4xx (invalid transition, bad request) | Inline error message with suggested action |
| HTTP 5xx (server error) | "Something went wrong" with retry button |
| SSE stream error (connection lost mid-stream) | "Stream interrupted" with reconnect |
| Schema validation error (in API response) | "Unexpected response format" — log diagnostic ID |
| Timeout (no response) | "Taking longer than expected" with cancel option |

The `ErrorDisplay` component:

```typescript
function ErrorDisplay({ error, onRetry, onReset }: ErrorDisplayProps) {
  return (
    <div className="error-display" role="alert">
      <h3>{getErrorTitle(error.code)}</h3>
      <p>{error.message}</p>
      {error.code !== 'INVALID_STATE_TRANSITION' && (
        <Button onClick={onRetry}>Retry</Button>
      )}
      <Button variant="secondary" onClick={onReset}>Start Over</Button>
      {error.diagnosticId && (
        <code className="diagnostic-id">ID: {error.diagnosticId}</code>
      )}
    </div>
  );
}
```

### Accessibility Requirements

- All interactive elements are keyboard-navigable (Tab, Enter, Escape).
- Chat messages use `role="log"` with `aria-live="polite"` for new messages.
- Streaming messages use `aria-live="assertive"` (but debounced to avoid spamming screen readers).
- Code blocks have `role="region"` with `aria-label` including language.
- Math formulas have `aria-label` with textual description (KaTeX provides this).
- Color is never the sole indicator of state (use icons + text).
- Focus management: focus moves to new content on state transitions.
- Skip link for keyboard users to jump past sidebar to main content.

### Responsive Design

Three breakpoints:
- **Mobile** (< 768px): Chat full width; sidebar hidden (hamburger toggle).
- **Tablet** (768px-1024px): Sidebar collapsible; content + chat stacked.
- **Desktop** (> 1024px): Three-column: sidebar | content | chat.

CSS uses CSS custom properties for theming (light mode only for Phase 1; dark mode deferred).

### Testing Strategy

| Test Category | Tool | Scope |
|---------------|------|-------|
| Component tests | Vitest + Testing Library | Chat, sidebar, content, error display |
| Hook tests | Vitest | useSession, useSSE, useChat |
| SSE mock tests | Vitest + MSW | Mock SSE streams with test fixtures |
| Integration tests | Vitest + Testing Library | Full user journey: goal → curriculum → chapter |
| Accessibility | axe-core + Vitest | Keyboard nav, ARIA attributes, color contrast |
| E2E tests | Playwright | Smoke test: create session, submit goal, see result |
| Type checking | `tsc --noEmit` | Zero type errors before merge |

### Quality Gates

- [ ] `npm run lint` passes with no errors
- [ ] `npm run typecheck` passes with no errors
- [ ] `npm run test` passes all component and hook tests
- [ ] All states render correctly (IDLE, GOAL_INPUT, FEASIBILITY_CHECK, PROFILE_COLLECTION, CURRICULUM_PLANNING, CHAPTER_LEARNING, COMPLETED, ERROR)
- [ ] SSE streaming works (chunk display, done transition, error recovery)
- [ ] SSE POST endpoints work via fetch + ReadableStream
- [ ] Chat auto-scrolls to new messages
- [ ] Curriculum sidebar navigation works
- [ ] Markdown, KaTeX, and CodeMirror render correctly
- [ ] Error states show retry/reset actions
- [ ] No API keys or secrets in browser storage
- [ ] No direct LLM API calls from UI code
- [ ] Accessibility: keyboard navigation, ARIA labels, focus management

### Risk Register

| Risk | Impact | Mitigation |
|------|--------|------------|
| EventSource doesn't support POST | Can't send goal/profile/question via SSE | Use fetch + ReadableStream for POST-based SSE |
| Markdown rendering performance | Laggy UI with large chapter content | Virtualize content area; paginate long chapters |
| KaTeX rendering errors on malformed LaTeX | Broken math display | Catch KaTeX errors, display raw LaTeX as fallback |
| SSE reconnect loses messages | Learner misses content | Replay buffer on server; Last-Event-ID in client |
| CodeMirror 6 bundle size | Slow initial load (~200KB) | Dynamic import for CodeBlock; only load language support for active code block |
| Browser back/forward breaks session flow | Confusing UX | Single-page navigation only; warn on unload if session active |
