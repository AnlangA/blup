# Add Multi-Language Sandbox Plan

## Purpose

This document describes the plan to extend the sandbox system from Python / JavaScript / Typst to support a broad set of programming languages, and to update the prompt system so the teaching agent can effectively generate and explain sandbox-runnable code examples.

---

## 1. Current Architecture Recap

```
Markdown code fence            Frontend                        Backend
────────────────────    ────────────────────────    ─────────────────────────────
```python               MarkdownRenderer.tsx         agent-core/sandbox.rs
print("hi")   ──────►   extract data-language        language_to_toolkind()
```                        ↓                            "python" → PythonExec
                     SUPPORTED_LANGUAGES lookup            ↓
                     "python" → "python"             ToolKind::to_image()
                          ↓                             "sandbox-python:latest"
                     SandboxRunner POST                    ↓
                     { language, code }              docker run sandbox-python:latest
                                                       python -c "code"
```

**Key observation:** Adding one language today requires touching at minimum **6 files** across 3 layers (schema, Rust backend, TypeScript frontend). This is error-prone and does not scale.

---

## 2. Architecture Improvement: Centralised Sandbox Registry

### 2.1 Problem

Every new language must be added to:

| Layer | File | What changes |
|-------|------|-------------|
| Schema | `schemas/sandbox_request.v1.schema.json` | `tool_kind` enum |
| Rust model | `crates/sandbox-manager/src/models/request.rs` | `ToolKind` enum variant + `to_image()` + `to_language()` |
| Rust handler | `crates/agent-core/src/server/handlers/sandbox.rs` | `language_to_toolkind()` match arm |
| TS types | `apps/web-ui/src/api/client.ts` | `SandboxExecuteRequest.language` union |
| TS runner | `apps/web-ui/src/components/sandbox/SandboxRunner.tsx` | `SUPPORTED_LANGUAGES` entry |
| TS renderer | `apps/web-ui/src/components/content/MarkdownRenderer.tsx` | (imports `SUPPORTED_LANGUAGES`) |

### 2.2 Solution: Single Source of Truth

Create a language registry file that serves as the canonical definition:

```
sandboxes/definitions/registry.yaml
```

```yaml
# Canonical sandbox language registry.
# Backend (Rust) and frontend (TypeScript) code generators read this file.
# To add a language: add an entry here + a Dockerfile in sandboxes/docker/.

languages:
  python:
    tool_kind: python_exec
    image: sandbox-python:latest
    execution_model: interpreted        # interpreted | compiled
    entrypoint: ["python", "-c"]
    aliases: [py, python3]
    display: Python
    schema_language: python
    compile_timeout_secs: 0             # 0 = no compile step
    run_timeout_secs: 10
    memory_mb: 512

  javascript:
    tool_kind: node_exec
    image: sandbox-node:latest
    execution_model: interpreted
    entrypoint: ["node", "-e"]
    aliases: [js, node]
    display: JavaScript
    schema_language: javascript
    compile_timeout_secs: 0
    run_timeout_secs: 10
    memory_mb: 512

  typescript:
    tool_kind: typescript_compile_run
    image: sandbox-typescript:latest
    execution_model: compiled
    runner_script: sandbox-run-ts        # wrapper inside image
    aliases: [ts]
    display: TypeScript
    schema_language: typescript
    compile_timeout_secs: 30
    run_timeout_secs: 10
    memory_mb: 512

  rust:
    tool_kind: rust_compile_run
    image: sandbox-rust:latest
    execution_model: compiled
    runner_script: sandbox-run-rust
    aliases: [rs]
    display: Rust
    schema_language: rust
    compile_timeout_secs: 60
    run_timeout_secs: 10
    memory_mb: 1024

  go:
    tool_kind: go_compile_run
    image: sandbox-go:latest
    execution_model: compiled
    runner_script: sandbox-run-go
    aliases: [golang]
    display: Go
    schema_language: go
    compile_timeout_secs: 30
    run_timeout_secs: 10
    memory_mb: 512

  c:
    tool_kind: c_compile_run
    image: sandbox-c:latest
    execution_model: compiled
    runner_script: sandbox-run-c
    aliases: []
    display: C
    schema_language: c
    compile_timeout_secs: 20
    run_timeout_secs: 10
    memory_mb: 256

  cpp:
    tool_kind: cpp_compile_run
    image: sandbox-cpp:latest
    execution_model: compiled
    runner_script: sandbox-run-cpp
    aliases: ["c++"]
    display: C++
    schema_language: cpp
    compile_timeout_secs: 30
    run_timeout_secs: 10
    memory_mb: 512

  java:
    tool_kind: java_compile_run
    image: sandbox-java:latest
    execution_model: compiled
    runner_script: sandbox-run-java
    aliases: []
    display: Java
    schema_language: java
    compile_timeout_secs: 30
    run_timeout_secs: 10
    memory_mb: 512

  ruby:
    tool_kind: ruby_exec
    image: sandbox-ruby:latest
    execution_model: interpreted
    entrypoint: ["ruby", "-e"]
    aliases: [rb]
    display: Ruby
    schema_language: ruby
    compile_timeout_secs: 0
    run_timeout_secs: 10
    memory_mb: 512

  bash:
    tool_kind: bash_exec
    image: sandbox-bash:latest
    execution_model: interpreted
    entrypoint: ["bash", "-c"]
    aliases: [sh, shell, zsh]
    display: Bash
    schema_language: bash
    compile_timeout_secs: 0
    run_timeout_secs: 10
    memory_mb: 128

  typst:
    tool_kind: typst_compile
    image: sandbox-typst:latest
    execution_model: compiled
    runner_script: sandbox-run-typst
    aliases: []
    display: Typst
    schema_language: typst
    compile_timeout_secs: 60
    run_timeout_secs: 10
    memory_mb: 1024
```

### 2.3 Code Generation Strategy

A build script (`tools/generate-sandbox-registry` or Rust `build.rs`) reads `registry.yaml` and generates:

**Rust output** (`crates/sandbox-manager/src/generated.rs`):

```rust
// AUTO-GENERATED from sandboxes/definitions/registry.yaml — do not edit by hand.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolKind {
    PythonExec,
    NodeExec,
    TypeScriptCompileRun,
    RustCompileRun,
    GoCompileRun,
    CCompileRun,
    CppCompileRun,
    JavaCompileRun,
    RubyExec,
    BashExec,
    TypstCompile,
}

impl ToolKind {
    pub fn from_language(lang: &str) -> Option<Self> {
        match lang.to_lowercase().as_str() {
            "python" | "py" | "python3" => Some(Self::PythonExec),
            "javascript" | "js" | "node" => Some(Self::NodeExec),
            "typescript" | "ts" => Some(Self::TypeScriptCompileRun),
            "rust" | "rs" => Some(Self::RustCompileRun),
            "go" | "golang" => Some(Self::GoCompileRun),
            "c" => Some(Self::CCompileRun),
            "cpp" | "c++" => Some(Self::CppCompileRun),
            "java" => Some(Self::JavaCompileRun),
            "ruby" | "rb" => Some(Self::RubyExec),
            "bash" | "sh" | "shell" | "zsh" => Some(Self::BashExec),
            "typst" => Some(Self::TypstCompile),
            _ => None,
        }
    }

    pub fn to_image(&self) -> &str { /* generated from registry */ }
    pub fn to_language(&self) -> &str { /* generated from registry */ }
    pub fn execution_model(&self) -> ExecutionModel { /* interpreted or compiled */ }
    pub fn entrypoint(&self) -> Option<&[String]> { /* for interpreted langs */ }
    pub fn runner_script(&self) -> Option<&str> { /* for compiled langs */ }
    pub fn schema_language(&self) -> &str { /* for JSON Schema enum */ }
    pub fn default_limits(&self) -> SandboxLimits { /* per-language defaults */ }
}
```

**TypeScript output** (`apps/web-ui/src/api/generated-sandbox.ts`):

```typescript
// AUTO-GENERATED from sandboxes/definitions/registry.yaml — do not edit by hand.

export const SUPPORTED_LANGUAGES: Record<string, SandboxLanguage> = {
  python: "python", py: "python", python3: "python",
  javascript: "javascript", js: "javascript", node: "javascript",
  typescript: "typescript", ts: "typescript",
  rust: "rust", rs: "rust",
  go: "go", golang: "go",
  c: "c",
  cpp: "cpp", "c++": "cpp",
  java: "java",
  ruby: "ruby", rb: "ruby",
  bash: "bash", sh: "bash", shell: "bash", zsh: "bash",
  typst: "typst",
} as const;

export type SandboxLanguage = "python" | "javascript" | "typescript" | "rust" | "go" | "c" | "cpp" | "java" | "ruby" | "bash" | "typst";
```

