export interface SSEEvent {
  type: string;
  data: unknown;
}

export interface SSEHandlers {
  onChunk?: (content: string, index: number) => void;
  onStatus?: (state: string, message: string) => void;
  onDone?: (result: unknown) => void;
  onError?: (code: string, message: string) => void;
  onPing?: () => void;
  onStdout?: (content: string) => void;
  onStderr?: (content: string) => void;
}

/**
 * Client for Server-Sent Events, supporting both GET (EventSource) and
 * POST (fetch + ReadableStream) modes.
 */
export class SSEClient {
  private eventSource: EventSource | null = null;
  private abortController: AbortController | null = null;
  private lastEventId: string | null = null;
  private reconnectAttempt = 0;
  private maxReconnectAttempts = 5;
  private baseReconnectMs = 1000;
  private maxReconnectMs = 30000;
  private reconnectTimer: ReturnType<typeof setTimeout> | null = null;
  private isDisposed = false;

  /** Connect via GET (uses browser EventSource with auto-reconnect). */
  connectGet(url: string, handlers: SSEHandlers): void {
    this.isDisposed = false;
    this.doConnectGet(url, handlers);
  }

  private doConnectGet(url: string, handlers: SSEHandlers): void {
    const fullUrl = this.lastEventId
      ? `${url}${url.includes('?') ? '&' : '?'}_lastEventId=${this.lastEventId}`
      : url;

    const es = new EventSource(fullUrl);
    this.eventSource = es;

    es.addEventListener('chunk', (e: MessageEvent) => {
      const d = JSON.parse(e.data);
      this.lastEventId = (e as unknown as { lastEventId: string }).lastEventId;
      this.reconnectAttempt = 0;
      handlers.onChunk?.(d.content, d.index ?? 0);
    });

    es.addEventListener('status', (e: MessageEvent) => {
      const d = JSON.parse(e.data);
      handlers.onStatus?.(d.state, d.message);
    });

    es.addEventListener('done', (e: MessageEvent) => {
      handlers.onDone?.(JSON.parse(e.data));
      this.close();
    });

    es.addEventListener('error', (e: Event) => {
      const msgEvent = e as MessageEvent;
      // Named SSE "error" event from the server (has data payload)
      if (msgEvent.data) {
        try {
          const d = JSON.parse(msgEvent.data);
          handlers.onError?.(d.code, d.message);
          return;
        } catch {
          // Malformed data — fall through to reconnect
        }
      }
      // Connection-level error (server closed stream, network issue, etc.)
      this.handleReconnect(url, handlers);
    });

    es.addEventListener('ping', () => {
      handlers.onPing?.();
    });

    es.addEventListener('stdout', (e: MessageEvent) => {
      const d = JSON.parse(e.data);
      handlers.onStdout?.(d.content);
    });

    es.addEventListener('stderr', (e: MessageEvent) => {
      const d = JSON.parse(e.data);
      handlers.onStderr?.(d.content);
    });
  }

  private handleReconnect(url: string, handlers: SSEHandlers): void {
    if (this.isDisposed) return;
    this.eventSource?.close();
    this.eventSource = null;
    this.reconnectAttempt++;

    if (this.reconnectAttempt <= this.maxReconnectAttempts) {
      const delay = Math.min(
        this.baseReconnectMs * Math.pow(2, this.reconnectAttempt - 1),
        this.maxReconnectMs,
      ) + Math.random() * 1000;

      console.warn(
        `SSE reconnect ${this.reconnectAttempt}/${this.maxReconnectAttempts} in ${Math.round(delay)}ms`,
      );

      this.reconnectTimer = setTimeout(() => {
        this.doConnectGet(url, handlers);
      }, delay);
    } else {
      handlers.onError?.(
        'SSE_RECONNECT_FAILED',
        `Failed to reconnect after ${this.maxReconnectAttempts} attempts.`,
      );
    }
  }

  /** Connect via POST using fetch + ReadableStream. */
  async connectPost(url: string, body: unknown, handlers: SSEHandlers): Promise<void> {
    this.isDisposed = false;
    this.abortController = new AbortController();

    const response = await fetch(url, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        Accept: 'text/event-stream',
        'Last-Event-ID': this.lastEventId || '',
      },
      body: JSON.stringify(body),
      signal: this.abortController.signal,
    });

    if (!response.ok) {
      const err = await response.json().catch(() => ({}));
      handlers.onError?.(
        err.error?.code || 'HTTP_ERROR',
        err.error?.message || `HTTP ${response.status}`,
      );
      return;
    }

    if (!response.body) {
      handlers.onError?.('NO_BODY', 'Response has no body');
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
        buffer = lines.pop() || '';

        for (const line of lines) {
          if (line.startsWith('event: ')) {
            currentEvent = line.slice(7).trim();
          } else if (line.startsWith('data: ')) {
            currentData = line.slice(6);
          } else if (line === '' && currentEvent) {
            this.dispatch(currentEvent, currentData, handlers);
            currentEvent = '';
            currentData = '';
          }
        }
      }
    } catch (err) {
      if ((err as Error).name === 'AbortError') return;
      handlers.onError?.('STREAM_ERROR', (err as Error).message);
    } finally {
      reader.releaseLock();
    }
  }

  private dispatch(event: string, data: string, handlers: SSEHandlers): void {
    try {
      const parsed = JSON.parse(data);
      switch (event) {
        case 'chunk':
          handlers.onChunk?.(parsed.content, parsed.index ?? 0);
          break;
        case 'status':
          handlers.onStatus?.(parsed.state, parsed.message);
          break;
        case 'done':
          handlers.onDone?.(parsed.result ?? parsed);
          break;
        case 'error':
          handlers.onError?.(parsed.code, parsed.message);
          break;
        case 'ping':
          handlers.onPing?.();
          break;
        case 'stdout':
          handlers.onStdout?.(parsed.content);
          break;
        case 'stderr':
          handlers.onStderr?.(parsed.content);
          break;
      }
    } catch {
      console.warn('SSE: Failed to parse event data', event, data.slice(0, 100));
    }
  }

  /** Abort any active connection. */
  close(): void {
    this.isDisposed = true;
    if (this.reconnectTimer !== null) {
      clearTimeout(this.reconnectTimer);
      this.reconnectTimer = null;
    }
    this.eventSource?.close();
    this.eventSource = null;
    this.abortController?.abort();
    this.abortController = null;
    this.lastEventId = null;
    this.reconnectAttempt = 0;
  }
}

export const sseClient = new SSEClient();
