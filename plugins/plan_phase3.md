# Plugins Module — Implementation Plan

## Module Overview

`plugins/` contains domain-specific learning extensions. A plugin provides specialized capabilities for programming, mathematics, language learning, physics simulation, art, or other domains. Plugins extend what the platform can teach without bloating the core.

**Core principle:** Plugins are untrusted by default. They request capabilities through explicit permission checks and communicate through structured schemas. They can never access files, network, shell, databases, or other plugins directly.

## Phase Scope

| Phase | Deliverables | Status |
|-------|-------------|--------|
| Phase 1 | None — plugin system explicitly excluded | — |
| Phase 2 | None | — |
| Phase 2.5 | None | — |
| Phase 3 | Plugin manifests, HTTP/microservice isolation, lifecycle management, contract tests | Planned |
| Future | WASM Component Model migration (if warranted) | Deferred |

## Phase 3 Detailed Plan

### Architecture

```
┌──────────────────────────────────────────────────────────────┐
│                     Agent Core (crates/)                      │
│  ┌────────────────────────────────────────────────────────┐  │
│  │                  Plugin Host                             │  │
│  │  ┌──────────┐  ┌──────────┐  ┌──────────────────────┐  │  │
│  │  │ Manifest  │  │Permission│  │  Lifecycle Manager   │  │  │
│  │  │  Loader   │  │  Engine  │  │  Load→Init→Activate  │  │  │
│  │  │           │  │          │  │  →Execute→Pause→     │  │  │
│  │  │           │  │          │  │  Unload               │  │  │
│  │  └─────┬─────┘  └────┬─────┘  └──────────┬───────────┘  │  │
│  │        │              │                    │              │  │
│  │  ┌─────▼──────────────▼────────────────────▼──────────┐  │  │
│  │  │               Plugin Runtime                         │  │  │
│  │  │  HTTP microservice (per plugin) or stdin/stdout     │  │  │
│  │  │  Future: WASM (Wasmtime)                             │  │  │
│  │  └─────────────────────────────────────────────────────┘  │  │
│  └────────────────────────────────────────────────────────┘  │
│                      │                                        │
│              Tool Router                                       │
│              (dispatches tool requests)                        │
└──────────────────────────────────────────────────────────────┘

Plugins communicate with Core through structured JSON messages.
They NEVER access external resources directly.
```

### File Structure

```
plugins/
├── AGENTS.md
├── plan_phase3.md
├── manifests/
│   └── example-plugin.manifest.v1.json
├── math-engine/                    # Example domain plugin
│   ├── manifest.v1.json
│   ├── src/
│   │   └── main.py                 # Plugin entry point
│   ├── tests/
│   │   ├── manifest_validation_test.json
│   │   ├── contract_tests.py
│   │   └── permission_denial_tests.py
│   └── README.md
├── code-practice/                  # Example domain plugin
│   ├── manifest.v1.json
│   ├── src/
│   │   └── main.py
│   └── tests/
│       └── ...
├── language-tutor/                 # Example domain plugin
│   └── ...
└── physics-sim/                    # Example domain plugin
    └── ...
```

### Plugin Manifest Schema

Coordinated with `schemas/plugin_manifest.v1.schema.json`:

```json
{
  "plugin_id": "math-engine",
  "name": "Math Engine Plugin",
  "version": "1.0.0",
  "description": "Provides symbolic math computation, equation solving, and visualization generation for mathematics learning content.",
  "author": "Blup Project",
  "license": "MIT",
  "runtime": {
    "type": "http_microservice",
    "entrypoint": "python main.py",
    "port": "auto",
    "health_check_path": "/health"
  },
  "capabilities": [
    {
      "id": "generate:math_exercise",
      "description": "Generate math exercises with varying difficulty",
      "input_schema": "math_exercise_request.v1.schema.json",
      "output_schema": "math_exercise.v1.schema.json"
    },
    {
      "id": "evaluate:math_answer",
      "description": "Evaluate a learner's answer to a math problem",
      "input_schema": "math_answer_eval_request.v1.schema.json",
      "output_schema": "math_answer_eval_result.v1.schema.json"
    },
    {
      "id": "request:tool",
      "description": "Request execution of a tool (math engine)",
      "input_schema": "tool_request.v1.schema.json",
      "output_schema": "tool_result.v1.schema.json"
    }
  ],
  "permissions": [
    "generate:content",
    "generate:assessment",
    "request:tool",
    "tool:math"
  ],
  "dependencies": {
    "schemas": ["math_exercise_request.v1", "math_answer_eval_request.v1"],
    "system": ["python>=3.12", "sympy>=1.12"]
  },
  "resource_limits": {
    "max_memory_mb": 256,
    "max_cpu_time_per_request_ms": 5000,
    "max_concurrent_requests": 5
  }
}
```