**JSON Schema snippet** (injected into `schemas/sandbox_request.v1.schema.json` during build):

```json
"tool_kind": {
  "enum": ["python_exec", "node_exec", "typescript_compile_run", "rust_compile_run", "go_compile_run", "c_compile_run", "cpp_compile_run", "java_compile_run", "ruby_exec", "bash_exec", "typst_compile"]
}
```

This means: **adding a new language = one line in registry.yaml + one Dockerfile**. All boilerplate is auto-generated.

---

## 3. Language Priority Tiers

### Tier 1 — Immediate (already partially supported or highest teaching demand)

| Language | Status | Work needed |
|----------|--------|------------|
| **Python** | Done | None |
| **JavaScript** | Done | None |
| **TypeScript** | New | Dockerfile, registry entry |
| **Rust** | Definition exists, missing Dockerfile | Dockerfile, registry entry, fix frontend |
| **Go** | New | Dockerfile, registry entry |
| **Bash** | New | Dockerfile, registry entry |

### Tier 2 — High teaching value

| Language | Rationale |
|----------|----------|
| **C** | Systems programming fundamentals, data structures |
| **C++** | OOP, competitive programming, game dev |
| **Java** | CS curriculum standard, enterprise teaching |
| **Ruby** | Web development, scripting |

### Tier 3 — Specialized / future

| Language | Rationale |
|----------|----------|
| **Kotlin** | Android, modern JVM |
| **Swift** | iOS/macOS development |
| **Zig** | Modern systems programming, simple toolchain |
| **R** | Statistics, data science |
| **SQL** | Database query teaching (requires sqlite sandbox, not code execution) |
| **Lua** | Embedded scripting, game modding |
| **WASM** | Browser runtime target |

---

## 4. Docker Image Standard

### 4.1 Interpreted Language Pattern

```dockerfile
# sandboxes/docker/Dockerfile.ruby
FROM ruby:3.3-alpine

RUN adduser -D -u 1000 sandbox
USER sandbox
WORKDIR /workspace

# For interpreted languages: entrypoint = runtime + flag to execute code string
ENTRYPOINT ["ruby", "-e"]
```

Container executor passes code as the final argument:
```
docker run ... sandbox-ruby:latest ruby -e "puts 'hello'"
```

### 4.2 Compiled Language Pattern

Every compiled-language image includes a standardised runner script at `/usr/local/bin/sandbox-run-<lang>`. The script:

1. Reads source from **stdin** (not command-line argument)
2. Writes to a temp file in `/workspace`
3. Compiles with appropriate flags
4. Runs the compiled binary
5. Propagates exit code

```dockerfile
# sandboxes/docker/Dockerfile.rust
FROM rust:1.83-alpine

RUN adduser -D -u 1000 sandbox

# Standard runner: read stdin → compile → run
RUN printf '#!/bin/sh\nset -e\nSRC=$(mktemp /workspace/main_XXXXXX.rs)\ncat > "$SRC"\nrustc -C opt-level=0 "$SRC" -o "${SRC%.rs}"\n"${SRC%.rs}"\n' \
    > /usr/local/bin/sandbox-run-rust && chmod +x /usr/local/bin/sandbox-run-rust

USER sandbox
WORKDIR /workspace

# entrypoint overridden at runtime to use runner script
```

Container executor pipes code via stdin:
```rust
// In container.rs, for compiled languages:
cmd.arg(image)
   .args(["--entrypoint", runner_script])
   .stdin(std::process::Stdio::piped());
// code bytes are written to stdin after spawn
```

**Why stdin over `sh -c`?**

| Approach | Shell injection | Special chars | Multi-line safety |
|----------|----------------|---------------|-------------------|
| `sh -c "echo 'code' > file && compile"` | Risk — `' " $ \` need escaping | Fragile | Error-prone |
| stdin pipe + runner script | None | Fully preserved | Safe by design |

### 4.3 Runner Script Templates

**Rust:**
```sh
#!/bin/sh
set -e
SRC=$(mktemp /workspace/main_XXXXXX.rs)
cat > "$SRC"
rustc -C opt-level=0 "$SRC" -o /workspace/a.out
/workspace/a.out
```

**Go:**
```sh
#!/bin/sh
set -e
DIR=$(mktemp -d /workspace/main_XXXXXX)
cat > "$DIR/main.go"
cd "$DIR" && go build -o a.out main.go && ./a.out
```

**C:**
```sh
#!/bin/sh
set -e
SRC=$(mktemp /workspace/main_XXXXXX.c)
cat > "$SRC"
gcc -Wall -O0 "$SRC" -o /workspace/a.out
/workspace/a.out
```

**C++:**
```sh
#!/bin/sh
set -e
SRC=$(mktemp /workspace/main_XXXXXX.cpp)
cat > "$SRC"
g++ -Wall -O0 "$SRC" -o /workspace/a.out
/workspace/a.out
```

**Java:**
```sh
#!/bin/sh
set -e
SRC=$(mktemp /workspace/Main_XXXXXX.java)
# Extract class name from source or default to Main
cat > "$SRC"
javac "$SRC" -d /workspace/out
CLASSFILE=$(basename "$SRC" .java)
java -cp /workspace/out "$CLASSFILE"
```

**TypeScript:**
```sh
#!/bin/sh
set -e
SRC=$(mktemp /workspace/main_XXXXXX.ts)
cat > "$SRC"
npx ts-node --transpile-only "$SRC"
```

### 4.4 Resource Limits Per Language

| Language | compile_timeout | run_timeout | memory_mb | disk_mb | rationale |
|----------|----------------|-------------|-----------|---------|----------|
| Python | 0 | 10 | 512 | 100 | Fast startup |
| JavaScript | 0 | 10 | 512 | 100 | Fast startup |
| TypeScript | 30 | 10 | 512 | 200 | tsc compilation |
| Rust | 60 | 10 | 1024 | 500 | rustc is memory-heavy |
| Go | 30 | 10 | 512 | 300 | go build uses cache |
| C | 20 | 10 | 256 | 100 | gcc is fast and lean |
| C++ | 30 | 10 | 512 | 200 | g++ heavier than gcc |
| Java | 30 | 10 | 512 | 300 | javac + JVM startup |
| Ruby | 0 | 10 | 512 | 100 | Fast startup |
| Bash | 0 | 10 | 128 | 100 | Minimal overhead |

---

## 5. Container Executor Changes

### 5.1 Current State

`container.rs` currently has a hardcoded `needs_shell` flag for Typst:

```rust
let needs_shell = matches!(request.tool_kind, ToolKind::TypstCompile);
```

### 5.2 Required Change

Replace with an `execution_model` check from the registry:

```rust
match request.tool_kind.execution_model() {
    ExecutionModel::Interpreted => {
        // docker run <image> <entrypoint> <code>
        cmd.arg(image);
        for arg in request.tool_kind.entrypoint().unwrap_or(&[]) {
            cmd.arg(arg);
        }
        cmd.arg(&request.code);
    }
    ExecutionModel::Compiled => {
        // docker run --entrypoint <runner> <image>
        // code piped via stdin
        cmd.arg(image)
           .args(["--entrypoint", request.tool_kind.runner_script().unwrap()]);
        // ... spawn, then write request.code to stdin
    }
}
```

### 5.3 Stdin Implementation

```rust
use std::io::Write;

