# Tools Module — Phase 2: Prompt Tester and Sandbox Builder

## Module Overview

Phase 2 adds two tools: `prompt-tester` for testing prompt templates with mock LLM outputs (and optional capture-replay against the Python LLM Gateway), and `sandbox-builder` for building Docker sandbox images with pinned versions, supply-chain verification, and vulnerability scanning.

## Deliverables

| Tool | Language | Purpose | Status |
|------|----------|---------|--------|
| `tools/prompt-tester/` | Python | Test prompt templates against fixtures and schema contracts; capture-replay with real LLM Gateway | Planned |
| `tools/sandbox-builder/` | Rust | Build sandbox Docker images with pinned versions, digest verification, and security scanning | Planned |

## Tool: prompt-tester

### Why Python?

The prompt-tester is implemented in Python because:
- It needs to interact with the Python LLM Gateway for capture-replay mode.
- Prompt rendering uses the same variable substitution as the gateway.
- Schema validation can use the `jsonschema` Python package (same logic as production).
- Test fixture management is more ergonomic in Python.

### CLI Specification

```
prompt-tester 0.1.0
Test prompt templates against fixtures and schema contracts

USAGE:
    prompt-tester <COMMAND>

COMMANDS:
    test-all                    Test all prompts with their fixtures (mock mode)
    test <prompt-name>          Test a specific prompt
    test --gateway              Test using real LLM Gateway (capture-replay)
    capture <prompt-name>       Capture LLM Gateway response as new fixture
    list                        List all prompts and their test status
    gen-fixtures <prompt-name>  Generate fixture scaffolding for a prompt

FLAGS:
    --prompts-dir <path>        Path to prompts directory (default: ../prompts)
    --schemas-dir <path>        Path to schemas directory (default: ../schemas)
    --gateway-url <url>         LLM Gateway URL for capture mode (default: http://127.0.0.1:9000)
    --verbose                   Show rendered prompts and full responses
    --json                      Output results as JSON
    --update-fixtures           Update fixtures with new captures
```

### Two Testing Modes

#### Mode 1: Mock Testing (Default, Offline)

Uses pre-written JSON fixtures — no network, no API keys, no cost. Runs in CI.

```
prompt-tester test-all
```

#### Mode 2: Gateway Capture-Replay Testing

Sends prompts to the real Python LLM Gateway (which calls OpenAI/Anthropic), captures the responses, and validates them against schemas. Used to update fixtures when prompts change.

```
prompt-tester test-all --gateway           # Test with real LLM, fail if schema mismatch
prompt-tester capture feasibility_check   # Capture response → save as new fixture
prompt-tester test-all --update-fixtures   # Re-capture all fixtures
```

**Important:** Gateway mode requires the Python LLM Gateway to be running and configured with valid API keys. This mode costs money and is never run in CI. It is a development tool for updating fixtures after prompt changes.

### File Structure

```
tools/prompt-tester/
├── pyproject.toml
├── requirements.txt
├── src/
│   ├── __init__.py
│   ├── main.py                  # CLI entry point (click or argparse)
│   ├── tester.py                # Test runner: load fixtures, render, validate
│   ├── mock_llm.py              # Returns fixture responses (offline mode)
│   ├── gateway_client.py        # Calls Python LLM Gateway (capture mode)
│   ├── renderer.py              # Prompt template rendering (variable substitution)
│   ├── validator.py              # Schema validation using jsonschema
│   ├── reporter.py               # Output formatting (terminal, JSON, JUnit XML)
│   ├── fixture_manager.py        # Load, save, and manage test fixtures
│   └── config.py                 # Configuration from CLI args and env vars
├── fixtures/                      # Per-prompt test fixtures
│   ├── feasibility_check/
│   │   ├── feasible_goal.json
│   │   ├── infeasible_goal.json
│   │   ├── minimal_input.json
│   │   └── malformed_input.json
│   ├── profile_collection/
│   │   ├── round1.json
│   │   ├── round3.json
│   │   └── round5_complete.json
│   ├── curriculum_planning/
│   │   ├── beginner_profile.json
│   │   └── advanced_profile.json
│   ├── chapter_teaching/
│   │   ├── short_chapter.json
│   │   └── long_chapter.json
│   └── question_answering/
│       ├── factual_question.json
│       └── clarification_question.json
└── tests/
    ├── test_tester.py
    ├── test_renderer.py
    ├── test_validator.py
    └── test_fixture_manager.py
```

