# Tools Module — Phase 0: Bootstrap, Check, and CI Scripts

## Module Overview

`tools/` contains developer utilities, validation scripts, and operational tooling. Phase 0 establishes the repository foundation: bootstrap, check, and CI-quality scripts that every contributor and CI pipeline uses.

**Core principle:** Every tool declares its inputs, outputs, side effects, and security limits. Tools make development workflows repeatable and auditable.

## Phase 0 Scope

| Deliverable | Description | Status |
|-------------|-------------|--------|
| `scripts/bootstrap` | Verify required development tooling (Rust, Node, package manager) | Planned |
| `scripts/check` | Run all repository validators: formatting, linting, type checks, schema validation, tests | Planned |
| `scripts/schema-check` | Validate all JSON Schema files and fixtures | Planned |
| Logging policy | Structured logging with required fields and redaction rules | Defined in root AGENTS.md |
| CI plan | Fail on formatting, lint, tests, schema errors, accidental secrets | Planned |

## File Structure

```
tools/
├── AGENTS.md
├── plan_phase0.md
├── plan_phase1.md
├── plan_phase2.md
├── plan_phase2.5.md
├── plan_phase3.md
├── scripts/
│   ├── bootstrap              # POSIX sh (macOS + Linux), no bashisms
│   ├── dev                    # Start backend + frontend for development
│   ├── check                  # Run all validators
│   ├── schema-check           # Validate schemas and fixtures
│   └── lib/
│       ├── colors.sh          # Terminal color helpers
│       ├── logging.sh         # Structured log output helpers
│       └── checks.sh          # Shared check functions
└── ci/
    ├── github-actions.yml     # CI workflow definition
    └── dependabot.yml         # Dependency update configuration
```

## Scripts

### `scripts/bootstrap`

**Purpose:** Verify that all required development tools are installed with acceptable versions. Fail with clear messages when tools are missing or too old.

```bash
#!/bin/sh
# bootstrap — Verify development tooling for Blup
# Usage: ./scripts/bootstrap [--phase phase1|phase2|phase2.5|phase3]
# Default phase: phase1
#
# Exit 0 if all tools are available, 1 otherwise.

set -e

PHASE="${1:-phase1}"
ERRORS=0

check_cmd() {
    cmd="$1"
    name="${2:-$cmd}"
    get_version="${3:-$cmd --version 2>&1}"
    min_version="${4:-}"

    if ! command -v "$cmd" >/dev/null 2>&1; then
        echo "ERROR: $name is not installed. Please install $name."
        ERRORS=$((ERRORS + 1))
        return
    fi

    version=$($get_version | head -1)
    echo "  OK: $name — $version"

    if [ -n "$min_version" ]; then
        # Version comparison would go here
        true
    fi
}

echo "=== Blup Bootstrap ==="
echo "Checking tools for: $PHASE"
echo ""

# Always required
check_cmd "cargo"     "Rust/Cargo"   "cargo --version"      "1.80.0"
check_cmd "rustc"     "Rust Compiler" "rustc --version"     "1.80.0"
check_cmd "python3"   "Python"       "python3 --version"    "3.12.0"
check_cmd "node"      "Node.js"      "node --version"       "20.0.0"
check_cmd "npm"       "npm"          "npm --version"        "10.0.0"
# Python packages for LLM Gateway
check_cmd "pip3"      "pip"          "pip3 --version"

# Phase-specific checks
case "$PHASE" in
    phase2|phase2.5|phase3)
        check_cmd "docker" "Docker" "docker --version" "24.0.0"
        ;;
    phase2.5)
        check_cmd "typst" "Typst" "typst --version" "0.11.0"
        ;;
esac

echo ""
if [ "$ERRORS" -gt 0 ]; then
    echo "$ERRORS tool(s) missing or outdated. Please install them and retry."
    exit 1
else
    echo "All required tools are available."
fi
```

### `scripts/dev`

**Purpose:** Start the Phase 1 backend and frontend for local development.