// For compiled languages, pipe code via stdin after spawning
if request.tool_kind.execution_model() == ExecutionModel::Compiled {
    cmd.stdin(std::process::Stdio::piped());
    let mut child = cmd.spawn()?;
    if let Some(ref mut stdin) = child.stdin {
        stdin.write_all(request.code.as_bytes())?;
        // stdin is dropped here → closes pipe → runner reads EOF
    }
    let output = child.wait_with_output()?;
    // ... parse output
}
```

---

## 6. Schema Changes

### 6.1 sandbox_request.v1.schema.json

The `tool_kind` and `language` enums must be extended (or auto-generated from registry):

```json
"tool_kind": {
  "enum": [
    "python_exec", "node_exec", "typescript_compile_run",
    "rust_compile_run", "go_compile_run", "c_compile_run",
    "cpp_compile_run", "java_compile_run", "ruby_exec",
    "bash_exec", "typst_compile"
  ]
},
"language": {
  "enum": [
    "python", "javascript", "typescript", "rust", "go",
    "c", "cpp", "java", "ruby", "bash", "typst"
  ]
}
```

Alternatively, remove the `language` enum restriction (use `"type": "string"`) and let the backend validate — this avoids schema churn when adding languages.

---

## 7. Frontend Changes

### 7.1 SandboxRunner.tsx

Replace hardcoded `SUPPORTED_LANGUAGES` with generated import:

```typescript
// Before (current):
const SUPPORTED_LANGUAGES: Record<string, SandboxExecuteRequest["language"]> = {
  python: "python", py: "python",
  javascript: "javascript", js: "javascript",
  rust: "rust", rs: "rust",
  typst: "typst",
};

// After:
import { SUPPORTED_LANGUAGES } from "../../api/generated-sandbox";
```

### 7.2 api/client.ts

Replace hardcoded union type:

```typescript
// Before:
language: "python" | "javascript" | "rust" | "typst";

// After:
import type { SandboxLanguage } from "./generated-sandbox";
language: SandboxLanguage;
```

### 7.3 Language Display Name

Extend the SandboxRunner to show human-readable language names instead of raw codes:

```typescript
const LANGUAGE_DISPLAY: Record<string, string> = {
  python: "Python", javascript: "JS", typescript: "TS",
  rust: "Rust", go: "Go", c: "C", cpp: "C++",
  java: "Java", ruby: "Ruby", bash: "Bash", typst: "Typst",
};
```

---

## 8. Prompt Modifications

The teaching agent needs to know which languages are sandbox-runnable so it can generate appropriate code examples. Currently, prompts describe code formatting rules but don't mention sandbox availability.

### 8.1 New Partial: `sandbox_language_guide.partial.md`

Create a new shared prompt partial that is injected into `chapter_teaching` and `question_answering`:

```markdown
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
```

### 8.2 Changes to `chapter_teaching.v1.prompt.md`

Insert the partial reference and update the "Match the Teaching Medium" section:

```markdown
<input>
- **chapter_id**: `{{chapter_id}}`
- **user_profile**: `{{user_profile}}`
- **curriculum_context**: `{{curriculum_context}}`
</input>

{{sandbox_language_guide}}        <!-- ← NEW: inject the partial -->

<instructions>
...
```

Update the **Differentiation by Learner Level** section to leverage sandbox:

```markdown
**For code-centric chapters (any tier):**
- Every code example MUST use a language fence that matches one of the available sandbox languages.
- Beginning learners: provide complete, runnable examples. The learner should be able to click Run and see output immediately.
- Intermediate learners: provide partial examples where the learner fills in the missing logic, then runs to verify.
- Advanced learners: provide a problem statement and expected output; the learner writes the full solution.
```

Update the **constraints** section:

```markdown
<constraints>
...
- Every code example must be syntactically correct for the stated language AND compatible with the sandbox environment (no network, no external packages, no file I/O beyond the code block itself).
- When the chapter topic is a supported programming language, at least 2 code blocks must be runnable (produce visible stdout).
- Do not generate code that contains interactive input prompts.
</constraints>
```

### 8.3 Changes to `question_answering.v1.prompt.md`

Add sandbox context so the agent can reference the Run button:

```markdown
<input>
- **chapter_id**: `{{chapter_id}}`
- **user_profile**: `{{user_profile}}`
- **question**: `{{question}}`
- **chapter_content**: `{{chapter_content}}`
</input>

{{sandbox_language_guide}}        <!-- ← NEW -->
```

Add a section:

```markdown
## When the Learner Asks About Code

If the learner asks a question about code in a supported language:
- Explain the concept first in prose.
- Then provide a minimal, runnable example in a fenced code block with the correct language identifier.
- If the learner's own code has a bug, show the corrected version in a code block they can run.
- Mention that they can click the "Run" button to execute the code and see the output.
```

### 8.4 Changes to `chapter_markdown_repair.v1.prompt.md`

Add instruction to fix language identifiers:

```markdown
## Repair priorities
...
- If the chapter contains code blocks, ensure each fenced code block has a correct language identifier from the supported list: python, javascript, typescript, rust, go, c, cpp, java, ruby, bash, text, sql, html, css, json, yaml, diff.
- If a code block is missing a language identifier or uses an unsupported one, correct it to the closest matching identifier.
- Do NOT add language identifiers to blocks that are clearly plain text, expected output, or transcripts — use `text` for those.
```

### 8.5 Changes to `output_format_guide.partial.md`

Expand the code block section:

```markdown
## Markdown Output
...
- When code blocks are needed, use fenced code blocks only for multi-line code, commands, configs, or literal output that materially helps the explanation.
- Every fenced code block must include the correct language identifier for the material. Supported runnable languages: `python`, `javascript`, `typescript`, `rust`, `go`, `c`, `cpp`, `java`, `ruby`, `bash`. Non-runnable: `text`, `sql`, `html`, `css`, `json`, `yaml`, `xml`, `diff`, `toml`.
- Use `bash` for shell commands and `text` for plain-text output, transcripts, or pseudocode that is not valid source code.
- Never nest fenced code blocks or wrap the entire response in a single fenced code block.
- If the topic is code-centric, prefer languages that the learner can actually run in the platform's sandbox.
...
```

---

## 9. Implementation Sequence

### Phase A: Foundation (registry + code generation)

1. Create `sandboxes/definitions/registry.yaml` with all Tier 1 + Tier 2 languages
2. Write `tools/generate-sandbox-registry` script (Python or Rust)
3. Generate `ToolKind` enum, TypeScript types, and schema snippet from registry
4. Refactor `container.rs` to use `execution_model` instead of `needs_shell`
5. Add stdin-pipe support for compiled languages
6. Run existing tests — ensure Python, JavaScript, and Typst still work

### Phase B: Tier 1 Docker Images

7. Write `Dockerfile.rust` + runner script
8. Write `Dockerfile.typescript` + runner script
9. Write `Dockerfile.go` + runner script
10. Write `Dockerfile.bash` (simple alpine image with bash)
11. Build and test each image with `tools/sandbox-builder`
12. Add E2E tests for each new language in `tests/e2e/`

### Phase C: Prompt Updates

13. Create `prompts/shared/sandbox_language_guide.partial.md`
14. Update `chapter_teaching.v1.prompt.md` to inject the partial
15. Update `question_answering.v1.prompt.md` to inject the partial
16. Update `chapter_markdown_repair.v1.prompt.md`
17. Update `output_format_guide.partial.md`
18. Add prompt contract tests with mock LLM outputs containing sandbox-runnable code

### Phase D: Tier 2 Docker Images

19. Write Dockerfiles for C, C++, Java, Ruby
20. Build and test

### Phase E: Frontend Polish

21. Replace hardcoded `SUPPORTED_LANGUAGES` with generated import
22. Add language display names
23. Add per-language icon/badge in SandboxRunner
24. E2E tests for all supported language badges

---

## 10. Acceptance Criteria (验收标准)

### 10.1 Core Acceptance Rule

**所有测试必须 100% 通过，零失败、零跳过。** 任何一个测试失败都视为该阶段未完成，不得进入下一阶段。

### 10.2 Per-Phase Acceptance Gates

| Phase | Gate | 条件 |
|-------|------|------|
| Phase A | Registry + codegen | `cargo test -p sandbox-manager` 全部通过；`npm run typecheck` 零错误；生成的 schema 通过 `scripts/schema-check` |
| Phase B | Tier 1 Docker images | 每个新语言的 Docker 镜像合约测试全部通过；E2E 测试覆盖 Python/JS/TS/Rust/Go/Bash |
| Phase C | Prompt updates | 所有 prompt 合约测试通过；`chapter_teaching` / `question_answering` 输出的代码块使用正确的 language identifier |
| Phase D | Tier 2 Docker images | C/C++/Java/Ruby 镜像合约测试全部通过；E2E 测试扩展覆盖 |
| Phase E | Frontend polish | 所有 Playwright E2E 测试通过；组件测试通过；TypeScript typecheck 零错误 |

### 10.3 CI 最终验收

在 CI pipeline 中必须运行以下命令且全部返回零退出码：

```bash
# Rust
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets

