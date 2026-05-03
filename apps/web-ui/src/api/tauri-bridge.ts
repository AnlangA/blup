/**
 * Bridge to Tauri's native APIs when running in the desktop app.
 * Falls back gracefully when running in a regular browser.
 *
 * Export commands accept chapter/curriculum data directly from the frontend
 * cache — no need for the desktop backend to re-fetch from agent-core.
 */

export interface TauriExportResult {
  path: string;
  checksum: string;
  size_bytes: number;
  page_count: number | null;
  compiled: boolean;
  format: string;
}

interface TauriWindow {
  __TAURI__?: {
    invoke: (cmd: string, args?: Record<string, unknown>) => Promise<unknown>;
    event?: {
      listen: (event: string, handler: (event: { payload: unknown }) => void) => Promise<() => void>;
    };
    notification?: {
      requestPermission: () => Promise<string>;
      sendNotification: (opts: Record<string, unknown>) => void;
    };
  };
}

function getTauri(): TauriWindow["__TAURI__"] | undefined {
  if (typeof window === "undefined") return undefined;
  const win = window as unknown as TauriWindow;
  return win.__TAURI__ ?? (win as unknown as Record<string, unknown>).__TAURI_INTERNALS__ as TauriWindow["__TAURI__"];
}

export function isTauri(): boolean {
  try {
    return getTauri() !== undefined || !!(window as unknown as Record<string, unknown>).__TAURI_INTERNALS__;
  } catch {
    return false;
  }
}

export async function tauriExportChapterPdf(
  chapter: Record<string, unknown>,
): Promise<TauriExportResult> {
  const tauri = getTauri();
  if (!tauri) throw new Error("Tauri not available");
  return tauri.invoke("export_chapter_pdf", { chapter }) as Promise<TauriExportResult>;
}

export async function tauriExportCurriculumPdf(
  curriculum: Record<string, unknown>,
): Promise<TauriExportResult> {
  const tauri = getTauri();
  if (!tauri) throw new Error("Tauri not available");
  return tauri.invoke("export_curriculum_pdf", { curriculum }) as Promise<TauriExportResult>;
}

export async function tauriExportChapterTypst(
  chapter: Record<string, unknown>,
): Promise<TauriExportResult> {
  const tauri = getTauri();
  if (!tauri) throw new Error("Tauri not available");
  return tauri.invoke("export_typst", { chapter }) as Promise<TauriExportResult>;
}

export async function tauriExportCurriculumTypst(
  curriculum: Record<string, unknown>,
): Promise<TauriExportResult> {
  const tauri = getTauri();
  if (!tauri) throw new Error("Tauri not available");
  return tauri.invoke("export_curriculum_typst", { curriculum }) as Promise<TauriExportResult>;
}

/**
 * Listen for Tauri export progress events.
 */
export function onTauriExportProgress(
  callback: (data: { stage: string; chapter?: string }) => void,
): () => void {
  if (typeof window === "undefined") return () => {};

  const tauri = getTauri();
  if (tauri?.event) {
    let unlisten: (() => void) | null = null;
    tauri.event.listen("export:progress", (event) => {
      callback(event.payload as { stage: string; chapter?: string });
    }).then((fn) => { unlisten = fn; });
    return () => { unlisten?.(); };
  }

  // Fallback for non-Tauri environments
  const handler = (event: Event) => {
    const detail = (event as CustomEvent).detail;
    if (detail) {
      callback(detail);
    }
  };

  window.addEventListener("export:progress", handler);
  return () => window.removeEventListener("export:progress", handler);
}

export function onTauriExportComplete(
  callback: (data: {
    chapter?: string;
    session?: string;
    path: string;
    compiled?: boolean;
    format?: string;
  }) => void,
): () => void {
  if (typeof window === "undefined") return () => {};

  const tauri = getTauri();
  if (tauri?.event) {
    let unlisten: (() => void) | null = null;
    tauri.event.listen("export:complete", (event) => {
      callback(event.payload as {
        chapter?: string;
        session?: string;
        path: string;
        compiled?: boolean;
        format?: string;
      });
    }).then((fn) => { unlisten = fn; });
    return () => { unlisten?.(); };
  }

  // Fallback for non-Tauri environments
  const handler = (event: Event) => {
    const detail = (event as CustomEvent).detail;
    if (detail) {
      callback(detail);
    }
  };

  window.addEventListener("export:complete", handler);
  return () => window.removeEventListener("export:complete", handler);
}
