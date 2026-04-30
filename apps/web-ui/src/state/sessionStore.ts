import { create } from "zustand";

export type CodeTheme = "github-dark" | "github-light";

export interface PlanMeta {
  id: string;
  title: string;
  domain: string;
  state: string;
  createdAt: string;
  updatedAt: string;
}

interface SessionStore {
  plans: PlanMeta[];
  activePlanId: string | null;
  currentChapterId: string | null;
  codeTheme: CodeTheme;

  // Derived
  sessionId: string | null;

  // Actions
  addPlan: (plan: PlanMeta) => void;
  removePlan: (id: string) => void;
  setActivePlan: (id: string) => void;
  updatePlanMeta: (id: string, patch: Partial<PlanMeta>) => void;
  setChapter: (chapterId: string) => void;
  setCodeTheme: (theme: CodeTheme) => void;
  reset: () => void;
}

const PLANS_KEY = "blup_plans";
const ACTIVE_PLAN_KEY = "blup_active_plan_id";
const CHAPTER_KEY = "blup_current_chapter_id";
const THEME_KEY = "blup_code_theme";

function loadPlans(): PlanMeta[] {
  try {
    const raw = localStorage.getItem(PLANS_KEY);
    return raw ? JSON.parse(raw) : [];
  } catch {
    return [];
  }
}

function savePlans(plans: PlanMeta[]) {
  localStorage.setItem(PLANS_KEY, JSON.stringify(plans));
}

export const useSessionStore = create<SessionStore>((set, get) => {
  const initialPlans = loadPlans();
  const initialActiveId = localStorage.getItem(ACTIVE_PLAN_KEY);
  const initialChapter = localStorage.getItem(CHAPTER_KEY);
  const initialTheme =
    (localStorage.getItem(THEME_KEY) as CodeTheme) || "github-dark";

  const activePlan = initialActiveId
    ? initialPlans.find((p) => p.id === initialActiveId) ?? null
    : initialPlans.length > 0
      ? initialPlans[0]
      : null;

  if (activePlan) {
    localStorage.setItem(ACTIVE_PLAN_KEY, activePlan.id);
  }

  return {
    plans: initialPlans,
    activePlanId: activePlan?.id ?? null,
    currentChapterId: initialChapter,
    codeTheme: initialTheme,
    sessionId: activePlan?.id ?? null,

    addPlan: (plan) => {
      const plans = [...get().plans, plan];
      savePlans(plans);
      localStorage.setItem(ACTIVE_PLAN_KEY, plan.id);
      localStorage.removeItem(CHAPTER_KEY);
      set({ plans, activePlanId: plan.id, sessionId: plan.id, currentChapterId: null });
    },

    removePlan: (id) => {
      const plans = get().plans.filter((p) => p.id !== id);
      savePlans(plans);
      const { activePlanId } = get();
      if (activePlanId === id) {
        const next = plans.length > 0 ? plans[0] : null;
        if (next) {
          localStorage.setItem(ACTIVE_PLAN_KEY, next.id);
        } else {
          localStorage.removeItem(ACTIVE_PLAN_KEY);
        }
        localStorage.removeItem(CHAPTER_KEY);
        set({
          plans,
          activePlanId: next?.id ?? null,
          sessionId: next?.id ?? null,
          currentChapterId: null,
        });
      } else {
        set({ plans });
      }
    },

    setActivePlan: (id) => {
      const plan = get().plans.find((p) => p.id === id);
      if (!plan) return;
      localStorage.setItem(ACTIVE_PLAN_KEY, id);
      localStorage.removeItem(CHAPTER_KEY);
      set({ activePlanId: id, sessionId: id, currentChapterId: null });
    },

    updatePlanMeta: (id, patch) => {
      const plans = get().plans.map((p) =>
        p.id === id ? { ...p, ...patch } : p,
      );
      savePlans(plans);
      set({ plans });
    },

    setChapter: (chapterId) => {
      localStorage.setItem(CHAPTER_KEY, chapterId);
      set({ currentChapterId: chapterId });
    },

    setCodeTheme: (codeTheme) => {
      localStorage.setItem(THEME_KEY, codeTheme);
      document.documentElement.setAttribute("data-theme", codeTheme);
      set({ codeTheme });
    },

    reset: () => {
      localStorage.removeItem(ACTIVE_PLAN_KEY);
      localStorage.removeItem(CHAPTER_KEY);
      localStorage.removeItem(THEME_KEY);
      set({
        activePlanId: null,
        sessionId: null,
        currentChapterId: null,
        codeTheme: "github-dark",
      });
    },
  };
});

// Apply theme on initial load
const initialTheme =
  (localStorage.getItem(THEME_KEY) as CodeTheme) || "github-dark";
document.documentElement.setAttribute("data-theme", initialTheme);