# TypeScript / Web UI
cd apps/web-ui
npm run typecheck
npm run lint
npm run test -- --run        # Vitest unit/component tests
npx playwright test          # E2E tests

# Schemas
scripts/schema-check

# Registry consistency check
tools/generate-sandbox-registry --check   # verify generated files match registry
```

---

## 11. Detailed Test Plan

### 11.1 Registry & Code Generation Tests

**位置:** `crates/sandbox-manager/tests/registry_tests.rs`（新增）

**测试目标:** 验证 `registry.yaml` 能被正确解析，生成的 Rust 和 TypeScript 代码有效。

| 测试ID | 测试名称 | 测试内容 | 预期结果 |
|--------|---------|---------|---------|
| REG-001 | `parse_registry_yaml` | 解析 `registry.yaml`，验证所有语言条目结构完整 | 所有语言解析成功，无缺失字段 |
| REG-002 | `registry_all_languages_have_image` | 每个语言条目的 `image` 字段非空 | 无空 image |
| REG-003 | `registry_all_languages_have_tool_kind` | 每个语言条目有唯一 `tool_kind` | 无重复 tool_kind |
| REG-004 | `registry_all_languages_have_execution_model` | `execution_model` 只允许 `interpreted` 或 `compiled` | 无非法值 |
| REG-005 | `registry_interpreted_langs_have_entrypoint` | `execution_model=interpreted` 的语言必须有 `entrypoint` | 无缺失 |
| REG-006 | `registry_compiled_langs_have_runner_script` | `execution_model=compiled` 的语言必须有 `runner_script` | 无缺失 |
| REG-007 | `generated_rust_compiles` | 生成的 `generated.rs` 能通过 `cargo check` | 编译成功 |
| REG-008 | `generated_typescript_typechecks` | 生成的 `generated-sandbox.ts` 能通过 `tsc --noEmit` | typecheck 零错误 |
| REG-009 | `generated_schema_valid_json` | 生成的 schema snippet 是合法的 JSON Schema | `scripts/schema-check` 通过 |
| REG-010 | `registry_generated_files_in_sync` | `tools/generate-sandbox-registry --check` 验证生成文件与 registry 一致 | 退出码 0，无差异 |
| REG-011 | `toolkind_from_language_all_aliases` | 每个语言的 `aliases` 都能通过 `ToolKind::from_language()` 正确映射 | `py`→PythonExec, `js`→NodeExec, `rs`→RustCompileRun 等 |
| REG-012 | `toolkind_from_language_unknown` | 未知语言返回 `None` | `from_language("brainfuck")` → None |
| REG-013 | `toolkind_from_language_case_insensitive` | 大小写不敏感 | `"PYTHON"`, `"Python"`, `"python"` 都映射到 PythonExec |

### 11.2 Docker Image Build Tests

**位置:** `sandboxes/tests/` 目录下（新增测试脚本）

**测试目标:** 验证每个 Dockerfile 能成功构建，镜像大小在合理范围内。

| 测试ID | 测试名称 | 测试内容 | 预期结果 |
|--------|---------|---------|---------|
| IMG-001 | `build_python_image` | `docker build -f Dockerfile.python .` | 构建成功，退出码 0 |
| IMG-002 | `build_node_image` | `docker build -f Dockerfile.node .` | 构建成功，退出码 0 |
| IMG-003 | `build_typescript_image` | `docker build -f Dockerfile.typescript .` | 构建成功，退出码 0 |
| IMG-004 | `build_rust_image` | `docker build -f Dockerfile.rust .` | 构建成功，退出码 0 |
| IMG-005 | `build_go_image` | `docker build -f Dockerfile.go .` | 构建成功，退出码 0 |
| IMG-006 | `build_bash_image` | `docker build -f Dockerfile.bash .` | 构建成功，退出码 0 |
| IMG-007 | `build_c_image` | `docker build -f Dockerfile.c .` | 构建成功，退出码 0 |
| IMG-008 | `build_cpp_image` | `docker build -f Dockerfile.cpp .` | 构建成功，退出码 0 |
| IMG-009 | `build_java_image` | `docker build -f Dockerfile.java .` | 构建成功，退出码 0 |
| IMG-010 | `build_ruby_image` | `docker build -f Dockerfile.ruby .` | 构建成功，退出码 0 |
| IMG-011 | `image_size_under_limit` | 每个镜像大小 < 2GB（编译型）或 < 500MB（解释型） | 所有镜像在限制内 |
| IMG-012 | `image_has_sandbox_user` | 每个镜像内存在 uid=1000 的 sandbox 用户 | `docker run <img> id -u` 输出 `1000` |
| IMG-013 | `image_has_no_root_process` | 容器默认不以 root 运行 | `docker run <img> whoami` 输出非 `root` |

### 11.3 Docker Image Contract Tests（每个语言独立）

**位置:** `sandboxes/tests/contract/` 目录下（新增）

**测试目标:** 验证每个 Docker 镜像的功能正确性——能执行代码、能报错、能被资源限制终止。

#### 11.3.1 通用合约测试（每个语言都要运行）

| 测试ID | 测试名称 | 测试内容 | 预期结果 |
|--------|---------|---------|---------|
| CT-001 | `hello_world` | 运行输出 "hello world" 的最小程序 | stdout 包含 "hello world"，exit_code=0 |
| CT-002 | `print_stdout` | 程序输出多行文本 | stdout 包含所有行，顺序正确 |
| CT-003 | `print_stderr` | 程序向 stderr 输出 | stderr 包含输出内容 |
| CT-004 | `syntax_error` | 包含语法错误的代码 | exit_code ≠ 0，stderr 非空 |
| CT-005 | `runtime_error` | 运行时错误（除零、越界等） | exit_code ≠ 0 |
| CT-006 | `exit_code_nonzero` | 程序主动 `exit(42)` 或等效操作 | exit_code = 42 |
| CT-007 | `run_timeout` | 死循环，设置 `run_timeout_secs=2` | status = `timeout_run`，duration_ms 约 2000 |
| CT-008 | `network_disabled_default` | 尝试发起 TCP 连接到 example.com:80 | 连接失败（exit_code ≠ 0 或 timeout） |
| CT-009 | `no_file_system_persistence` | 程序写文件后退出；再次运行读同一文件 | 第二次运行读不到第一次写入的文件 |
| CT-010 | `stdout_truncation` | 程序输出超过 64KB 的文本 | `stdout_truncated = true`，stdout 长度 = 64KB |
| CT-011 | `stdin_closed` | 程序尝试从 stdin 读取（对于编译型语言，stdin 在代码写入后立即关闭） | 读取操作返回 EOF/空，不挂起 |
| CT-012 | `unicode_support` | 程序输出 Unicode 字符（中文、emoji） | stdout 正确包含 Unicode 字符 |
| CT-013 | `empty_program` | 空代码或只有注释 | exit_code=0，stdout 可为空 |

#### 11.3.2 Python 专属合约测试

| 测试ID | 测试名称 | 测试内容 | 预期结果 |
|--------|---------|---------|---------|
| PY-001 | `standard_library_import` | `import json; print(json.dumps({"a":1}))` | stdout: `{"a": 1}` |
| PY-002 | `sympy_available` | `import sympy; print(sympy.Symbol('x'))` | stdout: `x` |
| PY-003 | `no_pip_packages` | `import requests` | ImportError，exit_code ≠ 0 |
| PY-004 | `no_input_hang` | `input()` 在无 stdin 时 | 抛出 EOFError，不挂起 |

#### 11.3.3 JavaScript 专属合约测试

| 测试ID | 测试名称 | 测试内容 | 预期结果 |
|--------|---------|---------|---------|
| JS-001 | `console_log` | `console.log("hello")` | stdout: `hello\n` |
| JS-002 | `async_await` | `(async () => { await Promise.resolve(); console.log("done"); })()` | stdout: `done\n` |
| JS-003 | `no_require_network` | `require('http').get(...)` | 失败或超时（网络禁用） |

#### 11.3.4 TypeScript 专属合约测试

| 测试ID | 测试名称 | 测试内容 | 预期结果 |
|--------|---------|---------|---------|
| TS-001 | `type_check_and_run` | 包含类型注解的 TS 代码能编译并运行 | exit_code=0，输出正确 |
| TS-002 | `type_error_caught` | 类型不匹配的代码 | 编译报错，exit_code ≠ 0 |
| TS-003 | `interface_and_generics` | 使用 interface 和泛型 | 正常编译运行 |

#### 11.3.5 Rust 专属合约测试

| 测试ID | 测试名称 | 测试内容 | 预期结果 |
|--------|---------|---------|---------|
| RS-001 | `hello_world_main` | `fn main() { println!("hello"); }` | stdout: `hello\n`，exit_code=0 |
| RS-002 | `compile_error` | 语法错误的 Rust 代码 | 编译失败，stderr 包含 `error`，exit_code ≠ 0 |
| RS-003 | `no_extern_crate` | 使用外部 crate（如 `use serde::*;`）的代码 | 编译失败 |
| RS-004 | `ownership_and_borrowing` | 涉及所有权和借用的合法 Rust 代码 | 正常编译运行 |
| RS-005 | `panic_caught` | `panic!("boom")` | 运行时错误，stderr 包含 panic 信息 |
| RS-006 | `compile_timeout` | 极复杂的泛型代码，设置 `compile_timeout_secs=5` | status = `timeout_compile` |

#### 11.3.6 Go 专属合约测试

| 测试ID | 测试名称 | 测试内容 | 预期结果 |
|--------|---------|---------|---------|
| GO-001 | `hello_world_main` | `package main; func main() { println("hello") }` | stdout: `hello\n` |
| GO-002 | `goroutine` | 使用 goroutine 的合法代码 | 正常编译运行 |
| GO-003 | `compile_error` | 语法错误的 Go 代码 | 编译失败，exit_code ≠ 0 |
| GO-004 | `no_external_module` | 引用外部 module 的代码 | 编译失败 |

#### 11.3.7 C 专属合约测试

| 测试ID | 测试名称 | 测试内容 | 预期结果 |
|--------|---------|---------|---------|
| C-001 | `hello_world` | `#include <stdio.h>; int main() { printf("hello\n"); return 0; }` | stdout: `hello\n`，exit_code=0 |
| C-002 | `segfault_caught` | 空指针解引用 | exit_code ≠ 0（SIGSEGV → 非零退出） |
| C-003 | `compile_warning_treated_as_error` | 启用 `-Wall -Werror` 后的警告代码 | 编译失败 |