### requirements.txt

```
jsonschema>=4.20.0           # Schema validation (same library used by schemas/)
click>=8.1.0                 # CLI framework
httpx>=0.27.0                # Async HTTP for gateway calls
structlog>=24.0.0            # Structured logging
pyyaml>=6.0                  # YAML fixture support
```

### Fixture Format

Each fixture file is a JSON object with input variables, expected output, and metadata:

```json
{
  "fixture_id": "feasibility_check-feasible_goal",
  "description": "A well-scoped programming goal should be feasible",
  "prompt_name": "feasibility_check",
  "prompt_version": 1,
  "target_schema": "feasibility_result.v1.schema.json",
  "input": {
    "learning_goal": "I want to learn Python for data analysis",
    "domain": "programming",
    "context": "I work with Excel spreadsheets and want to automate analysis"
  },
  "expected_output": {
    "feasible": true,
    "reason": "Python data analysis is a well-scoped, commonly taught topic...",
    "suggestions": [],
    "estimated_duration": "6-8 weeks",
    "prerequisites": ["basic computer literacy", "familiarity with spreadsheets"]
  },
  "semantic_checks": [
    {"rule": "feasible_true_implies_empty_suggestions", "description": "If feasible is true, suggestions array should be empty"},
    {"rule": "estimated_duration_format", "description": "Duration should be in format like 'N-N weeks' or 'N months'"},
    {"rule": "reason_is_specific", "description": "Reason should mention the specific domain, not generic text"}
  ],
  "captured_from": null,
  "captured_at": null,
  "last_passed": null
}
```

### Core Implementation

#### Main CLI

```python
# main.py
import click
from src.tester import PromptTester
from src.reporter import ReportFormat, Reporter
from src.config import Config

@click.group()
def cli():
    """Blup prompt-tester — validate prompt templates against fixtures."""
    pass

@cli.command()
@click.option("--gateway", is_flag=True, help="Use real LLM Gateway instead of mocks")
@click.option("--prompts-dir", default="../prompts")
@click.option("--schemas-dir", default="../schemas")
@click.option("--gateway-url", default="http://127.0.0.1:9000")
@click.option("--verbose", is_flag=True)
@click.option("--json-output", is_flag=True, help="Output results as JSON")
def test_all(gateway, prompts_dir, schemas_dir, gateway_url, verbose, json_output):
    """Test all prompt templates against their fixtures."""
    config = Config(
        prompts_dir=prompts_dir,
        schemas_dir=schemas_dir,
        gateway_url=gateway_url,
        use_gateway=gateway,
        verbose=verbose,
    )
    tester = PromptTester(config)
    results = tester.test_all()

    reporter = Reporter(format=ReportFormat.JSON if json_output else ReportFormat.TERMINAL)
    reporter.print(results)

    if not results.all_passed:
        raise SystemExit(1)

@cli.command()
@click.argument("prompt_name")
@click.option("--update-fixtures", is_flag=True)
def capture(prompt_name, update_fixtures):
    """Capture LLM Gateway responses as fixtures for a prompt."""
    # Requires running LLM Gateway with valid API keys
    config = Config(use_gateway=True, prompts_dir="../prompts", schemas_dir="../schemas")
    tester = PromptTester(config)
    fixture = tester.capture(prompt_name)
    if update_fixtures:
        tester.save_fixture(prompt_name, fixture)
        click.echo(f"Fixture saved for {prompt_name}")
    else:
        click.echo(json.dumps(fixture, indent=2))
```

#### Test Runner

