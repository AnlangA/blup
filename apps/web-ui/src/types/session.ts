/**
 * Typed view of a session snapshot from the backend API.
 * Maps raw JSON to typed fields to avoid unsafe type assertions in components.
 */

export interface SessionView {
  sessionId: string;
  state: string;
  goal: GoalData | null;
  feasibilityResult: FeasibilityData | null;
  profile: ProfileData | null;
  profileRounds: number;
  curriculum: CurriculumData | null;
  currentChapterId: string | null;
  chapterContents: Record<string, string>;
  messages: SessionMessage[];
}

export interface GoalData {
  description: string;
  domain: string;
  context?: string;
  currentLevel?: string;
}

export interface FeasibilityData {
  feasible: boolean;
  reason: string;
  suggestions: string[];
  estimatedDuration?: string;
  prerequisites: string[];
}

export interface ProfileData {
  experienceLevel?: {
    domainKnowledge?: string;
  };
  learningStyle?: {
    preferredFormat?: string[];
  };
  availableTime?: {
    hoursPerWeek?: number;
  };
}

export interface CurriculumData {
  title: string;
  description?: string;
  chapters: ChapterData[];
  estimatedDuration: string;
}

export interface ChapterData {
  id: string;
  title: string;
  order: number;
  objectives: string[];
  estimatedMinutes?: number;
}

export interface SessionMessage {
  id: string;
  role: "user" | "assistant" | "system";
  content: string;
  timestamp: string;
  chapterId?: string;
}

import { checkShape, type FieldRule } from "./validate";
import { deepSnakeToCamel } from "./caseConversion";

/** Expected shape of a session snapshot after camelCase conversion. */
const SESSION_FIELD_RULES: Record<string, FieldRule> = {
  sessionId: { type: "string" },
  state: { type: "string" },
  goal: { type: "object", optional: true },
  feasibilityResult: { type: "object", optional: true },
  profile: { type: "object", optional: true },
  profileRounds: { type: "number" },
  curriculum: { type: "object", optional: true },
  currentChapterId: { type: "string", optional: true },
  chapterContents: { type: "object" },
  messages: { type: "array" },
};

/**
 * Parse a raw session snapshot into a typed SessionView.
 *
 * Applies deep snake_case → camelCase conversion to the raw data first,
 * so any new backend fields are automatically available in camelCase
 * without requiring manual mapping updates.
 */
export function parseSession(raw: Record<string, unknown>): SessionView {
  const cc = deepSnakeToCamel(raw) as Record<string, unknown>;

  if (import.meta.env.DEV) {
    const warnings = checkShape("session", cc, SESSION_FIELD_RULES);
    if (warnings.length > 0) {
      console.warn(
        `[Blup] Session shape mismatch (${warnings.length} issues):`,
        warnings,
      );
    }
  }

  const goal = cc.goal as Record<string, unknown> | null;
  const feasibilityResult = cc.feasibilityResult as Record<string, unknown> | null;
  const profile = cc.profile as Record<string, unknown> | null;
  const curriculum = cc.curriculum as Record<string, unknown> | null;

  return {
    sessionId: (cc.sessionId as string) ?? "",
    state: (cc.state as string) ?? "IDLE",
    goal: goal
      ? {
          description: (goal.description as string) ?? "",
          domain: (goal.domain as string) ?? "",
          context: goal.context as string | undefined,
          currentLevel: goal.currentLevel as string | undefined,
        }
      : null,
    feasibilityResult: feasibilityResult as FeasibilityData | null,
    profile: profile as ProfileData | null,
    profileRounds: (cc.profileRounds as number) ?? 0,
    curriculum: curriculum as CurriculumData | null,
    currentChapterId: (cc.currentChapterId as string) ?? null,
    chapterContents: (cc.chapterContents as Record<string, string>) ?? {},
    messages: (cc.messages as SessionMessage[]) ?? [],
  };
}
