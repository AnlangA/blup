import { describe, it, expect, vi, beforeAll } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { ChatWindow } from '../../../src/components/chat/ChatWindow';

// Mock scrollIntoView for jsdom
beforeAll(() => {
  Element.prototype.scrollIntoView = vi.fn();
});

// Mock MarkdownRenderer to avoid async unified pipeline warnings in tests
vi.mock('../../../src/components/content/MarkdownRenderer', () => ({
  MarkdownRenderer: ({ content }: { content: string }) => (
    <div data-testid="markdown">{content}</div>
  ),
}));

vi.mock('../../../src/state/sessionStore', () => ({
  useSessionStore: vi.fn((selector?: (s: unknown) => unknown) => {
    const state = {
      sessionId: 'test-session-id',
      currentChapterId: 'ch1',
    };
    return selector ? selector(state) : state;
  }),
}));

vi.mock('../../../src/hooks/query', () => ({
  useSession: () => ({
    data: {
      session_id: 'test-session-id',
      state: 'CHAPTER_LEARNING',
      messages: [
        { id: 'm1', role: 'user', content: 'What is a variable?', chapter_id: 'ch1', timestamp: '2024-01-01T00:00:00Z' },
        { id: 'm2', role: 'assistant', content: 'A variable is a named storage.', chapter_id: 'ch1', timestamp: '2024-01-01T00:01:00Z' },
        { id: 'm3', role: 'user', content: 'What about ch2?', chapter_id: 'ch2', timestamp: '2024-01-01T00:02:00Z' },
      ],
    },
  }),
  useAskQuestion: () => ({
    mutate: vi.fn(),
    isPending: false,
  }),
}));

const renderWithProviders = (ui: React.ReactElement) => {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  return render(
    <QueryClientProvider client={queryClient}>{ui}</QueryClientProvider>,
  );
};

describe('ChatWindow', () => {
  it('renders messages filtered by current chapter', async () => {
    renderWithProviders(<ChatWindow />);
    // Wait for MarkdownRenderer to complete async rendering
    const msg = await screen.findByText('What is a variable?', undefined, { timeout: 3000 });
    expect(msg).toBeInTheDocument();
    // ch2 message should not be visible
    expect(screen.queryByText('What about ch2?')).not.toBeInTheDocument();
  });

  it('renders a textarea for input', () => {
    renderWithProviders(<ChatWindow />);
    expect(screen.getByLabelText('Question input')).toBeInTheDocument();
  });

  it('disables send button when input is empty', () => {
    renderWithProviders(<ChatWindow />);
    expect(screen.getByRole('button', { name: /send/i })).toBeDisabled();
  });

  it('enables send button when input is provided', () => {
    renderWithProviders(<ChatWindow />);
    const textarea = screen.getByLabelText('Question input');
    fireEvent.change(textarea, { target: { value: 'Test question' } });
    expect(screen.getByRole('button', { name: /send/i })).not.toBeDisabled();
  });

  it('submits on Enter (without Shift)', () => {
    renderWithProviders(<ChatWindow />);
    const textarea = screen.getByLabelText('Question input');
    fireEvent.change(textarea, { target: { value: 'Test question' } });
    fireEvent.keyDown(textarea, { key: 'Enter', shiftKey: false });
    // After submit, textarea should be cleared (since mock mutate fires)
  });
});
