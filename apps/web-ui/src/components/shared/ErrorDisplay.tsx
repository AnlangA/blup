import { useCreatePlan } from '../../hooks/query';

export function ErrorDisplay() {
  const createPlan = useCreatePlan();

  return (
    <div className="error-display" role="alert">
      <h3>Something went wrong</h3>
      <p>An unexpected error occurred. Please try again.</p>
      <div className="error-actions">
        <button
          onClick={() => createPlan.mutate()}
          disabled={createPlan.isPending}
        >
          Retry
        </button>
      </div>
    </div>
  );
}
