import { FormEvent, useState } from "react";

interface InteractiveTerminalProps {
  output: string;
  stderr: string;
  isConnected: boolean;
  isStarting: boolean;
  error: string | null;
  exitCode: number | null;
  onInput: (data: string) => void;
  onStop: () => void;
}

export function InteractiveTerminal({
  output,
  stderr,
  isConnected,
  isStarting,
  error,
  exitCode,
  onInput,
  onStop,
}: InteractiveTerminalProps) {
  const [input, setInput] = useState("");

  const submit = (event: FormEvent) => {
    event.preventDefault();
    if (!input || !isConnected) return;
    onInput(`${input}\n`);
    setInput("");
  };

  return (
    <div className="interactive-terminal" data-testid="interactive-terminal">
      <div className="interactive-terminal-header">
        <span>{isConnected ? "Interactive session connected" : isStarting ? "Starting..." : "Interactive session"}</span>
        <button type="button" className="sandbox-edit-btn" onClick={onStop}>
          Stop
        </button>
      </div>
      <pre className="interactive-terminal-screen" aria-live="polite">
        {output}
        {stderr && <span className="sandbox-stderr">{stderr}</span>}
        {error && <span className="sandbox-error">{error}</span>}
        {exitCode !== null && `\n[process exited with code ${exitCode}]\n`}
      </pre>
      <form className="interactive-terminal-input" onSubmit={submit}>
        <span>$</span>
        <input
          value={input}
          onChange={(event) => setInput(event.target.value)}
          disabled={!isConnected}
          placeholder={isConnected ? "Type stdin and press Enter" : "Waiting for connection"}
          aria-label="Interactive stdin"
        />
      </form>
    </div>
  );
}