```bash
#!/bin/sh
# dev — Start Blup development environment
# Usage: ./scripts/dev
#
# Starts agent-core on localhost:3000 and web-ui on localhost:5173.
# Uses cargo watch and vite dev server for hot reload.

set -e

echo "=== Blup Dev Server ==="

# Trap to kill background processes on exit
cleanup() {
    echo "Shutting down..."
    kill $GATEWAY_PID $BACKEND_PID $FRONTEND_PID 2>/dev/null || true
    wait $GATEWAY_PID $BACKEND_PID $FRONTEND_PID 2>/dev/null || true
}
trap cleanup EXIT INT TERM

# Start Python LLM Gateway
echo "Starting Python LLM Gateway..."
cd services/llm-gateway
python3 -m uvicorn src.main:app --host 127.0.0.1 --port 9000 &
GATEWAY_PID=$!
cd ../..

# Wait for gateway to be ready
echo "Waiting for LLM Gateway..."
for i in $(seq 1 15); do
    if curl -s http://localhost:9000/health >/dev/null 2>&1; then
        echo "LLM Gateway ready."
        break
    fi
    sleep 1
done

# Start backend with hot reload
echo "Starting agent-core..."
cd crates/agent-core
cargo watch -x run &
BACKEND_PID=$!
cd ../..

# Wait for backend to be ready
echo "Waiting for backend..."
for i in $(seq 1 30); do
    if curl -s http://localhost:3000/health >/dev/null 2>&1; then
        echo "Backend ready."
        break
    fi
    sleep 1
done

# Start frontend
echo "Starting web-ui..."
cd apps/web-ui
npm run dev &
FRONTEND_PID=$!
cd ../..

echo ""
echo "Backend:  http://localhost:3000"
echo "Frontend: http://localhost:5173"
echo "Press Ctrl+C to stop."
wait
```

### `scripts/check`

**Purpose:** Run all repository validators in order. Exit 0 only if all pass.

```bash
#!/bin/sh
# check — Run all repository validators
# Usage: ./scripts/check [--fix] [--skip-tests]
#
# Order: format → lint → typecheck → schema-check → tests
# Fails fast on first error.

set -e

FIX=false
SKIP_TESTS=false

for arg in "$@"; do
    case "$arg" in
        --fix) FIX=true ;;
        --skip-tests) SKIP_TESTS=true ;;
    esac
done

ERRORS=0

run_check() {
    name="$1"
    cmd="$2"
    echo "=== $name ==="
    if eval "$cmd"; then
        echo "  PASS: $name"
    else
        echo "  FAIL: $name"
        ERRORS=$((ERRORS + 1))
    fi
    echo ""
}

echo "Blup Check — $(date)"
echo ""

# 1. Rust formatting
if $FIX; then
    run_check "cargo fmt" "cargo fmt"
else
    run_check "cargo fmt --check" "cargo fmt --check"
fi

# 2. Rust linting
run_check "cargo clippy" "cargo clippy --all-targets --all-features -- -D warnings"

# 3. Rust tests
if ! $SKIP_TESTS; then
    run_check "cargo test" "cargo test"
fi

# 4. Schema validation
run_check "schema-check" "./scripts/schema-check"

# 5. Web UI checks (if web-ui exists)
if [ -f "apps/web-ui/package.json" ]; then
    cd apps/web-ui
    run_check "npm run typecheck" "npm run typecheck"
    run_check "npm run lint" "npm run lint"
    if ! $SKIP_TESTS; then
        run_check "npm test" "npm test"
    fi
    cd ../..
fi

# 6. Secret scanning (if gitleaks or detect-secrets is available)
if command -v gitleaks >/dev/null 2>&1; then
    run_check "gitleaks detect" "gitleaks detect --no-git"
fi

echo "=== Summary ==="
if [ "$ERRORS" -gt 0 ]; then
    echo "$ERRORS check(s) failed."
    exit 1
else
    echo "All checks passed."
fi
```

### `scripts/schema-check`

**Purpose:** Validate JSON Schema syntax and fixture validation.

