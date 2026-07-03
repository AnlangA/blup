import { FormEvent, useState } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { useImportWebsite, useSources } from "../../hooks/query";
import { useSessionStore } from "../../state/sessionStore";
import {
  isTauri,
  tauriImportFile,
  type TauriImportResult,
} from "../../api/tauri-bridge";

function shortChecksum(checksum: string): string {
  return checksum.replace(/^sha256:/, "").slice(0, 10);
}

export function MaterialsPanel() {
  const sessionId = useSessionStore((s) => s.sessionId);
  const [url, setUrl] = useState("");
  const [localImport, setLocalImport] = useState<TauriImportResult | null>(null);
  const [localError, setLocalError] = useState<string | null>(null);
  const sources = useSources(sessionId);
  const importWebsite = useImportWebsite(sessionId);
  const queryClient = useQueryClient();
  const desktop = isTauri();

  const handleWebsiteImport = (event: FormEvent) => {
    event.preventDefault();
    if (!sessionId || !url.trim()) return;
    importWebsite.mutate(url.trim(), {
      onSuccess: () => setUrl(""),
    });
  };

  const handleFileImport = async () => {
    if (!sessionId) return;
    setLocalError(null);
    try {
      const result = await tauriImportFile(sessionId);
      setLocalImport(result);
      queryClient.invalidateQueries({ queryKey: ["sources", sessionId] });
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      if (!message.includes("USER_CANCELLED") && !message.includes("cancelled")) {
        setLocalError(message);
      }
    }
  };

  const sourceItems = sources.data ?? [];

  return (
    <section className="materials-panel" aria-label="Materials">
      <div className="materials-header">
        <h3>Materials</h3>
        {desktop && (
          <button
            type="button"
            className="materials-icon-button"
            onClick={handleFileImport}
            title="Import file"
          >
            +
          </button>
        )}
      </div>

      <form className="materials-import" onSubmit={handleWebsiteImport}>
        <input
          type="url"
          value={url}
          onChange={(event) => setUrl(event.target.value)}
          placeholder="https://..."
          aria-label="Website URL"
        />
        <button type="submit" disabled={!url.trim() || importWebsite.isPending}>
          Import
        </button>
      </form>

      {(importWebsite.error || localError) && (
        <p className="materials-error">
          {importWebsite.error?.message || localError}
        </p>
      )}

      {localImport && (
        <div className="materials-local-result">
          <span>{localImport.title}</span>
          <small>{localImport.chunks} chunks</small>
        </div>
      )}

      <ul className="materials-list">
        {sourceItems.map((source) => (
          <li key={source.id}>
            <span className="materials-title">{source.title}</span>
            <span className="materials-meta">
              {source.source_type} | {source.chunk_count} chunks |{" "}
              {shortChecksum(source.checksum)}
            </span>
          </li>
        ))}
      </ul>
    </section>
  );
}