#### 11.3.8 C++ 专属合约测试

| 测试ID | 测试名称 | 测试内容 | 预期结果 |
|--------|---------|---------|---------|
| CPP-001 | `hello_world` | `#include <iostream>; int main() { std::cout << "hello" << std::endl; return 0; }` | stdout: `hello\n` |
| CPP-002 | `stl_vector` | 使用 `std::vector` 的代码 | 正常编译运行 |
| CPP-003 | `template_usage` | 使用模板的代码 | 正常编译运行 |

#### 11.3.9 Java 专属合约测试

| 测试ID | 测试名称 | 测试内容 | 预期结果 |
|--------|---------|---------|---------|
| JV-001 | `hello_world` | `public class Main { public static void main(String[] args) { System.out.println("hello"); } }` | stdout: `hello\n` |
| JV-002 | `compile_error` | 语法错误的 Java 代码 | 编译失败，exit_code ≠ 0 |
| JV-003 | `class_name_mismatch` | 类名与 runner 推断的文件名不匹配 | 编译失败 |

#### 11.3.10 Ruby 专属合约测试

| 测试ID | 测试名称 | 测试内容 | 预期结果 |
|--------|---------|---------|---------|
| RB-001 | `hello_world` | `puts "hello"` | stdout: `hello\n` |
| RB-002 | `syntax_error` | 语法错误的 Ruby 代码 | exit_code ≠ 0 |
| RB-003 | `standard_library` | `require 'json'; puts JSON.generate({a:1})` | stdout 包含 JSON |

#### 11.3.11 Bash 专属合约测试

| 测试ID | 测试名称 | 测试内容 | 预期结果 |
|--------|---------|---------|---------|
| SH-001 | `hello_world` | `echo "hello"` | stdout: `hello\n` |
| SH-002 | `exit_code` | `exit 7` | exit_code = 7 |
| SH-003 | `no_curl_wget` | `curl http://example.com` | 命令不可用或连接失败 |
| SH-004 | `pipe_and_redirect` | `echo foo | grep foo > /dev/null && echo ok` | stdout: `ok\n` |

### 11.4 Sandbox Manager Unit Tests（Rust）

**位置:** `crates/sandbox-manager/src/lib.rs` 及相关模块测试（扩展现有测试）

**测试目标:** 验证 sandbox-manager 的核心逻辑在所有语言上正确。

| 测试ID | 测试名称 | 测试内容 | 预期结果 |
|--------|---------|---------|---------|
| SM-001 | `toolkind_to_image_all` | 每个 `ToolKind` 变体的 `to_image()` 返回正确的镜像名 | PythonExec→"sandbox-python:latest" 等 |
| SM-002 | `toolkind_to_language_all` | 每个 `ToolKind` 变体的 `to_language()` 返回正确的语言字符串 | PythonExec→"python" 等 |
| SM-003 | `toolkind_execution_model_all` | 每个变体的 `execution_model()` 正确 | PythonExec→Interpreted, RustCompileRun→Compiled |
| SM-004 | `toolkind_default_limits_per_lang` | 每个语言的 `default_limits()` 符合定义 | Rust: memory=1024, compile=60s 等 |
| SM-005 | `mock_executor_all_toolkinds` | `MockExecutor` 能处理所有 `ToolKind` 变体 | 所有变体无 panic |
| SM-006 | `sandbox_manager_all_languages` | `SandboxManager` 接收所有语言的请求并返回结果 | 所有语言正常执行 |
| SM-007 | `sandbox_request_new_python` | `SandboxRequest::new_python()` 构造正确 | tool_kind=PythonExec, language="python" |
| SM-008 | `sandbox_request_new_node` | `SandboxRequest::new_node()` 构造正确 | tool_kind=NodeExec, language="javascript" |
| SM-009 | `sandbox_request_serialize_all` | 所有语言的 SandboxRequest 能正确序列化/反序列化 | JSON 序列化往返一致 |
| SM-010 | `sandbox_result_serialize_all_statuses` | 所有 `ExecutionStatus` 变体能正确序列化 | JSON 格式正确 |

### 11.5 Container Executor Tests（Rust，需 Docker 环境）

**位置:** `crates/sandbox-manager/tests/container_executor_tests.rs`（新增，`#[ignore]` 在没有 Docker 的环境）

**测试目标:** 验证 `ContainerExecutor` 在真实 Docker 环境下能正确执行各类语言的代码。

| 测试ID | 测试名称 | 测试内容 | 预期结果 |
|--------|---------|---------|---------|
| CE-001 | `execute_python_hello` | Docker 执行 Python hello world | stdout 包含 "hello"，exit_code=0 |
| CE-002 | `execute_rust_hello` | Docker 执行 Rust hello world（stdin pipe） | stdout 包含 "hello"，exit_code=0 |
| CE-003 | `execute_go_hello` | Docker 执行 Go hello world（stdin pipe） | stdout 包含 "hello"，exit_code=0 |
| CE-004 | `execute_c_hello` | Docker 执行 C hello world（stdin pipe） | stdout 包含 "hello"，exit_code=0 |
| CE-005 | `execute_compiled_stdin_pipe` | 编译型语言代码通过 stdin 正确传递 | 代码内容完整到达容器内 |
| CE-006 | `execute_interpreted_args` | 解释型语言代码通过命令行参数正确传递 | 代码内容完整到达容器内 |
| CE-007 | `container_timeout_kill` | 超时容器被正确 kill | status=TimeoutRun，容器已删除 |
| CE-008 | `container_cleanup_after_success` | 执行成功后容器被自动删除 | `docker ps -a` 不含该容器 |
| CE-009 | `container_cleanup_after_error` | 执行失败后容器被自动删除 | `docker ps -a` 不含该容器 |
| CE-010 | `resource_usage_collected` | `docker inspect` 收集到资源使用数据 | `resource_usage` 字段非零 |
| CE-011 | `memory_limit_enforced` | 超过内存限制（如 64MB 运行大数组分配） | status=MemoryExceeded |
| CE-012 | `network_disabled_by_default` | 容器默认无网络 | 所有网络请求失败 |
| CE-013 | `no_new_privileges` | 容器无法获取额外权限 | `--security-opt no-new-privileges` 生效 |
| CE-014 | `read_only_filesystem` | 容器根文件系统只读（/workspace 和 /tmp 除外） | 写入 /etc 失败，写入 /workspace 成功 |