```bash
#!/bin/sh
# schema-check — Validate all JSON Schema files and fixtures
# Usage: ./scripts/schema-check [--schemas-only] [--fixtures-only]

set -e

SCHEMAS_DIR="./schemas"
FIXTURES_DIR="./schemas/fixtures"
ERRORS=0

echo "=== Schema Validation ==="

# Phase 1: validate schema syntax if schema-validator tool doesn't exist yet
# Use a simple approach: check JSON parse + validate against meta-schema

for schema_file in "$SCHEMAS_DIR"/*.schema.json; do
    if [ ! -f "$schema_file" ]; then
        echo "  SKIP: No schema files found"
        break
    fi

    basename=$(basename "$schema_file")

    # Validate JSON syntax
    if ! python3 -m json.tool "$schema_file" >/dev/null 2>&1; then
        echo "  FAIL: $basename — invalid JSON syntax"
        ERRORS=$((ERRORS + 1))
        continue
    fi

    # Check required fields
    if ! python3 -c "
import json
s = json.load(open('$schema_file'))
assert '\$schema' in s, 'Missing \$schema'
assert '\$id' in s, 'Missing \$id'
assert 'version' in s, 'Missing version field'
" 2>&1; then
        echo "  FAIL: $basename — missing required fields"
        ERRORS=$((ERRORS + 1))
        continue
    fi

    echo "  OK: $basename"
done

# Validate fixtures against their schemas
if [ -d "$FIXTURES_DIR" ]; then
    echo ""
    echo "--- Fixture Validation ---"

    for fixture_dir in "$FIXTURES_DIR"/*/; do
        schema_name=$(basename "$fixture_dir")
        schema_file="$SCHEMAS_DIR/${schema_name}.v1.schema.json"

        if [ ! -f "$schema_file" ]; then
            echo "  WARN: No schema for fixture directory '$schema_name'"
            continue
        fi

        # Check valid fixtures pass
        for valid_file in "$fixture_dir"/valid-*.json; do
            if [ ! -f "$valid_file" ]; then
                continue
            fi
            # Note: full JSON Schema validation requires jsonschema library
            # For Phase 0, we validate JSON syntax and basic structure
            if python3 -m json.tool "$valid_file" >/dev/null 2>&1; then
                echo "  OK: $schema_name/$(basename $valid_file)"
            else
                echo "  FAIL: $schema_name/$(basename $valid_file) — invalid JSON"
                ERRORS=$((ERRORS + 1))
            fi
        done

        # Check invalid fixtures are syntactically valid JSON (but should fail schema)
        for invalid_file in "$fixture_dir"/invalid-*.json; do
            if [ ! -f "$invalid_file" ]; then
                continue
            fi
            if python3 -m json.tool "$invalid_file" >/dev/null 2>&1; then
                echo "  OK: $schema_name/$(basename $invalid_file)"
            else
                echo "  FAIL: $schema_name/$(basename $invalid_file) — invalid JSON"
                ERRORS=$((ERRORS + 1))
            fi
        done
    done
fi

echo ""
if [ "$ERRORS" -gt 0 ]; then
    echo "$ERRORS validation error(s) found."
    exit 1
else
    echo "All schemas valid."
fi
```

## CI Configuration

### GitHub Actions Workflow

```yaml
# ci/github-actions.yml
name: CI

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

env:
  CARGO_TERM_COLOR: always
  RUSTFLAGS: "-D warnings"

jobs:
  rust-checks:
    name: Rust Checks
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt, clippy

      - name: Cache cargo
        uses: actions/cache@v4
        with:
          path: |
            ~/.cargo/registry/
            ~/.cargo/git/
            target/
          key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}

      - name: Format check
        run: cargo fmt --check

      - name: Clippy
        run: cargo clippy --all-targets --all-features -- -D warnings

      - name: Test
        run: cargo test

  schema-check:
    name: Schema Check
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Validate schemas
        run: ./scripts/schema-check

  web-checks:
    name: Web Checks
    runs-on: ubuntu-latest
    if: hashFiles('apps/web-ui/package.json') != ''
    steps:
      - uses: actions/checkout@v4

      - name: Setup Node.js
        uses: actions/setup-node@v4
        with:
          node-version: '22'
          cache: 'npm'
          cache-dependency-path: apps/web-ui/package-lock.json

      - name: Install dependencies
        working-directory: apps/web-ui
        run: npm ci

      - name: Type check
        working-directory: apps/web-ui
        run: npm run typecheck

      - name: Lint
        working-directory: apps/web-ui
        run: npm run lint

      - name: Test
        working-directory: apps/web-ui
        run: npm test

  secret-scan:
    name: Secret Scan
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0

      - name: Gitleaks
        uses: gitleaks/gitleaks-action@v2
        with:
          config-path: .gitleaks.toml
```