```python
# tester.py
import json
from pathlib import Path
from jsonschema import validate, ValidationError
from src.renderer import PromptRenderer
from src.mock_llm import MockLlm
from src.gateway_client import GatewayClient
from src.fixture_manager import FixtureManager

class PromptTester:
    def __init__(self, config):
        self.config = config
        self.renderer = PromptRenderer(config.prompts_dir)
        self.fixtures = FixtureManager(config.prompts_dir)
        self.validator = SchemaValidator(config.schemas_dir)
        self.mock_llm = MockLlm()
        self.gateway = GatewayClient(config.gateway_url) if config.use_gateway else None

    def test_all(self) -> TestResults:
        results = TestResults()
        for prompt_name in self.fixtures.list_prompts():
            prompt_results = self.test_prompt(prompt_name)
            results.add(prompt_name, prompt_results)
        return results

    def test_prompt(self, prompt_name: str) -> list[FixtureResult]:
        results = []
        fixtures = self.fixtures.load_all(prompt_name)
        prompt_template = self.renderer.load(prompt_name, version=1)

        for fixture in fixtures:
            result = FixtureResult(fixture_id=fixture.id)

            try:
                # 1. Render prompt with fixture input variables
                rendered = self.renderer.render(prompt_template, fixture.input)

                # 2. Get LLM response (mock or real gateway)
                if self.gateway:
                    response = self.gateway.complete(
                        model="gpt-4o",  # Configurable
                        messages=[
                            {"role": "system", "content": rendered},
                            {"role": "user", "content": json.dumps(fixture.input)},
                        ],
                    )
                else:
                    response = self.mock_llm.respond(fixture.expected_output)

                # 3. Validate response against target schema
                self.validator.validate(response, fixture.target_schema)
                result.schema_valid = True

                # 4. Run semantic checks
                semantic_errors = self.run_semantic_checks(response, fixture.semantic_checks)
                result.semantic_errors = semantic_errors
                result.semantic_valid = len(semantic_errors) == 0

                # 5. Compare with expected output (mock mode only)
                if not self.gateway:
                    diff = self.compare_outputs(response, fixture.expected_output)
                    result.diff = diff

                result.passed = result.schema_valid and result.semantic_valid

            except ValidationError as e:
                result.schema_valid = False
                result.schema_errors = [str(e)]
                result.passed = False

            except Exception as e:
                result.error = str(e)
                result.passed = False

            results.append(result)

        return results

    def run_semantic_checks(self, output: dict, checks: list[dict]) -> list[str]:
        """Run custom semantic validation rules."""
        errors = []
        for check in checks:
            rule = check["rule"]
            if rule == "feasible_true_implies_empty_suggestions":
                if output.get("feasible") and len(output.get("suggestions", [])) > 0:
                    errors.append(f"feasible=true but suggestions is non-empty: {output['suggestions']}")

            elif rule == "estimated_duration_format":
                import re
                duration = output.get("estimated_duration", "")
                if not re.match(r'\d+[-–]\d+\s*(weeks?|months?|hours?)', duration):
                    errors.append(f"Duration '{duration}' does not match expected format")

            elif rule == "reason_is_specific":
                reason = output.get("reason", "")
                if len(reason) < 20:
                    errors.append(f"Reason is too short: '{reason}'")

            # ... more rules
        return errors

    def capture(self, prompt_name: str) -> dict:
        """Capture a real LLM response for use as a fixture."""
        if not self.gateway:
            raise RuntimeError("Gateway mode required for capture")
        fixtures = self.fixtures.load_all(prompt_name)
        fixture = fixtures[0]
        rendered = self.renderer.render(
            self.renderer.load(prompt_name, version=1),
            fixture.input,
        )
        response = self.gateway.complete(
            model="gpt-4o",
            messages=[
                {"role": "system", "content": rendered},
                {"role": "user", "content": json.dumps(fixture.input)},
            ],
        )
        # Validate the captured response
        self.validator.validate(response, fixture.target_schema)
        return {
            **fixture.to_dict(),
            "expected_output": response,
            "captured_from": "gpt-4o",
            "captured_at": datetime.utcnow().isoformat(),
        }
```

#### Gateway Client (for Capture Mode)

```python
# gateway_client.py
import httpx
from dataclasses import dataclass

@dataclass
class GatewayClient:
    gateway_url: str
    secret: str = ""  # From env: BLUP_LLM_GATEWAY_SECRET

    async def complete(self, model: str, messages: list[dict], **kwargs) -> dict:
        """Send a completion request to the Python LLM Gateway."""
        async with httpx.AsyncClient(timeout=60.0) as client:
            response = await client.post(
                f"{self.gateway_url}/v1/gateway/complete",
                headers={"X-Gateway-Secret": self.secret},
                json={
                    "model": model,
                    "messages": messages,
                    "stream": False,
                    **kwargs,
                },
            )
            response.raise_for_status()
            return response.json()

    async def health_check(self) -> bool:
        try:
            async with httpx.AsyncClient(timeout=5.0) as client:
                r = await client.get(f"{self.gateway_url}/health")
                return r.status_code == 200
        except Exception:
            return False
```