### 11.6 API Integration Tests（Rust）

**位置:** `tests/src/integration/` 目录下（扩展现有集成测试）

**测试目标:** 验证 `/api/sandbox/execute` SSE 端点对所有语言返回正确格式的 SSE 事件。

| 测试ID | 测试名称 | 测试内容 | 预期结果 |
|--------|---------|---------|---------|
| API-001 | `sandbox_execute_python` | POST `/api/sandbox/execute` with `language: "python"` | 返回 SSE 流：status→stdout→done |
| API-002 | `sandbox_execute_javascript` | POST with `language: "javascript"` | 同上 |
| API-003 | `sandbox_execute_typescript` | POST with `language: "typescript"` | 同上 |
| API-004 | `sandbox_execute_rust` | POST with `language: "rust"` | 同上 |
| API-005 | `sandbox_execute_go` | POST with `language: "go"` | 同上 |
| API-006 | `sandbox_execute_c` | POST with `language: "c"` | 同上 |
| API-007 | `sandbox_execute_cpp` | POST with `language: "cpp"` | 同上 |
| API-008 | `sandbox_execute_java` | POST with `language: "java"` | 同上 |
| API-009 | `sandbox_execute_ruby` | POST with `language: "ruby"` | 同上 |
| API-010 | `sandbox_execute_bash` | POST with `language: "bash"` | 同上 |
| API-011 | `sandbox_execute_alias_py` | POST with `language: "py"` | 正确路由到 Python sandbox |
| API-012 | `sandbox_execute_alias_js` | POST with `language: "js"` | 正确路由到 Node sandbox |
| API-013 | `sandbox_execute_alias_rs` | POST with `language: "rs"` | 正确路由到 Rust sandbox |
| API-014 | `sandbox_execute_unknown_language` | POST with `language: "brainfuck"` | 返回 400 错误，Validation error |
| API-015 | `sandbox_execute_sse_format` | 验证 SSE 事件格式正确 | `event:` + `data:` + 空行分隔 |
| API-016 | `sandbox_execute_sse_done_result` | 验证 `done` 事件包含 `exit_code` 和 `duration_ms` | JSON data 包含必需字段 |
| API-017 | `sandbox_health_endpoint` | GET `/api/sandbox/health` | 返回 `{ healthy: bool, images: [...] }` |
| API-018 | `sandbox_health_lists_all_images` | health 端点列出所有注册的语言镜像 | images 数组包含全部 11 个语言镜像 |
| API-019 | `sandbox_execute_invalid_session_id` | POST with 非法 UUID | 返回 400 错误 |

### 11.7 SSE Stream Format Tests (Rust)

**位置:** `tests/src/integration/` 目录下

**测试目标:** 验证 SSE 流的边界情况，与语言无关。

| 测试ID | 测试名称 | 测试内容 | 预期结果 |
|--------|---------|---------|---------|
| SSE-01 | `sse_ping_keepalive` | 长时间无事件的流是否发送 ping | 每 15 秒收到 ping 事件 |
| SSE-02 | `sse_stdout_multiline` | 多行 stdout 在多个 event 中返回 | 每行一个 stdout event |
| SSE-03 | `sse_stderr_streaming` | stderr 内容在多个 event 中返回 | 每行一个 stderr event |
| SSE-04 | `sse_error_event_on_failure` | 执行失败时返回 error event | event name = "error"，包含 code 和 message |
| SSE-05 | `sse_done_is_final_event` | done/error 之后无后续事件 | 流正确关闭 |

### 11.8 Frontend Component Tests（Vitest）

**位置:** `apps/web-ui/src/components/sandbox/__tests__/` （新增）

**测试目标:** 验证前端组件在所有语言下正确渲染和交互。

| 测试ID | 测试名称 | 测试内容 | 预期结果 |
|--------|---------|---------|---------|
| FE-001 | `sandbox_runner_renders_python` | `<SandboxRunner language="python" code="print('hi')" />` 渲染 | 显示 "Python" badge，Run 按钮可见 |
| FE-002 | `sandbox_runner_renders_all_languages` | 对所有 11 个语言渲染 SandboxRunner | 每个显示正确的 badge 文本 |
| FE-003 | `sandbox_runner_unknown_language_returns_null` | `<SandboxRunner language="haskell" code="..." />` | 组件返回 null，不渲染 |
| FE-004 | `sandbox_runner_reset_button` | 编辑代码后点击 Reset | 代码恢复为 initialCode |
| FE-005 | `sandbox_runner_run_button_disabled_during_execution` | 执行中 Run 按钮禁用 | 按钮 `disabled` 属性为 true |
| FE-006 | `sandbox_runner_output_terminal_visible` | 执行后有 stdout 时显示输出区域 | `data-testid="sandbox-output"` 可见 |
| FE-007 | `sandbox_runner_exit_code_display` | 显示 exit code 和 duration | 文本包含 "Exit code:" 和 "Duration:" |
| FE-008 | `sandbox_runner_error_display` | 执行出错时显示 error | `role="alert"` 元素可见 |
| FE-009 | `sandbox_runner_language_badge` | 每种语言显示正确的大写/缩写 badge | python→"Python", javascript→"JS", cpp→"C++" |
| FE-010 | `code_editor_renders_code` | `<CodeEditor code="..." language="python" />` 渲染 | CodeMirror 编辑器可见，内容为 code |

### 11.9 MarkdownRenderer Tests（Vitest）

**位置:** `apps/web-ui/src/components/content/__tests__/` （新增）

**测试目标:** 验证 MarkdownRenderer 正确从 HTML 中提取语言标识和代码内容。

