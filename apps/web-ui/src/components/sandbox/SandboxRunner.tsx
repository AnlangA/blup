import { useSandboxExecute } from "../../hooks/query";
import { useSessionStore } from "../../state/sessionStore";
import type { SandboxExecuteRequest } from "../../api/client";

interface SandboxRunnerProps {
  language: string;
  code: string;
}

const SUPPORTED_LANGUAGES: Record<string, SandboxExecuteRequest["language"]> = {
  python: "python",
  py: "python",
  javascript: "javascript",
  js: "javascript",
  node: "javascript",
  rust: "rust",
  rs: "rust",
  typst: "typst",
};

export function SandboxRunner({ language, code }: SandboxRunnerProps) {
  const sessionId = useSessionStore((s) => s.sessionId);
  const sandbox = useSandboxExecute();

  const normalizedLanguage = SUPPORTED_LANGUAGES[language.toLowerCase()];
  if (!normalizedLanguage) return null;

  const handleRun = () => {
    if (!sessionId || sandbox.isRunning) return;
    sandbox.execute({
      session_id: sessionId,
      language: normalizedLanguage,
      code,
      timeout_secs: 30,
    });
  };

  return (
    <div className="sandbox-runner" data-testid="sandbox-runner">
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

      {(sandbox.stdout ||
        sandbox.stderr ||
        sandbox.error ||
        sandbox.exitCode !== null) && (
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
              <span>
                Exit code: {sandbox.exitCode}
              </span>
              {sandbox.durationMs !== null && (
                <span>
                  Duration: {sandbox.durationMs}ms
                </span>
              )}
            </div>
          )}
        </div>
      )}
    </div>
  );
}

export { SUPPORTED_LANGUAGES };
