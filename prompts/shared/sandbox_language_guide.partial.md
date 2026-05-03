<available_sandbox_languages>
The learning platform provides an interactive code runner ("Run" button) for the following languages.
Learners can edit and execute code blocks directly in the browser.

| Language | Fence identifier | Example |
|----------|-----------------|---------|
| Python | `python` or `py` | ` ```python ` |
| JavaScript | `javascript` or `js` | ` ```javascript ` |
| TypeScript | `typescript` or `ts` | ` ```typescript ` |
| Rust | `rust` or `rs` | ` ```rust ` |
| Go | `go` or `golang` | ` ```go ` |
| C | `c` | ` ```c ` |
| C++ | `cpp` | ` ```cpp ` |
| Java | `java` | ` ```java ` |
| Ruby | `ruby` or `rb` | ` ```ruby ` |
| Bash | `bash` or `sh` | ` ```bash ` |

## Rules for Runnable Code Blocks

### DO — Make code runnable when possible

- When the chapter topic matches one of the supported languages, provide complete, syntactically correct code examples that learners can run immediately.
- Include a small, self-contained example that demonstrates the concept being taught. The example should produce visible output (print to stdout).
- If the code reads input, use hardcoded values in the example — the sandbox does not support interactive stdin.
- Keep examples concise (preferably under 30 lines). The sandbox has execution timeouts.

### DO NOT — Avoid frustrating the learner

- Do NOT generate code that requires network access — the sandbox has networking disabled.
- Do NOT generate code that depends on third-party packages not installed in the sandbox image. Only the standard library is available unless otherwise noted.
- Do NOT generate code that reads from files not created in the same code block — each execution starts with a clean workspace.
- Do NOT generate code that spawns subprocesses, uses OS-specific paths, or requires special system permissions.
- Do NOT generate infinite loops or unbounded recursion.
- Do NOT generate interactive input prompts (`input()`, `readline()`, `scanf()`).

### Language-Specific Notes

- **Python:** Standard library only (math, json, collections, itertools, etc.). No pip packages except `sympy` for symbolic math.
- **JavaScript/TypeScript:** Node.js standard library only. No npm packages. Use `console.log()` for output.
- **Rust:** No external crates. Code must be a single `main.rs` file. Use `println!()` for output.
- **Go:** Single `main.go` file. Standard library only.
- **C/C++:** Single `.c` / `.cpp` file. Standard library only (`stdio.h`, `stdlib.h`, `iostream`, `vector`, etc.).
- **Java:** Single class file. Standard library only. Class must be public and contain a `main` method.
- **Ruby:** Standard library only.
- **Bash:** POSIX-compatible commands only. No `curl`, `wget`, or network tools (network is disabled).

### When to Use Non-Runnable Code Blocks

- ` ```bash ` for shell commands that a learner would type in their own terminal (not run in the sandbox).
- ` ```text ` for expected output, transcripts, or pseudocode.
- ` ```sql `, ` ```html `, ` ```css `, ` ```yaml `, ` ```json ` for non-executable formats.
- ` ```diff ` for showing code changes.
- Fenceless inline `` `code` `` for identifiers, operators, file names, and short references.

### Exercise Integration

- When exercises ask the learner to write code, mention which language fence to use so the Run button appears.
- Provide a "starter template" as a fenced code block with the correct language identifier.
- In solutions, show the expected output in a separate ` ```text ` block, never as a comment inside the code.
</available_sandbox_languages>