## Shared Library (`scripts/lib/`)

### `scripts/lib/colors.sh`

```bash
# Terminal color helpers
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

pass() { echo "${GREEN}  PASS:${NC} $*"; }
fail() { echo "${RED}  FAIL:${NC} $*"; }
warn() { echo "${YELLOW}  WARN:${NC} $*"; }
info() { echo "${BLUE}  INFO:${NC} $*"; }
```

### `scripts/lib/logging.sh`

```bash
# Structured log output (matching tracing format for consistency)
log_info() {
    echo "{\"timestamp\":\"$(date -u +%Y-%m-%dT%H:%M:%SZ)\",\"level\":\"INFO\",\"message\":\"$*\"}"
}

log_error() {
    echo "{\"timestamp\":\"$(date -u +%Y-%m-%dT%H:%M:%SZ)\",\"level\":\"ERROR\",\"message\":\"$*\"}" >&2
}
```

## Security Rules for Scripts

| Rule | Why |
|------|-----|
| No hard-coded absolute paths | Scripts must work across developer machines and CI |
| No API keys, tokens, or credentials | Scripts are committed; secrets go in environment variables |
| No destructive file operations by default | `rm -rf` needs explicit `--force` flag and warning |
| No `curl | bash` patterns | All dependencies installed through package managers |
| POSIX sh compatibility | Scripts should work on macOS (BSD utils) and Linux (GNU utils) |
| Explicit error handling | `set -e` with meaningful error messages |

## Quality Gates

- [ ] `scripts/bootstrap` exits 0 when all required tools are installed
- [ ] `scripts/bootstrap` exits 1 with clear message when a tool is missing
- [ ] `scripts/check` runs all validators and exits with correct status
- [ ] `scripts/schema-check` validates all schema files
- [ ] CI workflow matches the check script (same checks, same order)
- [ ] CI fails on formatting, lint, type errors, schema errors, test failures
- [ ] Secret scanning detects committed API keys or tokens
- [ ] No hard-coded paths, credentials, or machine-specific configuration in any script
- [ ] All scripts have a usage comment and `--help` flag

## Developer Onboarding

### First-Time Setup

```bash
# 1. Clone the repository
git clone https://github.com/blup-project/blup.git
cd blup

# 2. Run bootstrap to verify tooling
./scripts/bootstrap
# Expected output: all tools OK

# 3. Install Python dependencies for LLM Gateway
cd services/llm-gateway
python3 -m venv .venv
source .venv/bin/activate
pip install -r requirements.txt
cd ../..

# 4. Install Node dependencies for Web UI
cd apps/web-ui
npm install
cd ../..

# 5. Set up environment variables
cp .env.example .env
# Edit .env with your API keys:
#   OPENAI_API_KEY=sk-...
#   ANTHROPIC_API_KEY=sk-ant-...
#   GATEWAY_SECRET=your-secret-here

# 6. Run the dev server (starts gateway + backend + frontend)
./scripts/dev

# 7. Open http://localhost:5173 in your browser
```

### Common Development Workflows

