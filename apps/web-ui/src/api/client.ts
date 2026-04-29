const BASE_URL = import.meta.env.VITE_API_URL || "";

export interface ApiError {
  code: string;
  message: string;
}

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
      const error = await res
        .json()
        .catch(() => ({
          error: { code: "UNKNOWN", message: "Request failed" },
        }));
      throw (
        error.error || { code: "HTTP_ERROR", message: `HTTP ${res.status}` }
      );
    }

    return res.json();
  }

  async createSession(): Promise<{ session_id: string; state: string }> {
    return this.request("POST", "/api/session");
  }

  async submitGoal(
    sessionId: string,
    goal: { description: string; domain: string; context?: string },
  ): Promise<Record<string, unknown>> {
    return this.request("POST", `/api/session/${sessionId}/goal`, goal);
  }

  async submitProfileAnswer(
    sessionId: string,
    answer: { question_id: string; answer: string },
  ): Promise<Record<string, unknown>> {
    return this.request(
      "POST",
      `/api/session/${sessionId}/profile/answer`,
      answer,
    );
  }

  async getCurriculum(sessionId: string): Promise<Record<string, unknown>> {
    return this.request("GET", `/api/session/${sessionId}/curriculum`);
  }

  async startChapter(
    sessionId: string,
    chapterId: string,
  ): Promise<Record<string, unknown>> {
    return this.request(
      "GET",
      `/api/session/${sessionId}/chapter/${chapterId}`,
    );
  }

  async askQuestion(
    sessionId: string,
    chapterId: string,
    question: string,
  ): Promise<Record<string, unknown>> {
    return this.request(
      "POST",
      `/api/session/${sessionId}/chapter/${chapterId}/ask`,
      { question },
    );
  }

  async completeChapter(
    sessionId: string,
    chapterId: string,
  ): Promise<Record<string, unknown>> {
    return this.request(
      "POST",
      `/api/session/${sessionId}/chapter/${chapterId}/complete`,
    );
  }

  async getSession(sessionId: string): Promise<{
    session_id: string;
    state: string;
    goal: Record<string, unknown> | null;
    feasibility_result: Record<string, unknown> | null;
    profile: Record<string, unknown> | null;
    curriculum: Record<string, unknown> | null;
    current_chapter_id: string | null;
    chapter_contents: Record<string, string>;
    messages: Array<{
      id: string;
      role: string;
      content: string;
      timestamp: string;
    }>;
  }> {
    return this.request("GET", `/api/session/${sessionId}`);
  }
}

export const api = new ApiClient();
