// AUTO-GENERATED from schemas/*.v1.schema.json - do not edit by hand.

export interface AssessmentResult {
  exercise_id: string;
  learner_answer: Record<string, unknown>;
  score: number;
  max_score: number;
  feedback: string;
  rubric_results?: Array<{
  dimension: string;
  score: number;
  max_score: number;
  comment: string;
  [key: string]: unknown;
}>;
  is_correct: boolean;
  [key: string]: unknown;
}

export interface Chapter {
  id: string;
  title: string;
  order: number;
  objectives: Array<string>;
  prerequisites?: Array<string>;
  content?: string;
  estimated_minutes?: number;
  key_concepts?: Array<string>;
  exercises?: Array<{
  question: string;
  type: "multiple_choice" | "short_answer" | "coding" | "reflection";
  difficulty?: "easy" | "medium" | "hard";
}>;
}

export interface ChapterProgress {
  chapter_id: string;
  status: "not_started" | "in_progress" | "completed" | "skipped";
  completion: number;
  time_spent_minutes?: number;
  exercises_completed?: number;
  exercises_total?: number;
  last_accessed?: string;
  notes?: Array<string>;
  difficulty_rating?: number;
}

export interface CurriculumPlan {
  title: string;
  description?: string;
  chapters: Array<{
  id: string;
  title: string;
  order: number;
  objectives: Array<string>;
  prerequisites?: Array<string>;
  estimated_minutes?: number;
  key_concepts?: Array<string>;
  exercises?: Array<{
  question: string;
  type: string;
  difficulty?: "beginner" | "intermediate" | "advanced";
}>;
}>;
  estimated_duration: string;
  prerequisites_summary?: Array<string>;
  learning_objectives?: Array<string>;
}

export interface DocumentArtifact {
  id: string;
  session_id?: string;
  format: "pdf" | "typst";
  checksum: string;
  size_bytes: number;
  page_count?: number;
  generated_at: string;
  source_content_ids: Array<string>;
  source_typst?: string;
}

export interface Exercise {
  id: string;
  chapter_id: string;
  question: string;
  exercise_type: Record<string, unknown>;
  difficulty: "easy" | "medium" | "hard";
  rubric?: Record<string, unknown>;
  max_score: number;
  hints?: Array<string>;
  explanation?: string;
  [key: string]: unknown;
}

export interface ExportConfig {
  template?: "chapter" | "curriculum";
  include_toc?: boolean;
  include_title_page?: boolean;
  compile_timeout_secs?: number;
}

export interface TypstDiagnostic {
  severity: "error" | "warning";
  message: string;
  line?: number;
  column?: number;
  source_line?: string;
  hint?: string;
}

export interface ExportJob {
  id: string;
  session_id?: string;
  export_type: "chapter" | "curriculum";
  source_id: string;
  config?: ExportConfig;
  status: "pending" | "rendering" | "compiling" | "completed" | "failed";
  error?: {
  code: string;
  message: string;
  diagnostics?: Array<TypstDiagnostic>;
};
  result_artifact_id?: string;
  created_at?: string;
  completed_at?: string;
}

export interface FeasibilityResult {
  feasible: boolean;
  reason: string;
  suggestions?: Array<string>;
  estimated_duration?: string;
  prerequisites?: Array<string>;
}

export interface ImportConfig {
  ocr_enabled?: boolean;
  max_chunk_size_chars?: number;
  chunk_overlap_chars?: number;
  timeout_secs?: number;
}

export interface ImportJob {
  id: string;
  session_id?: string;
  source_type: "pdf" | "markdown" | "plain_text" | "website";
  source_path?: string;
  source_url?: string;
  config?: ImportConfig;
  status: "pending" | "extracting" | "chunking" | "completed" | "failed";
  error?: {
  code: string;
  message: string;
};
  result_document_id?: string;
  created_at?: string;
  completed_at?: string;
}

