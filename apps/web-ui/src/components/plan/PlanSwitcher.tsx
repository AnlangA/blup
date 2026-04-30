import { useState } from "react";
import { useSessionStore } from "../../state/sessionStore";
import { useCreatePlan, useDeletePlan } from "../../hooks/query";
import { PlanCard } from "./PlanCard";

export function PlanSwitcher() {
  const plans = useSessionStore((s) => s.plans);
  const activePlanId = useSessionStore((s) => s.activePlanId);
  const setActivePlan = useSessionStore((s) => s.setActivePlan);
  const createPlan = useCreatePlan();
  const deletePlan = useDeletePlan();
  const [confirmDeleteId, setConfirmDeleteId] = useState<string | null>(null);
  const [collapsed, setCollapsed] = useState(false);

  const handleDelete = (id: string) => {
    if (confirmDeleteId === id) {
      deletePlan.mutate(id);
      setConfirmDeleteId(null);
    } else {
      setConfirmDeleteId(id);
      setTimeout(() => setConfirmDeleteId(null), 3000);
    }
  };

  const handleCreate = () => {
    if (!createPlan.isPending) {
      createPlan.mutate();
    }
  };

  if (collapsed) {
    return (
      <aside className="plan-switcher collapsed">
        <button
          className="plan-expand-btn"
          onClick={() => setCollapsed(false)}
          aria-label="Expand plan list"
        >
          <span className="plan-expand-icon">&#9654;</span>
        </button>
      </aside>
    );
  }

  return (
    <aside className="plan-switcher">
      <div className="plan-switcher-header">
        <h2>Plans</h2>
        <button
          className="plan-collapse-btn"
          onClick={() => setCollapsed(true)}
          aria-label="Collapse plan list"
        >
          &#9664;
        </button>
      </div>
      <button
        className="plan-new-btn"
        onClick={handleCreate}
        disabled={createPlan.isPending}
      >
        {createPlan.isPending ? "Creating..." : "+ New Plan"}
      </button>
      <div className="plan-list">
        {plans.length === 0 && (
          <div className="plan-list-empty">
            No plans yet. Create one to start learning.
          </div>
        )}
        {plans.map((plan) => (
          <PlanCard
            key={plan.id}
            plan={plan}
            isActive={plan.id === activePlanId}
            onSelect={() => setActivePlan(plan.id)}
            onDelete={() => handleDelete(plan.id)}
          />
        ))}
        {confirmDeleteId && (
          <div className="plan-delete-confirm">
            Click again to confirm delete
          </div>
        )}
      </div>
    </aside>
  );
}
