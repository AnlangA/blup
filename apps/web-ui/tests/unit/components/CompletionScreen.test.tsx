import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { CompletionScreen } from '../../../src/components/session/CompletionScreen';

vi.mock('../../../src/state/sessionStore', () => ({
  useSessionStore: vi.fn((selector?: (s: unknown) => unknown) => {
    const state = { sessionId: 'test-session-id' };
    return selector ? selector(state) : state;
  }),
}));

vi.mock('../../../src/hooks/query', () => ({
  useCurriculum: () => ({
    data: {
      title: 'Test Curriculum',
      chapters: [
        { id: 'ch1', title: 'Chapter 1', order: 1, objectives: [] },
        { id: 'ch2', title: 'Chapter 2', order: 2, objectives: [] },
      ],
    },
  }),
  useCreatePlan: () => ({
    mutate: vi.fn(),
    isPending: false,
  }),
  useExportChapterTypst: () => ({
    mutate: vi.fn(),
    isPending: false,
    error: null,
  }),
  useExportCurriculumTypst: () => ({
    mutate: vi.fn(),
    isPending: false,
    error: null,
  }),
  useExportChapterPdf: () => ({
    exportPdf: vi.fn(),
    isExporting: false,
    error: null,
    status: null,
    message: null,
    reset: vi.fn(),
  }),
  useExportCurriculumPdf: () => ({
    exportPdf: vi.fn(),
    isExporting: false,
    error: null,
    status: null,
    message: null,
    reset: vi.fn(),
  }),
}));

function renderWithProviders(ui: React.ReactElement) {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  return render(
    <QueryClientProvider client={queryClient}>{ui}</QueryClientProvider>,
  );
}

describe('CompletionScreen', () => {
  it('renders congratulations message', () => {
    renderWithProviders(<CompletionScreen />);
    expect(screen.getByText('Congratulations!')).toBeInTheDocument();
  });

  it('shows chapter count', () => {
    renderWithProviders(<CompletionScreen />);
    expect(screen.getByText(/2 chapters completed/)).toBeInTheDocument();
  });

  it('renders new plan button', () => {
    renderWithProviders(<CompletionScreen />);
    expect(
      screen.getByRole('button', { name: /start a new learning goal/i }),
    ).toBeInTheDocument();
  });
});