```bash
# ── Running all checks before commit ──
./scripts/check

# ── Running only specific checks ──
./scripts/check --skip-tests         # Skip slow tests
cargo test -p agent-core             # Run only agent-core tests
npm run typecheck                    # TypeScript type check only (from apps/web-ui)
./scripts/schema-check               # Schema validation only

# ── Working on prompts ──
# 1. Edit prompt file in prompts/
# 2. Test with prompt-tester (mock mode)
cd tools/prompt-tester
python -m src.main test feasibility_check --verbose

# 3. Capture updated fixtures (requires LLM Gateway running)
python -m src.main capture feasibility_check --update-fixtures

# ── Working on schemas ──
# 1. Edit schema file in schemas/
# 2. Validate
./scripts/schema-check

# 3. Add/update fixtures in schemas/fixtures/{schema_name}/
# 4. Run full schema validation
cargo run -p schema-validator -- validate --all

# ── Working on agent-core ──
# 1. Make changes in crates/agent-core/
# 2. Auto-reload via cargo watch (already running in ./scripts/dev)
# 3. Run specific tests
cargo test -p agent-core -- state::machine::test_valid_transitions

# ── Working on Web UI ──
# 1. Make changes in apps/web-ui/
# 2. Vite HMR auto-reloads in browser
# 3. Run UI tests
cd apps/web-ui && npm test

# ── Debugging SSE streams ──
# Use curl to inspect SSE events directly:
curl -N -X POST http://localhost:3000/api/session \
  -H "Content-Type: application/json"

# ── Debugging LLM Gateway ──
# Check gateway health and provider status
curl http://localhost:9000/health
curl http://localhost:9000/health/providers

# ── Running integration tests ──
# These start agent-core with mock gateway automatically:
cargo test -p blup-tests -- integration::
```

### Project Conventions

```text
Code Style:
  Rust:   Follow standard Rust conventions. cargo fmt handles formatting.
  Python: Follow PEP 8. Use `ruff` for linting and formatting.
  TypeScript: ESLint + Prettier configured in apps/web-ui/.
  Shell:   POSIX sh. No bashisms. Check with shellcheck.

Commit Messages:
  Format:  <area>: <imperative description>
  Examples:
    agent-core: add state machine transition validation
    schemas: define LearningGoal schema v1
    prompts: add feasibility check template with examples
    web-ui: implement SSE reconnection with backoff
    tools: add schema-validator CLI

Branching:
  main       — stable, deployable
  feat/xxx   — feature branches
  fix/xxx    — bug fixes
  docs/xxx   — documentation changes

Pull Requests:
  - Must pass CI (./scripts/check)
  - Must include tests for new functionality
  - Must not include API keys or .env changes
  - Prompt changes require contract test updates
  - Schema changes require fixture updates
```

### Troubleshooting

| Problem | Diagnostic | Solution |
|---------|-----------|----------|
| Gateway won't start | `curl localhost:9000/health` | Check Python venv is activated; check API keys in `.env` |
| Agent-core can't reach gateway | Agent-core logs: "Gateway unhealthy" | Verify `BLUP_LLM_GATEWAY_URL=http://127.0.0.1:9000` |
| LLM calls time out | Gateway logs show API errors | Check API key validity; check network; check rate limits |
| Schema validation fails in CI | `./scripts/schema-check` | Run locally first; check for stray non-JSON files in schemas/ |
| SSE stream disconnects | Browser console: EventSource errors | Check agent-core logs for SSE errors; increase timeout |
| Port conflicts | `lsof -i :3000` or `lsof -i :9000` | Kill conflicting process or change port in config |
| npm install fails | Node version warning | `node --version` must be >= 20; use `nvm use 20` |
| cargo build fails | Rust version warning | `rustc --version` must be >= 1.80; `rustup update` |

## Release Process

### Version Numbering