#### Reporter

```python
# reporter.py
import json
from enum import Enum

class ReportFormat(Enum):
    TERMINAL = "terminal"
    JSON = "json"
    JUNIT = "junit"

class Reporter:
    def __init__(self, format: ReportFormat = ReportFormat.TERMINAL):
        self.format = format

    def print(self, results: "TestResults"):
        if self.format == ReportFormat.TERMINAL:
            self._print_terminal(results)
        elif self.format == ReportFormat.JSON:
            self._print_json(results)
        elif self.format == ReportFormat.JUNIT:
            self._print_junit(results)

    def _print_terminal(self, results):
        print("=== Prompt Test Results ===\n")
        for prompt_name, fixture_results in results.by_prompt.items():
            passed = sum(1 for r in fixture_results if r.passed)
            total = len(fixture_results)
            status = "✓" if passed == total else "✗"
            print(f"{status} {prompt_name}: {passed}/{total} passed")

            for r in fixture_results:
                if r.passed:
                    print(f"  ✓ {r.fixture_id}")
                else:
                    print(f"  ✗ {r.fixture_id}")
                    if r.schema_errors:
                        for err in r.schema_errors:
                            print(f"    Schema error: {err}")
                    if r.semantic_errors:
                        for err in r.semantic_errors:
                            print(f"    Semantic error: {err}")
                    if r.error:
                        print(f"    Error: {r.error}")
            print()

        total_passed = sum(1 for results in results.by_prompt.values() for r in results if r.passed)
        total_all = sum(len(r) for r in results.by_prompt.values())
        print(f"Total: {total_passed}/{total_all} passed")

    def _print_json(self, results):
        print(json.dumps(results.to_dict(), indent=2))
```

## Tool: sandbox-builder

### CLI Specification

```
sandbox-builder 0.1.0
Build sandbox Docker images reproducibly

USAGE:
    sandbox-builder <COMMAND>

COMMANDS:
    build <sandbox>       Build a specific sandbox image (python, node, rust, typst, math, all)
    scan <image>          Scan a built image for vulnerabilities (trivy)
    verify <image>        Verify image digest matches expected value
    list                  List all sandbox definitions with build status
    clean                 Remove built images

FLAGS:
    --tag <tag>           Image tag (default: latest)
    --no-cache            Build without Docker cache
    --push                Push to registry after build
    --registry <url>      Container registry URL
    --platform <arch>     Target platform: amd64, arm64, all (default: amd64)
    --output <path>       Output SBOM to path (Software Bill of Materials)
```

### Sandbox Definitions

Each sandbox is defined in a YAML file with pinned versions:

```yaml
# sandboxes/definitions/python.yaml
name: sandbox-python
description: Python 3.12 execution sandbox for code exercises
dockerfile: sandboxes/docker/Dockerfile.python

# Pinned base image with digest for supply-chain integrity
base_image:
  name: python:3.12-slim
  digest: sha256:abc123def456...  # Verified at build time

# Pinned package versions for reproducibility
pinned_packages:
  - sympy==1.12.1
  - numpy==1.26.4
  - scipy==1.12.0

# Build arguments
build_args:
  USER_ID: "1000"
  USER_NAME: sandbox

# Security hardening
security:
  seccomp_profile: sandboxes/policies/seccomp-python.json
  read_only_rootfs: true
  no_new_privileges: true
  cap_drop: ["ALL"]

# Execution limits (documentation; enforced at runtime)
limits:
  compile_timeout_secs: 30
  run_timeout_secs: 10
  memory_mb: 512
  cpu_count: 1
  disk_mb: 100
  max_processes: 10
  network_enabled: false

# Test cases for image verification
tests:
  - name: python_version
    command: ["python", "--version"]
    expected_output: "Python 3.12"
  - name: sympy_import
    command: ["python", "-c", "import sympy; print(sympy.__version__)"]
    expected_output: "1.12.1"
  - name: network_blocked
    command: ["python", "-c", "import urllib.request; urllib.request.urlopen('http://example.com', timeout=5)"]
    expected_exit_code: 1
```