| 测试ID | 测试名称 | 测试内容 | 预期结果 |
|--------|---------|---------|---------|
| MR-001 | `extract_language_from_data_language` | `<pre data-language="python"><code>...</code></pre>` | 语言识别为 "python" |
| MR-002 | `extract_language_from_code_class` | `<pre><code class="language-javascript">...</code></pre>` | 语言识别为 "javascript" |
| MR-003 | `extract_code_from_ec_lines` | rehype-expressive-code 渲染的多行代码 | 代码按行提取，用 `\n` 连接 |
| MR-004 | `extract_code_preserves_indentation` | Python 代码含 4 空格缩进 | 缩进完整保留 |
| MR-005 | `unsupported_language_no_portal` | ````haskell` 代码块 | 不创建 SandboxRunner portal |
| MR-006 | `no_language_no_portal` | 无语言标识的代码块 | 不创建 SandboxRunner portal |
| MR-007 | `empty_code_block_no_portal` | 空白的代码块 | 不创建 SandboxRunner portal |
| MR-008 | `all_supported_languages_detected` | 对所有 11 个支持语言创建 code fence | 每个语言的 SandboxRunner portal 被创建 |
| MR-009 | `language_alias_normalized` | ````py` → python, ````js` → javascript, ````rs` → rust | 别名正确归一化 |
| MR-010 | `bash_creates_portal` | ````bash` 代码块 | 创建 SandboxRunner portal（bash 是支持的语言） |

### 11.10 Frontend E2E Tests（Playwright）

**位置:** `apps/web-ui/tests/e2e/sandbox-runner.spec.ts` （扩展现有测试）

**测试目标:** 在真实浏览器中验证完整的用户交互流程。

#### 11.10.1 Run 按钮可见性测试（每个语言一个）

| 测试ID | 测试名称 | 测试内容 | 预期结果 |
|--------|---------|---------|---------|
| E2E-001 | `run_button_python` | 页面包含 ````python` 代码块 | Run 按钮可见，label="Run python code" |
| E2E-002 | `run_button_javascript` | 页面包含 ````javascript` 代码块 | Run 按钮可见，label="Run javascript code" |
| E2E-003 | `run_button_typescript` | 页面包含 ````typescript` 代码块 | Run 按钮可见，label="Run typescript code" |
| E2E-004 | `run_button_rust` | 页面包含 ````rust` 代码块 | Run 按钮可见，label="Run rust code" |
| E2E-005 | `run_button_go` | 页面包含 ````go` 代码块 | Run 按钮可见，label="Run go code" |
| E2E-006 | `run_button_c` | 页面包含 ````c` 代码块 | Run 按钮可见，label="Run c code" |
| E2E-007 | `run_button_cpp` | 页面包含 ````cpp` 代码块 | Run 按钮可见，label="Run cpp code" |
| E2E-008 | `run_button_java` | 页面包含 ````java` 代码块 | Run 按钮可见，label="Run java code" |
| E2E-009 | `run_button_ruby` | 页面包含 ````ruby` 代码块 | Run 按钮可见，label="Run ruby code" |
| E2E-010 | `run_button_bash` | 页面包含 ````bash` 代码块 | Run 按钮可见，label="Run bash code" |

#### 11.10.2 执行流程测试（每个语言一个）

| 测试ID | 测试名称 | 测试内容 | 预期结果 |
|--------|---------|---------|---------|
| E2E-101 | `execute_python_and_see_output` | 点击 Python Run → 等待输出 | stdout 显示，exit_code=0 |
| E2E-102 | `execute_javascript_and_see_output` | 点击 JavaScript Run → 等待输出 | stdout 显示 |
| E2E-103 | `execute_typescript_and_see_output` | 点击 TypeScript Run → 等待输出 | stdout 显示 |
| E2E-104 | `execute_rust_and_see_output` | 点击 Rust Run → 等待输出 | stdout 显示 |
| E2E-105 | `execute_go_and_see_output` | 点击 Go Run → 等待输出 | stdout 显示 |
| E2E-106 | `execute_c_and_see_output` | 点击 C Run → 等待输出 | stdout 显示 |
| E2E-107 | `execute_cpp_and_see_output` | 点击 C++ Run → 等待输出 | stdout 显示 |
| E2E-108 | `execute_java_and_see_output` | 点击 Java Run → 等待输出 | stdout 显示 |
| E2E-109 | `execute_ruby_and_see_output` | 点击 Ruby Run → 等待输出 | stdout 显示 |
| E2E-110 | `execute_bash_and_see_output` | 点击 Bash Run → 等待输出 | stdout 显示 |

#### 11.10.3 通用 E2E 测试

| 测试ID | 测试名称 | 测试内容 | 预期结果 |
|--------|---------|---------|---------|
| E2E-201 | `non_runnable_no_run_button` | ````bash` (仅用于展示的 shell 命令) 和 ````text` 代码块 | 无 Run 按钮 |
| E2E-202 | `edit_code_and_run` | 编辑 CodeMirror 中的代码 → 点击 Run | 执行编辑后的代码 |
| E2E-203 | `reset_restores_original` | 编辑后点击 Reset | 代码恢复原始内容 |
| E2E-204 | `multiple_sandboxes_on_page` | 页面有 Python 和 JavaScript 两个代码块 | 两个 Run 按钮独立工作 |
| E2E-205 | `error_displayed_in_ui` | 执行包含语法错误的代码 | UI 显示 error 信息 |
| E2E-206 | `language_badge_display` | 检查每个语言的 badge 显示 | badge 文本正确（Python, JS, TS, Rust 等） |

### 11.11 Schema Validation Tests

**位置:** `tests/src/schema/` 目录下（扩展现有 schema 测试）

**测试目标:** 验证 `sandbox_request` 和 `sandbox_result` JSON Schema 覆盖所有语言。

| 测试ID | 测试名称 | 测试内容 | 预期结果 |
|--------|---------|---------|---------|
| SCH-001 | `validate_sandbox_request_all_toolkinds` | 对每个 `tool_kind` 值构造合法 JSON，验证通过 | 全部通过 schema 验证 |
| SCH-002 | `validate_sandbox_request_invalid_toolkind` | `tool_kind: "haskell_exec"` | 验证失败 |
| SCH-003 | `validate_sandbox_request_missing_code` | 无 `code` 字段 | 验证失败 |
| SCH-004 | `validate_sandbox_request_missing_request_id` | 无 `request_id` 字段 | 验证失败 |
| SCH-005 | `validate_sandbox_result_all_statuses` | 对每个 `status` 值构造合法 JSON | 全部通过验证 |
| SCH-006 | `validate_sandbox_result_invalid_status` | `status: "unknown_status"` | 验证失败 |
| SCH-007 | `validate_sandbox_result_optional_fields` | `session_id` 和 `error` 可选 | 缺省时仍通过验证 |
| SCH-008 | `schema_syntax_valid` | `sandbox_request.v1.schema.json` 和 `sandbox_result.v1.schema.json` | 本身是合法的 JSON Schema |

### 11.12 Prompt Contract Tests（Rust / Mock LLM）

**位置:** `crates/agent-core/tests/prompt_contract_tests.rs`（新增）

**测试目标:** 验证 prompt 修改后 LLM 输出的代码块使用正确的语言标识，且符合 sandbox 约束。

| 测试ID | 测试名称 | 测试内容 | 预期结果 |
|--------|---------|---------|---------|
| PR-001 | `chapter_teaching_uses_supported_language_fence` | 模拟 Python 课程 chapter_teaching 输出 | 代码块使用 `python` 语言标识 |
| PR-002 | `chapter_teaching_rust_uses_rust_fence` | 模拟 Rust 课程输出 | 代码块使用 `rust` 语言标识 |
| PR-003 | `chapter_teaching_no_unsupported_language` | 模拟输出包含 ` ```haskell ` | 验证/修复后不应包含不支持的语言标识 |
| PR-004 | `chapter_teaching_code_is_runnable` | 模拟输出的 Python 代码 | 代码无 `input()`、`requests`、文件路径 |
| PR-005 | `chapter_teaching_no_network_imports` | 模拟输出的代码包含 `import requests` | 检测并标记为不安全 |
| PR-006 | `chapter_teaching_no_interactive_input` | 模拟输出的代码包含 `input()` | 检测并标记为不安全 |
| PR-007 | `chapter_teaching_at_least_two_runnable_blocks` | 编程主题的章节 | 至少 2 个可运行代码块 |
| PR-008 | `chapter_teaching_output_in_text_fence` | 预期输出使用 ` ```text ` 而非 ` ```python ` | 输出代码块使用 `text` 标识 |
| PR-009 | `question_answering_code_example_runnable` | Q&A 中包含代码示例 | 代码使用支持的语言标识 |
| PR-010 | `question_answering_mentions_run_button` | Q&A 回答涉及代码 | 提示学习者可点击 Run 按钮 |
| PR-011 | `markdown_repair_fixes_language_identifier` | 输入 ` ``` ` (无标识) 的 Python 代码 | 修复后添加 `python` 标识 |
| PR-012 | `markdown_repair_keeps_correct_identifier` | 输入正确 ` ```rust ` | 保持 `rust` 标识不变 |
| PR-013 | `markdown_repair_removes_clipboard_artifacts` | 输入包含 `[Pasted ~2 lines]` | 修复后移除 artifacts |
| PR-014 | `output_format_guide_respected` | 验证输出符合 output_format_guide 约束 | 所有格式规则满足 |
| PR-015 | `sandbox_language_guide_available` | 确认 prompt 模板渲染后包含 `{{sandbox_language_guide}}` | 渲染后 HTML 包含完整的语言表 |

### 11.13 Regression Tests

**位置:** 现有测试文件中

**测试目标:** 确保新改动不破坏现有的 Python、JavaScript、Typst sandbox 功能。

| 测试ID | 测试名称 | 测试内容 | 预期结果 |
|--------|---------|---------|---------|
| REG-001 | `existing_python_sandbox_unchanged` | 所有现有 Python sandbox 测试继续通过 | 零失败 |
| REG-002 | `existing_javascript_sandbox_unchanged` | 所有现有 JS sandbox 测试继续通过 | 零失败 |
| REG-003 | `existing_typst_sandbox_unchanged` | 所有现有 Typst sandbox 测试继续通过 | 零失败 |
| REG-004 | `existing_e2e_tests_unchanged` | 所有现有 Playwright E2E 测试继续通过 | 零失败 |
| REG-005 | `existing_integration_tests_unchanged` | `tests/src/integration/` 下现有测试全部通过 | 零失败 |
| REG-006 | `existing_unit_tests_unchanged` | 所有 `#[test]` 单元测试全部通过 | `cargo test` 零失败 |
| REG-007 | `existing_prompt_tests_unchanged` | 现有 prompt 合约测试继续通过 | 零失败 |

### 11.14 Security Tests

**位置:** `sandboxes/tests/security/` （新增）