Blup follows [Semantic Versioning](https://semver.org/):

```
MAJOR.MINOR.PATCH

MAJOR — Breaking changes (schema major version bumps, API removal, state machine changes)
MINOR — New features (new endpoints, new prompt types, new tools)
PATCH — Bug fixes, performance improvements, dependency updates
```

**Phase 0 releases:** `0.0.x` (pre-release)
**Phase 1 releases:** `0.1.x` (MVP development), `0.2.0` (MVP stable)
**Phase 2 releases:** `0.3.x` (persistence + assessment)
**Phase 2.5 releases:** `0.4.x` (desktop + import/export)
**Phase 3 releases:** `0.5.x` → `1.0.0` (plugins + scenes)

### Git Tag Convention

```bash
# Tag format: v{MAJOR}.{MINOR}.{PATCH}
git tag -a v0.1.0 -m "Phase 1 MVP: web learning assistant"
git tag -a v0.1.1 -m "fix: SSE reconnection backoff"
git tag -a v0.2.0 -m "feat: add profile collection flow"
```

### Release Checklist

```markdown
## Pre-Release
- [ ] All CI checks pass (./scripts/check)
- [ ] All tests pass (cargo test, npm test, pytest)
- [ ] Schema validator passes (./scripts/schema-check)
- [ ] Prompt tester passes (prompt-tester test-all)
- [ ] No known security vulnerabilities (trivy scan, cargo audit)
- [ ] Changelog updated (CHANGELOG.md)
- [ ] Version bumped in Cargo.toml, package.json, pyproject.toml
- [ ] Breaking changes documented in CHANGELOG with migration guide

## Release
- [ ] Git tag created: v{version}
- [ ] Release notes published on GitHub
- [ ] Docker images built and pushed to registry
- [ ] Sandbox images rebuilt and pushed

## Post-Release
- [ ] Deploy to staging environment
- [ ] Smoke tests pass on staging
- [ ] Deploy to production
- [ ] Monitor for 24 hours (error rates, latency, sessions)
- [ ] Notify team in Slack/Email
```

### Changelog Format

```markdown
# Changelog

## [0.2.0] — 2025-07-15

### Added
- Profile collection flow with 3-5 adaptive Q&A rounds
- Curriculum planning with personalized chapter structure
- `POST /api/session/{id}/profile/answer` endpoint
- `GET /api/session/{id}/curriculum` endpoint

### Changed
- State machine: added PROFILE_COLLECTION and CURRICULUM_PLANNING states
- LLM Gateway: added fallback routing between providers

### Fixed
- SSE reconnection now uses exponential backoff with jitter
- Schema validation error messages now include field path

### Deprecated
- (none)

### Removed
- (none)

### Security
- Added PII detection in log redaction layer
```

### Breaking Change Migration Guide

When a release contains breaking changes, add a migration section:

```markdown
### Migration from v0.1.x to v0.2.0

#### Schema Changes
- `LearningGoal` v1 → v2: `current_level` is now required.
  Run migration: `schema-validator migrate --from learning_goal.v1 --to learning_goal.v2 --fixtures`
  Database: sessions with v1 goals get `current_level: "unknown"` on next read.

#### API Changes
- (none — all v0.1.x endpoints remain unchanged)

#### Configuration Changes
- `BLUP_LLM_API_KEY` → removed. Use `OPENAI_API_KEY` and `ANTHROPIC_API_KEY` in gateway config.
- Added `BLUP_LLM_GATEWAY_SECRET` (required).

#### Prompt Changes
- `feasibility_check.v1.prompt.md` updated with few-shot examples — regenerate fixtures:
  `prompt-tester capture feasibility_check --update-fixtures`
```

### CI Release Pipeline

```yaml
# .github/workflows/release.yml
name: Release

on:
  push:
    tags: ['v*']

jobs:
  build-and-push:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Build agent-core
        run: cargo build --release -p agent-core

      - name: Build Docker images
        run: |
          docker build -t blup/agent-core:${{ github.ref_name }} -f crates/agent-core/Dockerfile .
          docker build -t blup/llm-gateway:${{ github.ref_name }} -f services/llm-gateway/Dockerfile .
          docker build -t blup/web-ui:${{ github.ref_name }} -f apps/web-ui/Dockerfile .

      - name: Push to registry
        run: |
          docker push blup/agent-core:${{ github.ref_name }}
          docker push blup/llm-gateway:${{ github.ref_name }}
          docker push blup/web-ui:${{ github.ref_name }}

      - name: Create GitHub Release
        uses: softprops/action-gh-release@v2
        with:
          body_path: CHANGELOG.md
          generate_release_notes: true
```

## Risk Register

| Risk | Impact | Mitigation |
|------|--------|------------|
| Shell script portability (macOS vs Linux) | CI passes, dev machine fails | POSIX sh only; test on both platforms |
| Missing tool detection in bootstrap | Developer frustration | Check both command existence and minimum version |
| CI and local check diverge | CI green, local red (or vice versa) | `scripts/check` is the single source of truth; CI calls it |
| Schema check is too lenient (just JSON parse) | Invalid schemas pass | Phase 1 adds full jsonschema validation via schema-validator tool |
| Secret scanning false positives | CI blocked on innocent commits | `.gitleaks.toml` allowlist for test fixtures and known-safe patterns |
