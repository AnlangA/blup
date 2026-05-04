import { useState, useCallback } from "react";
import { useInteractiveSandbox, useSandboxExecute } from "../../hooks/query";
import { useSessionStore } from "../../state/sessionStore";
import { SUPPORTED_LANGUAGES, LANGUAGE_DISPLAY } from "../../api/generated-sandbox";
import type { SandboxLanguage } from "../../api/generated-sandbox";
import { CodeEditor } from "./CodeEditor";
import { InteractiveTerminal } from "./InteractiveTerminal";

interface SandboxRunnerProps {
  language: string;
  code: string;
}

export function SandboxRunner({ language, code: initialCode }: SandboxRunnerProps) {
  const sessionId = useSessionStore((s) => s.sessionId);
  const sandbox = useSandboxExecute();
  const interactive = useInteractiveSandbox();
  const [editableCode, setEditableCode] = useState(initialCode);
  const [mode, setMode] = useState<"batch" | "interactive">(() =>
    needsInteractive(initialCode) ? "interactive" : "batch",
  );

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
    interactive.reset();
  }, [initialCode, sandbox, interactive]);

  const handleRunInteractive = useCallback(() => {
    if (!sessionId || interactive.isStarting || !normalizedLanguage) return;
    void interactive.start({
      session_id: sessionId,
      language: normalizedLanguage,
      code: editableCode,
      timeout_secs: 180,
    });
  }, [sessionId, interactive, normalizedLanguage, editableCode]);

  if (!normalizedLanguage) return null;

  const displayLang = LANGUAGE_DISPLAY[normalizedLanguage] || normalizedLanguage;

  return (
    <div className="sandbox-runner" data-testid="sandbox-runner">
      <div className="sandbox-toolbar">
        <span className="sandbox-lang-badge">{displayLang}</span>
        <div className="sandbox-toolbar-actions">
          <button
            className="sandbox-edit-btn"
            onClick={() => setMode(mode === "batch" ? "interactive" : "batch")}
            aria-label="Toggle sandbox mode"
          >
            {mode === "batch" ? "Batch" : "Interactive"}
          </button>
          <button className="sandbox-edit-btn" onClick={handleReset} aria-label="Reset code">
            Reset
          </button>
          {mode === "batch" ? (
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
          ) : (
            <button
              className="sandbox-run-btn"
              onClick={handleRunInteractive}
              disabled={interactive.isStarting || interactive.isConnected}
              aria-label={`Run ${normalizedLanguage} code interactively`}
            >
              {interactive.isStarting ? (
                <>
                  <span className="sandbox-spinner" /> Starting...
                </>
              ) : (
                "Run Interactive"
              )}
            </button>
          )}
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

      {mode === "interactive" && (
        <InteractiveTerminal
          output={interactive.output}
          stderr={interactive.stderr}
          isConnected={interactive.isConnected}
          isStarting={interactive.isStarting}
          error={interactive.error}
          exitCode={interactive.exitCode}
          onInput={interactive.sendInput}
          onStop={interactive.reset}
        />
      )}
    </div>
  );
}

function needsInteractive(code: string): boolean {
  return /\b(input|readline|scanf|gets)\s*\(/.test(code);
}