**测试目标:** 验证 sandbox 安全隔离有效，恶意输入无法逃逸。

| 测试ID | 测试名称 | 测试内容 | 预期结果 |
|--------|---------|---------|---------|
| SEC-001 | `no_network_egress` | 所有语言尝试建立网络连接 | 全部失败/超时 |
| SEC-002 | `no_host_filesystem_access` | 尝试读取 `/etc/passwd` 或宿主路径 | 失败 |
| SEC-003 | `no_privilege_escalation` | 尝试 `sudo`、`su`、`setuid` | 失败（命令不存在或无权限） |
| SEC-004 | `no_new_processes_spawn` | fork bomb `:(){ :|:& };:` | 被 pids-limit 阻止 |
| SEC-005 | `resource_exhaustion_memory` | 分配超大数组（> memory_mb） | 被 OOM killer 终止 |
| SEC-006 | `resource_exhaustion_disk` | 写大量文件填满 /workspace tmpfs | 写操作失败，磁盘限制生效 |
| SEC-007 | `code_injection_via_stdin` | 编译型语言的代码包含 shell 元字符 `$(rm -rf /)` | 不执行注入命令（stdin pipe 不经过 shell） |
| SEC-008 | `code_injection_interpreted` | 解释型语言的代码包含反引号或 `$()` | 在沙箱隔离内执行，不影响宿主 |
| SEC-009 | `docker_socket_access` | 尝试访问 `/var/run/docker.sock` | 不存在（未挂载） |
| SEC-010 | `procfs_sysfs_not_leaked` | 尝试读取 `/proc/self/environ` 或 `/sys/...` | 读不到敏感信息或路径不存在 |
| SEC-011 | `seccomp_profile_enforced` | 尝试调用被 seccomp 禁止的系统调用 | 进程被终止 |
| SEC-012 | `long_output_truncation` | 输出 200KB 的文本 | stdout 被截断到 64KB，`stdout_truncated=true` |

### 11.15 测试数量汇总

| 测试类别 | 测试项数量 | 位置 |
|---------|-----------|------|
| Registry & Code Generation | 13 | `crates/sandbox-manager/tests/` |
| Docker Image Build | 13 | `sandboxes/tests/` |
| Docker Image Contract (通用) | 13 × 11 语言 = 可参数化 | `sandboxes/tests/contract/` |
| Docker Image Contract (语言专属) | Python 4 + JS 3 + TS 3 + Rust 6 + Go 4 + C 3 + C++ 3 + Java 3 + Ruby 3 + Bash 4 = 36 | `sandboxes/tests/contract/` |
| Sandbox Manager Unit | 10 | `crates/sandbox-manager/src/` |
| Container Executor | 14 | `crates/sandbox-manager/tests/` |
| API Integration | 19 | `tests/src/integration/` |
| SSE Stream Format | 5 | `tests/src/integration/` |
| Frontend Component (Vitest) | 10 | `apps/web-ui/src/components/` |
| MarkdownRenderer (Vitest) | 10 | `apps/web-ui/src/components/` |
| Frontend E2E (Playwright) | 26 | `apps/web-ui/tests/e2e/` |
| Schema Validation | 8 | `tests/src/schema/` |
| Prompt Contract | 15 | `crates/agent-core/tests/` |
| Regression | 7 | 现有测试文件 |
| Security | 12 | `sandboxes/tests/security/` |
| **总计** | **约 240+** | |

> 注：通用合约测试 13 项 × 11 语言 = 143，但应通过参数化测试实现，避免代码重复。实际测试函数数量约 130-150 个，覆盖 240+ 个测试场景。

---

## 11. Risks and Mitigations

| Risk | Impact | Mitigation |
|------|--------|-----------|
| Registry → codegen adds build complexity | Medium | Make codegen optional; fall back to committed generated files. CI checks that generated files match registry. |
| Stdin pipe fails for large code | Low | Sandbox limits already cap execution; code over 64KB is unlikely for teaching examples. |
| Compiler version drift (rustc, gcc updates) | Low | Pin base image digests in Dockerfiles. Definition YAML references pinned versions. |
| LLM generates code with external packages | Medium | Prompt explicitly forbids this. Add regex validation in agent-core to reject known disallowed imports before sandbox execution. |
| LLM generates code with `input()` / `scanf()` | Medium | Prompt forbids interactive input. Sandbox runner treats stdin read on a closed pipe as immediate EOF, causing error — not a hang. |
| Docker image build time explosion with many languages | Low | Images are built once, cached, and versioned. CI only rebuilds changed images. |
| Memory/disk usage of many Docker images on dev machines | Low | Tier 2+ images are optional. `docker compose` profile to select which sandboxes to pull. |

---

## 12. Open Questions

1. **SQL sandbox?** SQL is not a programming language per se — it needs a database engine. Consider a separate `sandbox-sqlite` image that accepts SQL queries and returns result sets, rather than fitting into the code-execution model.

2. **Package allowlists?** Should some languages allow a curated set of additional packages? E.g., Python with `numpy`, Node with `lodash`? This significantly increases image size and maintenance burden. Recommendation: start with standard library only, add packages based on actual teaching demand.

3. **Multi-file projects?** Currently single-file only. For advanced learners (especially Go/Java), multi-file support would be valuable. This requires the runner script to accept a tarball or multiple stdin chunks. Defer to a later phase.

4. **Interactive REPL?** Some teaching scenarios benefit from a REPL (Python, Node, Ruby). This requires WebSocket for persistent container sessions. Not in scope for this plan.

5. **WASM sandbox?** Running code in the browser via WASM (e.g., Python via Pyodide) could eliminate the Docker dependency for simple cases. Evaluate as a lightweight alternative for Tier 3.

---

## 13. Summary of Changes by File

| File | Change type | Description |
|------|------------|-------------|
| `sandboxes/definitions/registry.yaml` | **NEW** | Single source of truth for all languages |
| `tools/generate-sandbox-registry` | **NEW** | Code generator script |
| `crates/sandbox-manager/src/generated.rs` | **NEW** | Auto-generated Rust types |
| `crates/sandbox-manager/src/models/request.rs` | MODIFY | Remove hand-written ToolKind, import generated |
| `crates/sandbox-manager/src/docker/container.rs` | MODIFY | Use execution_model; add stdin pipe for compiled langs |
| `crates/agent-core/src/server/handlers/sandbox.rs` | MODIFY | Use generated `ToolKind::from_language()` |
| `schemas/sandbox_request.v1.schema.json` | MODIFY | Auto-generated enums (or relax language to string) |
| `apps/web-ui/src/api/generated-sandbox.ts` | **NEW** | Auto-generated TS types |
| `apps/web-ui/src/api/client.ts` | MODIFY | Import generated types |
| `apps/web-ui/src/components/sandbox/SandboxRunner.tsx` | MODIFY | Import generated SUPPORTED_LANGUAGES |
| `apps/web-ui/src/components/content/MarkdownRenderer.tsx` | MODIFY | Import generated SUPPORTED_LANGUAGES |
| `sandboxes/docker/Dockerfile.rust` | **NEW** | Rust sandbox image |
| `sandboxes/docker/Dockerfile.typescript` | **NEW** | TypeScript sandbox image |
| `sandboxes/docker/Dockerfile.go` | **NEW** | Go sandbox image |
| `sandboxes/docker/Dockerfile.bash` | **NEW** | Bash sandbox image |
| `sandboxes/docker/Dockerfile.c` | **NEW** | C sandbox image |
| `sandboxes/docker/Dockerfile.cpp` | **NEW** | C++ sandbox image |
| `sandboxes/docker/Dockerfile.java` | **NEW** | Java sandbox image |
| `sandboxes/docker/Dockerfile.ruby` | **NEW** | Ruby sandbox image |
| `prompts/shared/sandbox_language_guide.partial.md` | **NEW** | Prompt partial for sandbox language rules |
| `prompts/chapter_teaching.v1.prompt.md` | MODIFY | Inject sandbox guide; update constraints |
| `prompts/question_answering.v1.prompt.md` | MODIFY | Inject sandbox guide; add code Q&A rules |
| `prompts/chapter_markdown_repair.v1.prompt.md` | MODIFY | Add language identifier repair rules |
| `prompts/shared/output_format_guide.partial.md` | MODIFY | Expand code block language list |
| `apps/web-ui/tests/e2e/sandbox-runner.spec.ts` | MODIFY | Add per-language test cases |
