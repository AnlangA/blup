import { useState, useRef, useEffect, useCallback } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { useSessionStore } from "../../state/sessionStore";
import {
  useExportChapterTypst,
  useExportCurriculumTypst,
  useExportChapterPdf,
  useExportCurriculumPdf,
} from "../../hooks/query";
import {
  isTauri,
  tauriExportChapterPdf,
  tauriExportCurriculumPdf,
  tauriExportChapterTypst,
  tauriExportCurriculumTypst,
  type TauriExportResult,
} from "../../api/tauri-bridge";
import type { CurriculumPlan, ChapterContent } from "../../api/client";

/** Extract just the filename from a full path for display. */
function fileName(path: string): string {
  const i = Math.max(path.lastIndexOf("/"), path.lastIndexOf("\\"));
  return i >= 0 ? path.slice(i + 1) : path;
}

function formatTauriError(err: unknown): string {
  if (err instanceof Error) return err.message;
  if (typeof err === "string") return err;
  if (err && typeof err === "object") {
    const obj = err as Record<string, unknown>;
    if (typeof obj.message === "string") return obj.message;
    if (typeof obj.code === "string" && typeof obj.message === "string") {
      return `${obj.code}: ${obj.message}`;
    }
    try {
      return JSON.stringify(err);
    } catch {
      return String(err);
    }
  }
  return String(err);
}

interface ExportButtonProps {
  chapterId?: string;
  variant?: "icon" | "text";
}

