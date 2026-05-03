import { useState, useCallback } from "react";
import { useSandboxExecute } from "../../hooks/query";
import { useSessionStore } from "../../state/sessionStore";
import { SUPPORTED_LANGUAGES, LANGUAGE_DISPLAY } from "../../api/generated-sandbox";
import type { SandboxLanguage } from "../../api/generated-sandbox";
import { CodeEditor } from "./CodeEditor";

interface SandboxRunnerProps {
  language: string;
  code: string;
}

export function SandboxRunner({ language, code: initialCode }: SandboxRunnerProps) {
  const sessionId = useSessionStore((s) => s.sessionId);
  const sandbox = useSandboxExecute();
  const [editableCode, setEditableCode] = useState(initialCode);

  const normalizedLanguage = SUPPORTED_LANGUAGES[language.toLowerCase()] as SandboxLanguage | undefined;

  const handleRun = useCallback(() => {
    if (!sessionId || sandbox.isRunning || !normalizedLanguage) return;
    sandbox.execute({
      session_id: sessionId,
      language: normalizedLanguage,
      code: editableCode,
      timeout_secs: 30,
    });
  }, [sessionId, sandbox, normalizedLanguage, editableCode]);

  const handleReset = useCallback(() => {
    setEditableCode(initialCode);
    sandbox.reset();
  }, [initialCode, sandbox]);

  if (!normalizedLanguage) return null;

  const displayLang = LANGUAGE_DISPLAY[normalizedLanguage] || normalizedLanguage;

  return (
    <div className="sandbox-runner" data-testid="sandbox-runner">
      <div className="sandbox-toolbar">
        <span className="sandbox-lang-badge">{displayLang}</span>
        <div className="sandbox-toolbar-actions">
          <button className="sandbox-edit-btn" onClick={handleReset} aria-label="Reset code">
            Reset
          </button>
          <button
            className="sandbox-run-btn"
            onClick={handleRun}
            disabled={sandbox.isRunning}
            aria-label={`Run ${normalizedLanguage} code`}
          >
            {sandbox.isRunning ? (
              <>
                <span className="sandbox-spinner" /> Running...
              </>
            ) : (
              <>
                <svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor">
                  <path d="M4 2l10 6-10 6V2z" />
                </svg>
                Run
              </>
            )}
          </button>
        </div>
      </div>

      <CodeEditor code={editableCode} language={normalizedLanguage} onChange={setEditableCode} />

      {(sandbox.stdout || sandbox.stderr || sandbox.error || sandbox.exitCode !== null) && (
        <div className="sandbox-output" data-testid="sandbox-output">
          {sandbox.stdout && (
            <pre className="sandbox-stdout" data-testid="sandbox-stdout">
              {sandbox.stdout}
            </pre>
          )}
          {sandbox.stderr && (
            <pre className="sandbox-stderr" data-testid="sandbox-stderr">
              {sandbox.stderr}
            </pre>
          )}
          {sandbox.error && (
            <pre className="sandbox-error" role="alert">
              {sandbox.error}
            </pre>
          )}
          {sandbox.exitCode !== null && !sandbox.isRunning && (
            <div className="sandbox-meta">
              <span>Exit code: {sandbox.exitCode}</span>
              {sandbox.durationMs !== null && <span>Duration: {sandbox.durationMs}ms</span>}
            </div>
          )}
        </div>
      )}
    </div>
  );
}