### Permission Model

Plugins request permissions in their manifest. Core's permission engine enforces them.

**Available permissions:**

| Permission | Description | Risk Level |
|-----------|-------------|------------|
| `read:curriculum` | Read the learner's curriculum plan | Medium — accesses learning data |
| `read:user_profile` | Read the learner's profile (minimized, redacted) | High — accesses personal data |
| `generate:content` | Generate learning content (text, Markdown) | Low — output is validated |
| `generate:assessment` | Generate exercises and assessments | Medium — affects learning quality |
| `generate:scene` | Generate Bevy scene specifications | Medium — affects rendering |
| `request:tool` | Request tool execution through Core | High — delegates execution |
| `tool:math` | Access math computation tools | Medium — deterministic but resource-bound |
| `tool:code_run` | Request sandboxed code execution | High — runs user code |
| `tool:render` | Request scene rendering | Low — output only |

**Forbidden capabilities (hard-coded denials):**

| Capability | Reason |
|-----------|--------|
| Direct file-system access | Privacy, sandbox escape risk |
| Direct network access | Data exfiltration risk |
| Direct shell command execution | Host compromise risk |
| Direct database access | Bypasses access control and audit |
| Direct access to other plugins | Isolation violation |
| Direct Bevy ECS manipulation | Could crash or exploit the renderer |
| Final learning progress decisions | Core owns assessment integrity |

**Permission enforcement:**
- Plugin host checks requested permission against manifest before every capability call.
- User consent may be required for high-risk permissions (`read:user_profile`, `tool:code_run`).
- Permission decisions are logged in the audit trail.

### Plugin Lifecycle

```
  Load ──→ Init ──→ Activate ──→ Execute ──→ Pause ──→ Unload
   │        │         │            │            │          │
   │        │         │            │            │          │
   ▼        ▼         ▼            ▼            ▼          ▼
 Validate  Start   Run health    Handle      Freeze     Stop
 manifest  plugin  check,       capability  state,     plugin,
 file,     process enable      requests    release    remove
 check     (HTTP   permissions  from Core   resources  process
 deps      or        │                                   │
           stdin)    │                                   │
                     ▼                                   ▼
              Permission check                   Audit log entry
              before every call
```

**Lifecycle states:**

| State | Description | Allowed Transitions |
|-------|-------------|---------------------|
| `Loaded` | Manifest validated, dependencies checked | → `Init`, `Unload` |
| `Initialized` | Plugin process started, not yet accepting requests | → `Activate`, `Unload` |
| `Active` | Plugin running, health checks passing, accepting capability calls | → `Execute`, `Pause`, `Unload` |
| `Executing` | Plugin is handling a capability request | → `Active` (complete), `Pause` (timeout), `Error` |
| `Paused` | Plugin suspended, resources held, not accepting requests | → `Activate`, `Unload` |
| `Error` | Plugin encountered an error or crashed | → `Init` (restart), `Unload` |
| `Unloaded` | Plugin process terminated, resources freed | Terminal state |

### Communication Protocols

#### Phase 3: HTTP Microservices

Each plugin runs as an HTTP server on a random localhost port. Core communicates via HTTP.

**Plugin health check:**
```
GET /health → 200 OK { "status": "ok", "version": "1.0.0" }
```

**Capability call:**
```
POST /capability/{capability_id}
Content-Type: application/json
Body: PluginRequest
Response: PluginResponse
```

**PluginRequest schema:**
```json
{
  "request_id": "uuid",
  "session_id": "uuid",
  "capability_id": "generate:math_exercise",
  "parameters": {
    "topic": "quadratic equations",
    "difficulty": "medium",
    "count": 3
  },
  "context": {
    "chapter_id": "quadratic-equations",
    "user_level": "intermediate",
    "previous_exercises": []
  }
}
```

