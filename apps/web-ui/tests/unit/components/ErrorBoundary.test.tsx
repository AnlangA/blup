import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { ErrorBoundary } from '../../../src/components/shared/ErrorBoundary';

const renderWithProviders = (ui: React.ReactElement) => {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  return render(
    <QueryClientProvider client={queryClient}>{ui}</QueryClientProvider>,
  );
};

function ThrowingComponent(): React.ReactElement {
  throw new Error('Test error');
}

function NormalComponent(): React.ReactElement {
  return <div>Normal content</div>;
}

describe('ErrorBoundary', () => {
  it('renders children when no error', () => {
    renderWithProviders(
      <ErrorBoundary>
        <NormalComponent />
      </ErrorBoundary>,
    );
    expect(screen.getByText('Normal content')).toBeInTheDocument();
  });

  it('renders error UI when child throws', () => {
    // Suppress console.error for expected errors
    const spy = vi.spyOn(console, 'error').mockImplementation(() => {});
    renderWithProviders(
      <ErrorBoundary>
        <ThrowingComponent />
      </ErrorBoundary>,
    );
    expect(screen.getByText('Something went wrong')).toBeInTheDocument();
    expect(screen.getByText('Test error')).toBeInTheDocument();
    spy.mockRestore();
  });

  it('renders custom fallback when provided', () => {
    const spy = vi.spyOn(console, 'error').mockImplementation(() => {});
    renderWithProviders(
      <ErrorBoundary fallback={<div>Custom fallback</div>}>
        <ThrowingComponent />
      </ErrorBoundary>,
    );
    expect(screen.getByText('Custom fallback')).toBeInTheDocument();
    spy.mockRestore();
  });

  it('has a reload button', () => {
    const spy = vi.spyOn(console, 'error').mockImplementation(() => {});
    renderWithProviders(
      <ErrorBoundary>
        <ThrowingComponent />
      </ErrorBoundary>,
    );
    expect(screen.getByRole('button', { name: /reload/i })).toBeInTheDocument();
    spy.mockRestore();
  });
});
