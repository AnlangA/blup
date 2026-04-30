import type { PlanMeta } from "../../state/sessionStore";

interface PlanCardProps {
  plan: PlanMeta;
  isActive: boolean;
  onSelect: () => void;
  onDelete: () => void;
}

const STATE_COLORS: Record<string, string> = {
  IDLE: "var(--plan-badge-idle)",
  GOAL_INPUT: "var(--plan-badge-idle)",
  FEASIBILITY_CHECK: "var(--plan-badge-progress)",
  PROFILE_COLLECTION: "var(--plan-badge-progress)",
  CURRICULUM_PLANNING: "var(--plan-badge-progress)",
  CHAPTER_LEARNING: "var(--plan-badge-active)",
  COMPLETED: "var(--plan-badge-done)",
  ERROR: "var(--plan-badge-error)",
};

const STATE_LABELS: Record<string, string> = {
  IDLE: "New",
  GOAL_INPUT: "Draft",
  FEASIBILITY_CHECK: "Checking",
  PROFILE_COLLECTION: "Profile",
  CURRICULUM_PLANNING: "Planning",
  CHAPTER_LEARNING: "Learning",
  COMPLETED: "Done",
  ERROR: "Error",
};

function relativeTime(dateStr: string): string {
  if (!dateStr) return "";
  const now = Date.now();
  const then = new Date(dateStr).getTime();
  const diffMs = now - then;
  const mins = Math.floor(diffMs / 60000);
  if (mins < 1) return "just now";
  if (mins < 60) return `${mins}m ago`;
  const hours = Math.floor(mins / 60);
  if (hours < 24) return `${hours}h ago`;
  const days = Math.floor(hours / 24);
  return `${days}d ago`;
}

export function PlanCard({
  plan,
  isActive,
  onSelect,
  onDelete,
}: PlanCardProps) {
  const badgeColor = STATE_COLORS[plan.state] || "var(--plan-badge-idle)";
  const stateLabel = STATE_LABELS[plan.state] || plan.state;

  return (
    <div
      className={`plan-card ${isActive ? "active" : ""}`}
      onClick={onSelect}
      role="button"
      tabIndex={0}
      onKeyDown={(e) => e.key === "Enter" && onSelect()}
    >
      <div className="plan-card-indicator" style={{ background: badgeColor }} />
      <div className="plan-card-body">
        <div className="plan-card-title" title={plan.title}>
          {plan.title}
        </div>
        <div className="plan-card-meta">
          {plan.domain && (
            <span className="plan-card-domain">{plan.domain}</span>
          )}
          <span className="plan-card-badge" style={{ color: badgeColor }}>
            {stateLabel}
          </span>
          <span className="plan-card-time">
            {relativeTime(plan.updatedAt)}
          </span>
        </div>
      </div>
      <button
        className="plan-card-delete"
        onClick={(e) => {
          e.stopPropagation();
          onDelete();
        }}
        title="Delete plan"
        aria-label={`Delete ${plan.title}`}
      >
        &times;
      </button>
    </div>
  );
}