export interface LearningGoal {
  description: string;
  domain: string;
  context?: string;
  current_level?: "beginner" | "intermediate" | "advanced" | "unknown";
}

export interface Message {
  id: string;
  role: "user" | "assistant" | "system";
  content: string;
  timestamp: string;
  chapter_id: string;
  content_type?: "text" | "question" | "exercise" | "feedback" | "explanation" | "example" | "summary";
  references?: Array<{
  chapter_id?: string;
  concept?: string;
  source_chunk_id?: string;
}>;
  metadata?: {
  tokens_used?: number;
  model?: string;
  generation_duration_ms?: number;
};
}

export interface SandboxRequest {
  request_id: string;
  session_id: string;
  tool_kind: "python_exec" | "node_exec" | "typescript_compile_run" | "rust_compile_run" | "go_compile_run" | "c_compile_run" | "cpp_compile_run" | "java_compile_run" | "ruby_exec" | "bash_exec" | "typst_compile";
  code: string;
  language?: "python" | "javascript" | "typescript" | "rust" | "go" | "c" | "cpp" | "java" | "ruby" | "bash" | "typst";
  limits?: {
  compile_timeout_secs?: number;
  run_timeout_secs?: number;
  memory_mb?: number;
  cpu_count?: number;
  disk_mb?: number;
  network_enabled?: boolean;
  max_processes?: number;
  [key: string]: unknown;
};
  stdin?: string;
  environment?: Record<string, string>;
  [key: string]: unknown;
}

export interface SandboxResult {
  request_id: string;
  session_id?: string;
  status: "success" | "timeout_compile" | "timeout_run" | "memory_exceeded" | "cpu_exceeded" | "disk_exceeded" | "non_zero_exit" | "network_blocked" | "internal_error";
  exit_code: number | null;
  stdout: string;
  stderr: string;
  stdout_truncated: boolean;
  stderr_truncated: boolean;
  duration_ms: number;
  resource_usage?: {
  peak_memory_mb?: number;
  cpu_time_ms?: number;
  disk_used_kb?: number;
  oom_killed?: boolean;
  [key: string]: unknown;
};
  error?: {
  code?: string;
  message?: string;
  [key: string]: unknown;
};
  [key: string]: unknown;
}

export interface SourceChunk {
  id: string;
  document_id: string;
  index: number;
  content: string;
  heading_path?: Array<string>;
  token_count?: number;
  overlap_with_previous?: boolean;
}

export interface SourceMetadata {
  page_count?: number;
  word_count: number;
  character_count?: number;
  extraction_method: "direct_text" | "pdftotext" | "ocr" | "fetch_readability" | "markdown_parse" | "text_read";
  extraction_confidence?: number;
  ocr_applied?: boolean;
  warnings?: Array<string>;
}

export interface SourceDocument {
  id: string;
  source_type: "pdf" | "markdown" | "plain_text" | "website";
  title: string;
  origin: string;
  checksum: string;
  language?: string;
  license_or_usage_note?: string;
  extracted_at: string;
  metadata: SourceMetadata;
  chunks: Array<SourceChunk>;
}

export interface UserProfile {
  experience_level: {
  domain_knowledge: "none" | "beginner" | "intermediate" | "advanced";
  related_domains?: Array<string>;
  years_of_experience?: number;
};
  learning_style: {
  preferred_format: Array<"text" | "visual" | "interactive" | "audio" | "exercise-based" | "project-based">;
  pace_preference?: "slow_thorough" | "moderate" | "fast_paced" | "self_directed";
  notes?: string;
};
  available_time: {
  hours_per_week: number;
  preferred_session_length_minutes?: number;
  timezone?: string;
};
  goals?: {
  primary_goal?: string;
  secondary_goals?: Array<string>;
  success_criteria?: string;
};
  preferences?: {
  language?: string;
  difficulty_bias?: "easier" | "standard" | "challenging";
  feedback_frequency?: "immediate" | "end_of_section" | "end_of_chapter";
};
}
