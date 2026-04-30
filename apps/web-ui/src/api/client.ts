const BASE_URL = import.meta.env.VITE_API_URL || "";

export interface ApiError {
  code: string;
  message: string;
}

// ── Request / Response types ──

export interface CreateSessionResponse {
  session_id: string;
  state: string;
}

export interface LearningGoal {
  description: string;
  domain: string;
  context?: string;
  current_level?: string;
}

export interface GoalSubmitResult {
  feasibility: FeasibilityData;
  state: string;
}

export interface FeasibilityData {
  feasible: boolean;
  reason: string;
  suggestions: string[];
  estimated_duration?: string;
  prerequisites: string[];
}

export interface ProfileAnswer {
  question_id: string;
  answer: string;
}

export interface ProfileAnswerResult {
  is_complete: boolean;
  profile: Record<string, unknown>;
  state: string;
}

export interface Chapter {
  id: string;
  title: string;
  order: number;
  objectives: string[];
  estimated_minutes?: number;
}

export interface CurriculumPlan {
  title: string;
  description: string;
  chapters: Chapter[];
  estimated_duration: string;
}

export interface ChapterContent {
  id: string;
  role: string;
  content: string;
  timestamp: string;
}

export interface QuestionRequest {
  question: string;
}

export interface ChapterProgress {
  chapter_id: string;
  status: string;
  completion: number;
  last_accessed: string;
}

export interface SessionSnapshot {
  session_id: string;
  state: string;
  goal: Record<string, unknown> | null;
  feasibility_result: Record<string, unknown> | null;
  profile: Record<string, unknown> | null;
  profile_rounds?: number;
  curriculum: CurriculumPlan | null;
  current_chapter_id: string | null;
  chapter_contents: Record<string, string>;
  messages: Array<{
    id: string;
    role: string;
    content: string;
    timestamp: string;
    chapter_id?: string;
  }>;
}

// ── Client ──

export class ApiClient {
  private async request<T>(
    method: string,
    path: string,
    body?: unknown,
  ): Promise<T> {
    const url = `${BASE_URL}${path}`;
    const res = await fetch(url, {
      method,
      headers: body
        ? { "Content-Type": "application/json", Accept: "application/json" }
        : { Accept: "application/json" },
      body: body ? JSON.stringify(body) : undefined,
    });

    const contentType = res.headers.get("content-type") || "";
    if (!contentType.includes("application/json")) {
      const text = await res.text().catch(() => "");
      console.error(
        `API returned non-JSON response: ${res.status} ${res.statusText}`,
        text.substring(0, 200),
      );
      throw {
        code: "NON_JSON_RESPONSE",
        message: `Server returned ${res.status} ${res.statusText}`,
      } as ApiError;
    }

    if (!res.ok) {
      const error = await res.json().catch(() => ({
        error: { code: "UNKNOWN", message: "Request failed" },
      }));
      throw (
        error.error || { code: "HTTP_ERROR", message: `HTTP ${res.status}` }
      );
    }

    return res.json();
  }

  async createSession(): Promise<CreateSessionResponse> {
    return this.request("POST", "/api/session");
  }

  async submitGoal(
    sessionId: string,
    goal: LearningGoal,
  ): Promise<GoalSubmitResult> {
    return this.request("POST", `/api/session/${sessionId}/goal`, goal);
  }

  async submitProfileAnswer(
    sessionId: string,
    answer: ProfileAnswer,
  ): Promise<ProfileAnswerResult> {
    return this.request(
      "POST",
      `/api/session/${sessionId}/profile/answer`,
      answer,
    );
  }

  async getCurriculum(sessionId: string): Promise<CurriculumPlan> {
    return this.request("GET", `/api/session/${sessionId}/curriculum`);
  }

  async startChapter(
    sessionId: string,
    chapterId: string,
  ): Promise<ChapterContent> {
    return this.request(
      "GET",
      `/api/session/${sessionId}/chapter/${chapterId}`,
    );
  }

  async askQuestion(
    sessionId: string,
    chapterId: string,
    question: string,
  ): Promise<ChapterContent> {
    return this.request(
      "POST",
      `/api/session/${sessionId}/chapter/${chapterId}/ask`,
      { question },
    );
  }

  async completeChapter(
    sessionId: string,
    chapterId: string,
  ): Promise<ChapterProgress> {
    return this.request(
      "POST",
      `/api/session/${sessionId}/chapter/${chapterId}/complete`,
    );
  }

  async getSession(sessionId: string): Promise<SessionSnapshot> {
    return this.request("GET", `/api/session/${sessionId}`);
  }
}

export const api = new ApiClient();
