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
  prerequisites?: string[];
  key_concepts?: string[];
  exercises?: Array<{
    question?: string;
    options?: string[];
    type?: string;
    [key: string]: unknown;
  }>;
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

export interface SessionListEntry {
  id: string;
  state: string;
  goal_description: string;
  domain: string;
  updated_at: string;
}

export interface SessionSnapshot {
  session_id: string;
  state: string;
  goal: Record<string, unknown> | null;
  feasibility_result: Record<string, unknown> | null;
  profile: Record<string, unknown> | null;
  profile_rounds: number;
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

export interface ExportResult {
  filename: string;
  checksum: string;
  size_bytes?: number;
  pdf_base64?: string;
  typst_source?: string;
  page_count?: number;
}

export interface SandboxExecuteRequest {
  session_id: string;
  language: "python" | "javascript" | "rust" | "typst";
  code: string;
  stdin?: string;
  timeout_secs?: number;
}

export interface SandboxHealth {
  healthy: boolean;
  images: Array<{ name: string; version: string }>;
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

  async listSessions(): Promise<SessionListEntry[]> {
    return this.request("GET", "/api/sessions");
  }

  async deleteSession(sessionId: string): Promise<{ deleted: boolean }> {
    return this.request("DELETE", `/api/session/${sessionId}`);
  }

  async exportChapterTypst(
    sessionId: string,
    chapterId: string,
  ): Promise<ExportResult> {
    return this.request(
      "POST",
      `/api/session/${sessionId}/export/chapter/${chapterId}/typst`,
    );
  }

  async exportCurriculumTypst(sessionId: string): Promise<ExportResult> {
    return this.request(
      "POST",
      `/api/session/${sessionId}/export/curriculum/typst`,
    );
  }

  async getSandboxHealth(): Promise<SandboxHealth> {
    return this.request("GET", "/api/sandbox/health");
  }
}

export const api = new ApiClient();

export async function downloadBlob(
  data: Blob | string,
  filename: string,
  mimeType: string,
) {
  const blob =
    data instanceof Blob ? data : new Blob([data], { type: mimeType });

  // Use native File System Access API save dialog when available
  if ("showSaveFilePicker" in window) {
    try {
      const ext = filename.split(".").pop() || "";
      const types: FilePickerAcceptType[] = [
        {
          description:
            ext === "pdf"
              ? "PDF Document"
              : ext === "typ"
                ? "Typst Source"
                : "Document",
          accept: { [mimeType]: [`.${ext}`] },
        },
      ];

      const handle = await (
        window as Window & {
          showSaveFilePicker: (opts: {
            suggestedName: string;
            types: FilePickerAcceptType[];
          }) => Promise<{
            createWritable: () => Promise<{
              write: (data: Blob) => Promise<void>;
              close: () => Promise<void>;
            }>;
          }>;
        }
      ).showSaveFilePicker({
        suggestedName: filename,
        types,
      });

      const writable = await handle.createWritable();
      await writable.write(blob);
      await writable.close();
      return;
    } catch (err) {
      if ((err as Error).name === "AbortError") {
        // User cancelled the save dialog — silently return
        return;
      }
      // Fall through to legacy download on error
    }
  }

  // Legacy fallback: trigger browser download
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = filename;
  a.click();
  URL.revokeObjectURL(url);
}

interface FilePickerAcceptType {
  description: string;
  accept: Record<string, string[]>;
}