export function ExportButton({
  chapterId,
  variant = "icon",
}: ExportButtonProps) {
  const [open, setOpen] = useState(false);
  const [tauriStatus, setTauriStatus] = useState("");
  const [tauriError, setTauriError] = useState<string | null>(null);
  const [tauriExporting, setTauriExporting] = useState(false);
  const sessionId = useSessionStore((s) => s.sessionId);
  const ref = useRef<HTMLDivElement>(null);
  const dismissTimer = useRef<ReturnType<typeof setTimeout> | null>(null);

  const isChapter = !!chapterId;
  const useTauri = isTauri();

  const clearDismissTimer = useCallback(() => {
    if (dismissTimer.current) {
      clearTimeout(dismissTimer.current);
      dismissTimer.current = null;
    }
  }, []);

  // Show a status message that auto-dismisses after 5 s.
  const showStatus = useCallback((msg: string) => {
    clearDismissTimer();
    setTauriStatus(msg);
    setTauriError(null);
    dismissTimer.current = setTimeout(() => setTauriStatus(""), 5000);
  }, [clearDismissTimer]);

  const queryClient = useQueryClient();

  // Resolve chapter data from the react-query cache for Tauri export.
  // Includes chapter metadata (objectives, prerequisites, etc.) so the
  // Typst renderer produces a complete chapter document.
  const resolveChapterData = (): Record<string, unknown> | null => {
    if (!sessionId || !chapterId) return null;
    const curriculum = queryClient.getQueryData<CurriculumPlan>(["curriculum", sessionId]);
    const content = queryClient.getQueryData<ChapterContent>(["chapter", sessionId, chapterId]);
    if (!curriculum || !content) return null;
    const ch = curriculum.chapters.find((c) => c.id === chapterId);
    if (!ch) return null;
    const data: Record<string, unknown> = {
      title: ch.title,
      content: content.content,
    };
    if (ch.estimated_minutes) data.estimated_minutes = ch.estimated_minutes;
    if (ch.objectives?.length) data.objectives = ch.objectives;
    if (ch.prerequisites?.length) data.prerequisites = ch.prerequisites;
    if (ch.key_concepts?.length) data.key_concepts = ch.key_concepts;
    if (ch.exercises?.length) data.exercises = ch.exercises;
    return data;
  };

  const resolveCurriculumData = (): Record<string, unknown> | null => {
    if (!sessionId) return null;
    const data = queryClient.getQueryData<CurriculumPlan>(["curriculum", sessionId]);
    if (!data) return null;
    return data as unknown as Record<string, unknown>;
  };

  // Browser-mode hooks (SSE-based)
  const chapterTypst = useExportChapterTypst(sessionId, chapterId ?? null);
  const curriculumTypst = useExportCurriculumTypst(sessionId);
  const chapterPdf = useExportChapterPdf(sessionId, chapterId ?? null);
  const curriculumPdf = useExportCurriculumPdf(sessionId);

  const browserTypstMutation = isChapter ? chapterTypst : curriculumTypst;
  const browserPdfHook = isChapter ? chapterPdf : curriculumPdf;

  const isExporting = useTauri
    ? tauriExporting
    : browserTypstMutation.isPending || browserPdfHook.isExporting;

  // Close dropdown on outside click
  useEffect(() => {
    if (!open) return;
    const handler = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) {
        setOpen(false);
      }
    };
    document.addEventListener("mousedown", handler);
    return () => document.removeEventListener("mousedown", handler);
  }, [open]);

  const doTauriChapterExport = async (
    mode: "pdf" | "typst",
  ): Promise<TauriExportResult | null> => {
    const chapterData = resolveChapterData();
    if (!chapterData) throw new Error("Chapter data not available. Please reload the page.");
    return mode === "pdf"
      ? tauriExportChapterPdf(chapterData)
      : tauriExportChapterTypst(chapterData);
  };

  const doTauriCurriculumExport = async (
    mode: "pdf" | "typst",
  ): Promise<TauriExportResult | null> => {
    const curriculumData = resolveCurriculumData();
    if (!curriculumData) throw new Error("Curriculum data not available. Please reload the page.");
    return mode === "pdf"
      ? tauriExportCurriculumPdf(curriculumData)
      : tauriExportCurriculumTypst(curriculumData);
  };

  const handleExportTypst = async () => {
    setOpen(false);
    if (useTauri && sessionId) {
      setTauriExporting(true);
      try {
        const result = await (isChapter ? doTauriChapterExport("typst") : doTauriCurriculumExport("typst"));
        if (result) showStatus(`Saved — ${fileName(result.path)}`);
      } catch (err) {
        const msg = formatTauriError(err);
        if (!msg.includes("USER_CANCELLED") && !msg.includes("cancelled")) {
          setTauriError(msg);
        }
      } finally {
        setTauriExporting(false);
      }
    } else {
      browserTypstMutation.mutate();
    }
  };

  const handleExportPdf = async () => {
    setOpen(false);
    if (useTauri && sessionId) {
      setTauriExporting(true);
      try {
        const result = await (isChapter ? doTauriChapterExport("pdf") : doTauriCurriculumExport("pdf"));
        if (result) {
          const label = result.compiled ? "PDF exported" : "Typst saved (PDF unavailable)";
          showStatus(`${label} — ${fileName(result.path)}`);
        }
      } catch (err) {
        const msg = formatTauriError(err);
        if (!msg.includes("USER_CANCELLED") && !msg.includes("cancelled")) {
          setTauriError(msg);
        }
      } finally {
        setTauriExporting(false);
      }
    } else {
      browserPdfHook.exportPdf();
    }
  };

  const browserError =
    browserTypstMutation.error?.message ||
    (typeof browserPdfHook.error === "string" ? browserPdfHook.error : null);

  const browserStatus =
    browserPdfHook.status ? browserPdfHook.message : "";

  const toastMessage = useTauri
    ? tauriError || tauriStatus
    : browserError || browserStatus;

  const toastType = (useTauri && tauriError) || (!useTauri && browserError)
    ? "error"
    : toastMessage
      ? "status"
      : null;

  return (
    <div className="export-button" ref={ref}>
      <button
        className={`export-trigger export-trigger--${variant}`}
        onClick={() => setOpen(!open)}
        disabled={isExporting}
        title={`Export ${isChapter ? "chapter" : "curriculum"}`}
        aria-haspopup="true"
        aria-expanded={open}
      >
        {isExporting ? (
          <span className="export-spinner" aria-label="Exporting..." />
        ) : (
          <svg
            width="16"
            height="16"
            viewBox="0 0 16 16"
            fill="none"
            stroke="currentColor"
            strokeWidth="1.5"
          >
            <path d="M8 2v8M4 6l4 4 4-4M2 12v1a1 1 0 001 1h10a1 1 0 001-1v-1" />
          </svg>
        )}
        {variant === "text" && (
          <span className="export-label">
            {isExporting
              ? "Exporting..."
              : `Export ${isChapter ? "Chapter" : "Curriculum"}`}
          </span>
        )}
      </button>

      {open && (
        <div className="export-dropdown" role="menu">
          <button
            className="export-option"
            role="menuitem"
            onClick={handleExportPdf}
          >
            <svg
              width="14"
              height="14"
              viewBox="0 0 16 16"
              fill="none"
              stroke="currentColor"
              strokeWidth="1.5"
            >
              <path d="M4 10V2h8v8M2 13h12v1H2z" />
            </svg>
            Export as PDF
          </button>
          <button
            className="export-option"
            role="menuitem"
            onClick={handleExportTypst}
          >
            <svg
              width="14"
              height="14"
              viewBox="0 0 16 16"
              fill="none"
              stroke="currentColor"
              strokeWidth="1.5"
            >
              <path d="M2 2h12v12H2zM5 2v12M2 6h12M2 10h12" />
            </svg>
            Export as Typst
          </button>
        </div>
      )}

      {toastMessage && (
        <div
          className={`export-toast export-toast--${toastType}`}
          role={toastType === "error" ? "alert" : "status"}
        >
          <span className="export-toast-text">{toastMessage}</span>
          <button
            className="export-toast-close"
            onClick={() => {
              if (useTauri) { setTauriStatus(""); setTauriError(null); }
              else { browserPdfHook.reset?.(); }
            }}
            aria-label="Dismiss"
          >
            ×
          </button>
        </div>
      )}
    </div>
  );
}