### Build Process (Detailed)

```rust
// builder.rs (conceptual) — Sandbox build logic
use std::process::Command;
use sha2::{Sha256, Digest};

struct SandboxBuilder {
    docker_cmd: String,       // "docker" or "podman"
    registry: Option<String>,
    platform: String,         // "linux/amd64", "linux/arm64"
}

impl SandboxBuilder {
    /// Build a sandbox image from its definition.
    async fn build(&self, definition: &SandboxDef) -> Result<BuildResult, BuildError> {
        let image_tag = self.compute_image_tag(definition);

        // 1. Verify base image digest
        self.verify_base_image(&definition.base_image).await?;

        // 2. Check if Dockerfile has changed since last build (cache optimization)
        let dockerfile_hash = self.hash_file(&definition.dockerfile)?;

        // 3. Build the image
        let status = Command::new(&self.docker_cmd)
            .args(["build",
                "--file", &definition.dockerfile,
                "--tag", &image_tag,
                "--platform", &self.platform,
                "--build-arg", &format!("USER_ID={}", definition.build_args.user_id),
                "--no-cache",  // Always clean build for reproducibility
                ".",
            ])
            .status()?;

        if !status.success() {
            return Err(BuildError::DockerBuildFailed);
        }

        // 4. Extract built image digest
        let digest = self.get_image_digest(&image_tag)?;

        // 5. Run verification tests inside the image
        let test_results = self.run_verification_tests(&image_tag, &definition.tests).await?;

        // 6. Generate SBOM (Software Bill of Materials)
        let sbom = self.generate_sbom(&image_tag)?;

        // 7. Vulnerability scan
        let scan_result = self.scan_image(&image_tag).await?;

        Ok(BuildResult {
            image_tag,
            digest,
            dockerfile_hash,
            test_results,
            sbom,
            scan_result,
            built_at: chrono::Utc::now(),
        })
    }

    /// Verify base image digest matches expected value.
    async fn verify_base_image(&self, base: &BaseImage) -> Result<(), BuildError> {
        // Pull the base image by digest (not tag) to prevent tag mutation attacks
        let full_ref = format!("{}@{}", base.name, base.digest);

        let output = Command::new(&self.docker_cmd)
            .args(["pull", &full_ref])
            .output()?;

        if !output.status.success() {
            return Err(BuildError::BaseImagePullFailed {
                image: full_ref,
                error: String::from_utf8_lossy(&output.stderr).to_string(),
            });
        }

        // Verify the pulled image's digest matches
        let actual_digest = self.get_image_digest(&full_ref)?;
        if actual_digest != base.digest {
            return Err(BuildError::DigestMismatch {
                expected: base.digest.clone(),
                actual: actual_digest,
            });
        }

        tracing::info!(
            base_image = %base.name,
            digest = %base.digest,
            "Base image digest verified"
        );
        Ok(())
    }

    /// Run verification tests inside the built image.
    async fn run_verification_tests(
        &self,
        image_tag: &str,
        tests: &[ImageTest],
    ) -> Result<Vec<TestResult>, BuildError> {
        let mut results = Vec::new();

        for test in tests {
            let mut cmd = Command::new(&self.docker_cmd);
            cmd.args(["run", "--rm", image_tag]);
            cmd.args(&test.command);

            let output = cmd.output()?;

            let passed = match (&test.expected_output, &test.expected_exit_code) {
                (Some(expected), _) => {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    stdout.trim().contains(expected.as_str())
                }
                (_, Some(expected_code)) => {
                    output.status.code() == Some(*expected_code)
                }
                _ => output.status.success(),
            };

            results.push(TestResult {
                name: test.name.clone(),
                passed,
                stdout: String::from_utf8_lossy(&output.stdout).to_string(),
                stderr: String::from_utf8_lossy(&output.stderr).to_string(),
                exit_code: output.status.code(),
            });

            if !passed {
                tracing::warn!(
                    test = %test.name,
                    "Image verification test failed"
                );
            }
        }

        Ok(results)
    }

    /// Scan image for vulnerabilities using Trivy.
    async fn scan_image(&self, image_tag: &str) -> Result<ScanResult, BuildError> {
        let output = Command::new("trivy")
            .args(["image",
                "--format", "json",
                "--severity", "HIGH,CRITICAL",
                "--ignore-unfixed",
                image_tag,
            ])
            .output()?;

        let scan: TrivyOutput = serde_json::from_slice(&output.stdout)?;

        let critical_count = scan.results.iter()
            .flat_map(|r| r.vulnerabilities.as_deref().unwrap_or(&[]))
            .filter(|v| v.severity == "CRITICAL")
            .count();

        Ok(ScanResult {
            total_vulnerabilities: scan.results.iter()
                .flat_map(|r| r.vulnerabilities.as_deref().unwrap_or(&[]))
                .count(),
            critical_count,
            high_count: 0, // Parsed from scan
            passed: critical_count == 0,
            raw: scan,
        })
    }

    /// Compute a deterministic image tag from definition content hash.
    fn compute_image_tag(&self, definition: &SandboxDef) -> String {
        let content = serde_json::to_string(&definition).unwrap();
        let hash = Sha256::digest(content.as_bytes());
        let short_hash = hex::encode(&hash[..8]);
        format!("blup/{}:v{}-sha256:{}",
            definition.name,
            definition.version,
            short_hash
        )
    }
}
```