**PluginResponse schema:**
```json
{
  "request_id": "uuid",
  "status": "success | error | partial",
  "result": { /* capability-specific output, schema-validated */ },
  "error": {
    "code": "string",
    "message": "string (redacted)"
  },
  "metadata": {
    "duration_ms": 234,
    "model_used": "string (if LLM was called)"
  }
}
```

#### Future: stdin/stdout Protocol

For plugins that don't need a full HTTP server:
```
Core → plugin stdin: JSON request line
Plugin → Core stdout: JSON response line
Plugin → Core stderr: structured diagnostics
```

#### Future: WASM (Wasmtime)

For maximum isolation:
- Plugin compiled to `.wasm` with WASI preview 2.
- Wasmtime runtime embedded in `crates/plugin-host`.
- Capability calls are WASM function exports.
- Resource limits enforced by Wasmtime engine.

### Plugin Isolation Rules

| Isolation Layer | HTTP Microservice | stdin/stdout | WASM |
|-----------------|-------------------|--------------|------|
| Process isolation | Yes (separate process) | Yes | Yes (Wasmtime) |
| Network isolation | localhost only, random port | None | None |
| Filesystem isolation | No access (Core doesn't expose paths) | None | WASI sandbox |
| Resource limits | OS process limits | OS process limits | Wasmtime fuel metering |
| Crash isolation | Plugin crash doesn't affect Core | Same | Same |
| Memory isolation | OS process memory | Same | Wasmtime memory |

### Reference Plugin Implementation: Math Engine

This is a complete, production-quality reference plugin. All other plugins follow the same pattern.

#### Manifest

```json
{
  "plugin_id": "math-engine",
  "name": "Math Engine Plugin",
  "version": "1.0.0",
  "description": "Symbolic math computation, exercise generation, and answer evaluation for mathematics learning content.",
  "author": "Blup Project",
  "license": "MIT",
  "runtime": {
    "type": "http_microservice",
    "entrypoint": "src/main.py",
    "health_check_path": "/health"
  },
  "capabilities": [
    {
      "id": "generate:math_exercise",
      "description": "Generate math exercises with varying difficulty and topics",
      "input_schema": "math_exercise_request.v1.schema.json",
      "output_schema": "math_exercise.v1.schema.json",
      "required_permissions": ["generate:content"]
    },
    {
      "id": "evaluate:math_answer",
      "description": "Evaluate a learner's answer using symbolic comparison",
      "input_schema": "math_answer_eval_request.v1.schema.json",
      "output_schema": "math_answer_eval_result.v1.schema.json",
      "required_permissions": ["generate:assessment"]
    },
    {
      "id": "request:tool",
      "description": "Execute math computation via SymPy",
      "input_schema": "tool_request.v1.schema.json",
      "output_schema": "tool_result.v1.schema.json",
      "required_permissions": ["request:tool", "tool:math"]
    }
  ],
  "permissions": ["generate:content", "generate:assessment", "request:tool", "tool:math"],
  "dependencies": {
    "python": ">=3.12",
    "packages": ["sympy>=1.12", "fastapi>=0.115", "uvicorn>=0.30"]
  },
  "resource_limits": {
    "max_memory_mb": 256,
    "max_cpu_time_per_request_ms": 5000
  }
}
```

#### Plugin Server (main.py)

```python
"""Math Engine Plugin — FastAPI server for math capabilities."""
import os
import json
import traceback
from contextlib import asynccontextmanager
from fastapi import FastAPI, Request, HTTPException
from fastapi.responses import JSONResponse
import structlog
from sympy import sympify, SympifyError, solve, diff, integrate, limit, oo

from src.capabilities.exercise_generator import ExerciseGenerator
from src.capabilities.answer_evaluator import AnswerEvaluator
from src.capabilities.tool_executor import ToolExecutor

logger = structlog.get_logger()

# ── Plugin identity ──
PLUGIN_ID = os.environ.get("PLUGIN_ID", "math-engine")
TEST_MODE = os.environ.get("BLUP_TEST_MODE", "0") == "1"

# ── Initialize capability handlers ──
exercise_generator = ExerciseGenerator()
answer_evaluator = AnswerEvaluator()
tool_executor = ToolExecutor()


@asynccontextmanager
async def lifespan(app: FastAPI):
    """Startup and shutdown."""
    logger.info("plugin_starting", plugin_id=PLUGIN_ID, test_mode=TEST_MODE)
    yield
    logger.info("plugin_stopping", plugin_id=PLUGIN_ID)


app = FastAPI(title=f"Blup Plugin: {PLUGIN_ID}", version="1.0.0", lifespan=lifespan)


# ── Health ──
@app.get("/health")
async def health():
    return {"status": "ok", "plugin_id": PLUGIN_ID, "version": "1.0.0"}


# ── Capability router ──
@app.post("/capability/{capability_id}")
async def handle_capability(capability_id: str, request: Request):
    """Route capability calls to the correct handler."""
    try:
        body = await request.json()
    except json.JSONDecodeError:
        raise HTTPException(400, detail={"error": {"code": "INVALID_JSON", "message": "Request body is not valid JSON"}})

    request_id = body.get("request_id", "unknown")
    logger.info("capability_call", capability=capability_id, request_id=request_id)

    try:
        if capability_id == "generate:math_exercise":
            result = await exercise_generator.generate(body)
        elif capability_id == "evaluate:math_answer":
            result = await answer_evaluator.evaluate(body)
        elif capability_id == "request:tool":
            result = await tool_executor.execute(body)
        else:
            raise HTTPException(404, detail={"error": {"code": "UNKNOWN_CAPABILITY", "message": f"No handler for '{capability_id}'"}})

        return JSONResponse(content=result)

    except HTTPException:
        raise
    except Exception as e:
        logger.error("capability_error", capability=capability_id, error=str(e), traceback=traceback.format_exc())
        return JSONResponse(
            status_code=500,
            content={"error": {"code": "PLUGIN_ERROR", "message": str(e) if TEST_MODE else "Internal plugin error"}}
        )
```

#### Exercise Generator

```python
"""Generate math exercises with varying difficulty and topics."""
import random
from sympy import symbols, sympify, latex, solve, diff, integrate, limit, oo, sin, cos, tan, exp, log

class ExerciseGenerator:
    TOPICS = ["algebra", "calculus", "geometry", "statistics", "trigonometry"]

    async def generate(self, request: dict) -> dict:
        params = request.get("parameters", {})
        topic = params.get("topic", "algebra")
        difficulty = params.get("difficulty", "medium")
        count = min(params.get("count", 1), 5)

        if topic not in self.TOPICS:
            return self._error("INVALID_TOPIC", f"Topic must be one of: {', '.join(self.TOPICS)}")

        exercises = []
        for _ in range(count):
            exercise = self._generate_single(topic, difficulty)
            exercises.append(exercise)

        return {
            "request_id": request["request_id"],
            "status": "success",
            "result": {"exercises": exercises},
            "metadata": {"duration_ms": 0, "topic": topic, "difficulty": difficulty},
        }

    def _generate_single(self, topic: str, difficulty: str) -> dict:
        x = symbols('x')

        if topic == "algebra":
            return self._algebra_exercise(x, difficulty)
        elif topic == "calculus":
            return self._calculus_exercise(x, difficulty)
        elif topic == "trigonometry":
            return self._trigonometry_exercise(x)
        elif topic == "geometry":
            return self._geometry_exercise()
        else:
            return self._algebra_exercise(x, difficulty)

    def _algebra_exercise(self, x, difficulty: str) -> dict:
        """Generate an algebra exercise: solve an equation."""
        if difficulty == "easy":
            a, b = random.randint(1, 10), random.randint(1, 20)
            equation = f"{a}*x + {b} = 0"
            prompt = f"Solve for x: {a}x + {b} = 0"
            solution_expr = -b / a
        elif difficulty == "medium":
            a, b, c = random.randint(1, 5), random.randint(1, 10), random.randint(1, 15)
            equation = f"{a}*x**2 + {b}*x + {c} = 0"
            prompt = f"Solve the quadratic equation: {a}x² + {b}x + {c} = 0"
            discriminant = b**2 - 4*a*c
            if discriminant >= 0:
                sqrt_d = int(discriminant ** 0.5) if discriminant ** 0.5 == int(discriminant ** 0.5) else f"√{discriminant}"
                solution_expr = f"x = ({-b} ± {sqrt_d}) / {2*a}"
            else:
                solution_expr = f"x = ({-b} ± i√{abs(discriminant)}) / {2*a}"
        else:  # hard
            a, b = random.randint(2, 6), random.randint(1, 10)
            equation = f"{a}*exp({b}*x) = {a}*{b}"
            prompt = f"Solve for x: {a}e^({b}x) = {a*b}"
            solution_expr = f"x = ln({b})/{b}"

        return {
            "id": f"math-{random.randint(10000, 99999)}",
            "question": prompt,
            "type": "short_answer",
            "difficulty": difficulty,
            "topic": "algebra",
            "solution": {
                "expression": solution_expr if isinstance(solution_expr, str) else str(solution_expr),
                "numeric_approximation": str(round(float(solution_expr), 4)) if not isinstance(solution_expr, str) else None,
                "steps": [
                    f"Start with: {equation}",
                    "Apply algebraic operations to isolate x",
                    f"Solution: {solution_expr}"
                ]
            },
            "hints": [
                "Try moving all terms to one side first",
                "Remember to check your answer by substituting back",
            ],
            "explanation": f"To solve {equation}, we isolate x step by step. The solution is {solution_expr}."
        }

    def _calculus_exercise(self, x, difficulty: str) -> dict:
        """Generate a calculus exercise: differentiation or integration."""
        if difficulty == "easy":
            n = random.randint(2, 5)
            prompt = f"Find the derivative of f(x) = x^{n}"
            solution_expr = f"{n}x^{n-1}"
            steps = [f"f(x) = x^{n}", f"Using the power rule: d/dx(x^n) = n·x^(n-1)", f"f'(x) = {solution_expr}"]
        elif difficulty == "medium":
            a, n = random.randint(2, 5), random.randint(3, 6)
            prompt = f"Find the integral of f(x) = {a}x^{n}"
            solution_expr = f"({a/(n+1)})x^{n+1} + C"
            steps = [f"f(x) = {a}x^{n}", f"Using the power rule for integration: ∫x^n dx = x^(n+1)/(n+1) + C",
                     f"∫{a}x^{n} dx = {solution_expr}"]
        else:
            prompt = "Find the limit: lim(x→∞) (1 + 1/x)^x"
            solution_expr = "e"
            steps = ["This is the definition of Euler's number e", "lim(x→∞) (1 + 1/x)^x = e"]

        return {
            "id": f"math-{random.randint(10000, 99999)}",
            "question": prompt,
            "type": "short_answer",
            "difficulty": difficulty,
            "topic": "calculus",
            "solution": {"expression": solution_expr, "steps": steps},
            "hints": ["Review the basic rules of differentiation/integration"],
            "explanation": "\n".join(steps),
        }

    def _trigonometry_exercise(self, x) -> dict:
        angle = random.choice([30, 45, 60, 90, 120, 180])
        func = random.choice(["sin", "cos", "tan"])
        prompt = f"Calculate {func}({angle}°)"
        import math
        rad = math.radians(angle)
        values = {"sin": math.sin, "cos": math.cos, "tan": math.tan}
        solution = round(values[func](rad), 4)
        return {
            "id": f"math-{random.randint(10000, 99999)}",
            "question": prompt,
            "type": "short_answer",
            "difficulty": "easy",
            "topic": "trigonometry",
            "solution": {"expression": str(solution), "numeric_approximation": str(solution)},
            "hints": [f"Recall the unit circle values for {angle}°"],
            "explanation": f"{func}({angle}°) = {solution}"
        }

    def _geometry_exercise(self) -> dict:
        radius = random.randint(3, 15)
        prompt = f"A circle has radius {radius} cm. Find its area and circumference."
        area = round(3.14159 * radius**2, 2)
        circumference = round(2 * 3.14159 * radius, 2)
        return {
            "id": f"math-{random.randint(10000, 99999)}",
            "question": prompt,
            "type": "short_answer",
            "difficulty": "easy",
            "topic": "geometry",
            "solution": {
                "expression": f"Area = {area} cm², Circumference = {circumference} cm",
                "steps": [
                    f"Area = π × r² = π × {radius}² = {area} cm²",
                    f"Circumference = 2 × π × r = 2 × π × {radius} = {circumference} cm"
                ]
            },
            "hints": ["Area = πr², Circumference = 2πr"],
            "explanation": f"For a circle with radius {radius} cm:\n- Area = π × {radius}² = {area} cm²\n- Circumference = 2 × π × {radius} = {circumference} cm"
        }

    def _error(self, code: str, message: str) -> dict:
        return {"status": "error", "error": {"code": code, "message": message}}
```

#### Answer Evaluator

```python
"""Evaluate learner answers using symbolic comparison via SymPy."""
from sympy import sympify, SympifyError, symbols, simplify, N

class AnswerEvaluator:
    async def evaluate(self, request: dict) -> dict:
        params = request.get("parameters", {})
        learner_answer = params.get("answer", "")
        expected_solution = params.get("expected_solution", "")
        tolerance = params.get("tolerance", 1e-10)

        if not learner_answer or not expected_solution:
            return self._error("MISSING_PARAMS", "Both 'answer' and 'expected_solution' are required")

        try:
            # Parse both expressions
            x = symbols('x')
            learner_expr = sympify(learner_answer)
            expected_expr = sympify(expected_solution)

            # Simplify both for comparison
            learner_simplified = simplify(learner_expr)
            expected_simplified = simplify(expected_expr)

            # Method 1: Symbolic equality
            if learner_simplified.equals(expected_simplified):
                return self._result(request, True, 1.0, "Symbolic match: expressions are mathematically equivalent")

            # Method 2: Algebraic difference
            difference = simplify(learner_simplified - expected_simplified)
            if difference == 0:
                return self._result(request, True, 1.0, "Algebraic match: difference is zero")

            # Method 3: Numeric evaluation at multiple points
            import random
            test_points = [random.uniform(-10, 10) for _ in range(5)]
            max_diff = 0.0
            for point in test_points:
                subs = {x: point}
                try:
                    learner_val = float(N(learner_simplified.subs(subs)))
                    expected_val = float(N(expected_simplified.subs(subs)))
                    diff = abs(learner_val - expected_val)
                    max_diff = max(max_diff, diff)
                except Exception:
                    continue

            if max_diff < tolerance:
                return self._result(request, True, 0.9,
                    f"Numeric match: maximum difference {max_diff:.2e} < tolerance {tolerance}")
            elif max_diff < tolerance * 100:
                return self._result(request, False, 0.5,
                    f"Close but not exact: maximum difference {max_diff:.2e}. Check your expression.")

            return self._result(request, False, 0.0,
                f"No match. Your answer differs from the expected solution. Maximum difference: {max_diff:.2e}")

        except SympifyError as e:
            return self._result(request, False, 0.0,
                f"Could not parse your answer as a mathematical expression: {e}")
        except Exception as e:
            return self._error("EVALUATION_ERROR", str(e))

    def _result(self, request: dict, is_correct: bool, score: float, feedback: str) -> dict:
        return {
            "request_id": request["request_id"],
            "status": "success",
            "result": {
                "is_correct": is_correct,
                "score": score,
                "max_score": 1.0,
                "feedback": feedback,
            },
            "metadata": {"duration_ms": 0},
        }

    def _error(self, code: str, message: str) -> dict:
        return {"status": "error", "error": {"code": code, "message": message}}
```

#### Tool Executor

```python
"""Execute math computations via SymPy."""
from sympy import sympify, solve, diff, integrate, limit, oo, latex, factor, expand, simplify

class ToolExecutor:
    async def execute(self, request: dict) -> dict:
        params = request.get("parameters", {})
        expression = params.get("expression", "")
        operation = params.get("operation", "evaluate")

        if not expression:
            return self._error("MISSING_PARAMS", "'expression' is required")

        try:
            expr = sympify(expression)

            if operation == "evaluate":
                result = expr.evalf()
            elif operation == "solve":
                result = solve(expr)
            elif operation == "diff":
                result = diff(expr)
            elif operation == "integrate":
                result = integrate(expr)
            elif operation == "limit":
                result = limit(expr, symbols('x'), oo)
            elif operation == "simplify":
                result = simplify(expr)
            elif operation == "expand":
                result = expand(expr)
            elif operation == "factor":
                result = factor(expr)
            elif operation == "latex":
                result = latex(expr)
            else:
                return self._error("UNKNOWN_OPERATION", f"Unknown operation: {operation}")

            return {
                "request_id": request["request_id"],
                "status": "success",
                "result": {
                    "expression": str(result),
                    "latex": latex(result) if hasattr(result, 'evalf') else str(result),
                    "operation": operation,
                },
                "metadata": {"duration_ms": 0},
            }

        except Exception as e:
            return self._error("COMPUTATION_ERROR", str(e))

    def _error(self, code: str, message: str) -> dict:
        return {"status": "error", "error": {"code": code, "message": message}}
```

#### Other Plugin Types (Summary)

**Code Practice Plugin** — Python + subprocess calls to Docker sandbox:
- `generate:coding_exercise` — LLM-assisted generation with structured output (language, starter code, test cases)
- `evaluate:code_submission` — runs learner's code against test cases in sandbox; returns pass/fail + output
- Key dependency: `docker` CLI access through Core's sandbox manager

**Language Tutor Plugin** — Python with NLP libraries:
- `generate:vocabulary_exercise` — spaced-repetition word lists from curriculum topics
- `generate:grammar_exercise` — fill-in-the-blank, sentence transformation
- `evaluate:translation` — semantic similarity scoring using sentence transformers
- Key dependency: `sentence-transformers` package

**Physics Simulation Plugin** — Python producing SceneSpec JSON:
- `generate:physics_scene` — produces `SceneSpec` for projectile motion, collisions, orbits
- `generate:physics_exercise` — embeds simulation parameters + questions
- Key output: Bevy-compatible `SceneSpec` with initial conditions and forces

### Testing Strategy

| Test Category | Method | Scope |
|---------------|--------|-------|
| Manifest validation | Schema check | Every manifest is valid JSON Schema |
| Contract tests | Mock capability call → validate response | Each declared capability |
| Permission denial | Request unauthorized capability → assert rejection | Every forbidden permission |
| Malformed input | Send invalid JSON → assert structured error | Every capability |
| Lifecycle tests | Load → Init → Activate → Execute → Pause → Unload | Every lifecycle transition |
| Crash recovery | Kill plugin process → assert error state → restart | Plugin host resilience |
| Resource limit | Exceed memory/cpu limit → assert plugin throttled/terminated | Resource enforcement |
| Health check | Plugin returns unhealthy → assert Core detects | Health monitoring |

All tests must use synthetic inputs. No real learner data. No paid API calls.

### Logging and Observability

Plugin audit logs:

```json
{
  "timestamp": "2025-01-01T00:00:00Z",
  "level": "INFO",
  "plugin_id": "math-engine",
  "plugin_version": "1.0.0",
  "request_id": "uuid",
  "session_id": "uuid",
  "capability": "generate:math_exercise",
  "permission_decision": "granted",
  "lifecycle_state": "executing",
  "duration_ms": 234,
  "error_code": null,
  "resource_usage": {
    "memory_mb": 45,
    "cpu_time_ms": 200
  }
}
```

Never log: raw learner content sent to plugins, plugin internal state, or plugin process environment variables.

### Security and Privacy Rules

| Rule | Enforcement |
|------|-------------|
| Plugins are untrusted by default | All capability calls require permission check |
| Plugins must not decide final learning progress | Core owns progress state; plugins provide input only |
| Plugins must not treat LLM output as deterministic | Plugin-generated content is schema-validated before use |
| Plugins must not access secrets | No environment variables with credentials passed to plugin processes |
| Plugins must not access network | localhost-only; no external network routing |
| Plugins must not access filesystem | No directory mounts; no file paths in request parameters |
| Plugins must not access databases | No database credentials; no DB connection libraries |
| Plugins must not access other plugins | No inter-plugin communication channels |

### Quality Gates

- [ ] Plugin manifest schema is defined and validated in CI
- [ ] Permission model is documented with threat model
- [ ] At least one reference plugin (math-engine) is implemented and tested
- [ ] Lifecycle manager handles all states and transitions
- [ ] Plugin crash does not affect Core or other plugins
- [ ] Permission denial tests pass for all forbidden capabilities
- [ ] Audit logs capture all plugin interactions
- [ ] No plugin has access to files, network (external), shell, databases, or other plugins

### Risk Register

| Risk | Impact | Mitigation |
|------|--------|------------|
| Plugin vulnerability exploited | Learner data exposure, host compromise | Process isolation, permission denial, no direct resource access |
| Malicious plugin in marketplace | Learners install harmful plugin | Manifest review; permission model warns on high-risk permissions; sandboxing |
| Plugin LLM calls bypass Core | Unaudited LLM usage, cost, privacy | Plugins cannot access network or API keys; LLM calls go through Core only |
| WASM Component Model instability | Migration rework | Start with HTTP microservices; evaluate WASM maturity in a separate ADR before migration |
| Plugin-to-plugin side channels | Isolation violation | No shared filesystem; no shared ports; monitor for unusual resource patterns |