### Image Tag Convention

```
blup/sandbox-python:v1-sha256:a1b2c3d4

blup/                    — Organization prefix
sandbox-python           — Sandbox name
v1                       — Dockerfile major version
sha256:a1b2c3d4           — Content hash of Dockerfile + pinned deps (first 8 chars)
```

This ensures:
- **Immutability**: Same content → same tag (no `:latest` mutation).
- **Traceability**: Tag reveals the exact Dockerfile + deps that produced the image.
- **Reproducibility**: Rebuilding the same Dockerfile + deps produces the same tag.

## Testing Strategy

### prompt-tester Tests

| Test | Method | Expected |
|------|--------|----------|
| All prompts pass with valid fixtures | `test-all` (mock mode) | Exit 0, all fixtures pass |
| Schema violation detected | Fixture with intentionally bad mock response | Exit 1, schema error reported |
| Missing template variable | Omit a required variable from fixture input | Clear error: "Missing variable: X" |
| Template not found | Request non-existent prompt | Clear error: "Prompt not found: X" |
| Gateway unavailable | `test-all --gateway` with gateway down | Clear error with health check diagnostic |
| Gate capture produces valid fixture | `capture <prompt>` | Output is valid JSON with schema validation |
| All 5 Phase 1 prompts have ≥2 fixtures | `list` command | 5 prompts × ≥2 fixtures each |
| JUnit XML output | `test-all --json-output` | Valid JUnit XML for CI integration |
| Semantic check catches contradiction | Fixture with feasible=true but non-empty suggestions | Semantic error reported |

### sandbox-builder Tests

| Test | Method | Expected |
|------|--------|----------|
| Build all sandbox images | `build all` in CI with Docker | All images built successfully |
| Digest verification | `verify <image>` after build | Actual digest matches expected |
| Reproducible build | Build same Dockerfile twice | Same image digest |
| Vulnerability scan | `scan <image>` with Trivy | Zero CRITICAL findings |
| Malformed YAML definition | Build with invalid YAML | Clear parse error with line number |
| Missing Dockerfile | Build non-existent sandbox | Clear file-not-found error |
| Base image digest mismatch | Tamper with expected digest | Digest mismatch error, build fails |
| Verification test failure | Build with test that expects wrong version | Test failure reported, build fails |

## Quality Gates

- [ ] `prompt-tester test-all` passes on all Phase 1 prompts (mock mode, CI)
- [ ] Every prompt has ≥2 fixtures covering different input scenarios
- [ ] Capture mode can successfully record fixtures from the LLM Gateway
- [ ] All sandbox images build successfully with pinned base image digests
- [ ] Sandbox image builds are reproducible (same digest on rebuild)
- [ ] Vulnerability scan passes (zero critical CVEs in built images)
- [ ] All sandbox images pass their verification tests (correct Python version, etc.)
- [ ] Image digests are recorded in a manifest file for audit
- [ ] SBOM is generated for each sandbox image
