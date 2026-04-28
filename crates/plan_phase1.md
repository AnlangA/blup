# Crates Module — Phase 1: Agent Core

## Module Overview

`crates/agent-core` is the trusted Rust core of the Blup platform. It owns the learning state machine, orchestrates LLM calls, loads and renders prompt templates, validates all structured data against schemas, and exposes the HTTP + SSE API that the Web UI consumes. It is the single Rust crate for Phase 1.

**Core principle:** Agent Core treats LLMs, sandboxes, plugins, and renderers as external capabilities. It validates every input and output before changing state. It never trusts LLM output without schema validation.

## Phase 1 Scope

| Deliverable | Description | Status |
|-------------|-------------|--------|
| HTTP service | Axum server with REST + SSE endpoints | Planned |
| State machine | 8-state FSM with validated transitions | Planned |
| LLM boundary | HTTP client calling Python LLM Gateway (uses `openai` and `anthropic` official packages) | Planned |
| Prompt loader | Template loading, partial inclusion, variable substitution | Planned |
| Schema validator | JSON Schema validation for API payloads and LLM outputs | Planned |
| Session store | In-memory session storage (Phase 1); JSON file fallback | Planned |
| Structured logging | `tracing` with JSON output, redaction, required fields | Planned |
| Error handling | Typed errors with `thiserror`, structured error responses | Planned |

### Explicit Exclusions

- No persistent database (in-memory or JSON files only).
- No `storage`, `assessment-engine`, or `llm-gateway` crate splits.
- No sandbox execution.
- No plugin hosting.
- No Tauri or Bevy integration.
- No user authentication.
- No code execution of any kind.

## File Structure

```
crates/agent-core/
├── Cargo.toml
├── src/
│   ├── main.rs                     # Entry point: parse args, init tracing, start server
│   ├── lib.rs                      # Library exports for testing
│   ├── config.rs                   # Configuration (port, LLM URL, prompts dir, log level)
│   │
│   ├── server/
│   │   ├── mod.rs
│   │   ├── router.rs              # Axum router: all routes, middleware
│   │   └── middleware.rs           # Request ID injection, logging, error mapping
│   │
│   ├── api/
│   │   ├── mod.rs
│   │   ├── session.rs             # POST /api/session
│   │   ├── goal.rs                # POST /api/session/{id}/goal (SSE)
│   │   ├── profile.rs             # POST /api/session/{id}/profile/answer (SSE)
│   │   ├── curriculum.rs          # GET /api/session/{id}/curriculum
│   │   ├── chapter.rs             # GET /api/session/{id}/chapter/{ch_id} (SSE)
│   │   ├── ask.rs                 # POST /api/session/{id}/chapter/{ch_id}/ask (SSE)
│   │   ├── complete.rs            # POST /api/session/{id}/chapter/{ch_id}/complete
│   │   └── error.rs               # Error response formatting
│   │
│   ├── state/
│   │   ├── mod.rs
│   │   ├── machine.rs             # FSM: states, transitions, validation
│   │   ├── session.rs             # Session struct, SessionStore trait, InMemoryStore
│   │   └── types.rs               # State enum, Transition enum, SessionError
│   │
│   ├── llm/
│   │   ├── mod.rs
│   │   ├── client.rs              # HTTP client → Python LLM Gateway (localhost)
│   │   ├── types.rs               # Gateway request/response types
│   │   └── stream.rs              # SSE parsing for streaming responses from gateway
│   │
│   ├── prompts/
│   │   ├── mod.rs
│   │   ├── loader.rs              # PromptLoader: load, parse, include partials
│   │   ├── renderer.rs            # Variable substitution
│   │   └── types.rs               # PromptTemplate, RenderError
│   │
│   ├── validation/
│   │   ├── mod.rs
│   │   ├── schema_validator.rs    # JSON Schema validation (jsonschema crate)
│   │   └── fixtures.rs            # Load and cache schema files
│   │
│   ├── models/
│   │   ├── mod.rs
│   │   ├── learning_goal.rs
│   │   ├── feasibility_result.rs
│   │   ├── user_profile.rs
│   │   ├── curriculum_plan.rs
│   │   ├── chapter.rs
│   │   ├── message.rs
│   │   ├── chapter_progress.rs
│   │   └── api.rs                 # API request/response types (CreateSession, ErrorResponse, etc.)
│   │
│   └── observability/
│       ├── mod.rs
│       ├── tracing_init.rs        # tracing subscriber setup (JSON/fmt)
│       └── redaction.rs           # Redact sensitive fields from logs
│
├── tests/
│   ├── api/
│   │   ├── session_test.rs
│   │   ├── goal_test.rs
│   │   ├── profile_test.rs
│   │   ├── curriculum_test.rs
│   │   ├── chapter_test.rs
│   │   ├── ask_test.rs
│   │   └── complete_test.rs
│   ├── state/
│   │   ├── machine_test.rs        # All valid + invalid transitions
│   │   ├── session_test.rs        # Session create, resume, reset
│   │   └── error_test.rs          # Error state transitions
│   ├── llm/
│   │   ├── client_test.rs         # Mock LLM tests
│   │   └── stream_test.rs         # SSE parsing tests
│   ├── prompts/
│   │   ├── loader_test.rs
│   │   └── renderer_test.rs
│   └── validation/
│       └── schema_validator_test.rs
│
├── fixtures/                       # Test fixtures
│   ├── schemas/                   # Copy of schemas/ for tests
│   └── prompts/                   # Test prompt templates
│
└── prompts/                        # Symlink or copy of ../../prompts/
```

## Cargo Dependencies

```toml
[package]
name = "agent-core"
version = "0.1.0"
edition = "2024"
description = "Blup AI learning-agent core — orchestrates learning flow, LLM calls, validation"

[dependencies]
# HTTP and async
axum = { version = "0.7", features = ["macros"] }
tokio = { version = "1", features = ["full"] }
tower = "0.4"
tower-http = { version = "0.5", features = ["cors", "trace", "request-id"] }

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# HTTP client (LLM calls)
reqwest = { version = "0.12", features = ["json", "stream"] }

# Schema validation
jsonschema = "0.18"

# Logging
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["json", "env-filter"] }
tracing-axiom = "0.1"  # optional, for structured log export

# Error handling
thiserror = "1"

# UUID generation
uuid = { version = "1", features = ["v4", "serde"] }

# Date/time
chrono = { version = "0.4", features = ["serde"] }

# URL parsing
url = "2"

[dev-dependencies]
axum-test = "0.15"          # Axum integration test helpers
tower = { version = "0.4", features = ["util"] }
wiremock = "0.6"            # Mock HTTP server for LLM API
tempfile = "3"
pretty_assertions = "1"
```

## Configuration

```rust
// config.rs (conceptual)
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct Config {
    /// Host to bind the HTTP server
    pub host: String,                    // default: "127.0.0.1"

    /// Port to bind the HTTP server
    pub port: u16,                       // default: 3000

    /// Python LLM Gateway URL (localhost)
    pub llm_gateway_url: String,         // default: "http://127.0.0.1:9000"

    /// LLM API keys are managed by the Python gateway, not agent-core.
    /// Agent-core authenticates to the gateway with a shared secret.
    pub llm_gateway_secret: String,       // env: BLUP_LLM_GATEWAY_SECRET

    /// Default LLM model name (passed to gateway; gateway selects provider)
    pub llm_model: String,               // default: "gpt-4o"

    /// Directory containing prompt templates
    pub prompts_dir: PathBuf,            // default: "../prompts"

    /// Directory containing JSON Schema files
    pub schemas_dir: PathBuf,            // default: "../schemas"

    /// Log format: "json" or "pretty"
    pub log_format: String,              // default: "pretty" (dev), "json" (prod)

    /// Max concurrent sessions
    pub max_sessions: usize,             // default: 1000

    /// SSE keepalive interval in seconds
    pub sse_ping_interval_secs: u64,     // default: 15

    /// Max replay buffer size per SSE connection
    pub sse_replay_buffer_size: usize,   // default: 100
}

impl Config {
    pub fn from_env() -> Self {
        // Read from environment variables with sensible defaults
        // LLM_API_KEY is required; fail fast if missing
    }
}
```

## State Machine Implementation

### States

```rust
// state/types.rs (conceptual)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionState {
    Idle,
    GoalInput,
    FeasibilityCheck,
    ProfileCollection,
    CurriculumPlanning,
    ChapterLearning,
    Completed,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Transition {
    SubmitGoal,             // GoalInput → FeasibilityCheck
    GoalFeasible,           // FeasibilityCheck → ProfileCollection
    GoalInfeasible,         // FeasibilityCheck → GoalInput (adjust)
    StartProfile,           // (handled within ProfileCollection)
    ProfileComplete,        // ProfileCollection → CurriculumPlanning
    CurriculumReady,        // CurriculumPlanning → ChapterLearning
    StartChapter,           // ChapterLearning → ChapterLearning (different chapter)
    ChapterComplete,        // ChapterLearning → ChapterLearning / Completed
    AllChaptersDone,        // ChapterLearning → Completed
    ErrorOccurred,          // Any → Error
    Retry,                  // Error → previous state
    Reset,                  // Any → Idle
}
```

### State Machine

```rust
// state/machine.rs (conceptual)
pub struct StateMachine {
    current_state: SessionState,
    previous_state: Option<SessionState>,  // For retry from Error
    transition_history: Vec<TransitionRecord>,
}

#[derive(Debug, Clone)]
pub struct TransitionRecord {
    pub from: SessionState,
    pub to: SessionState,
    pub transition: Transition,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl StateMachine {
    pub fn new() -> Self {
        Self {
            current_state: SessionState::Idle,
            previous_state: None,
            transition_history: Vec::new(),
        }
    }

    pub fn current_state(&self) -> SessionState {
        self.current_state
    }

    /// Attempt a state transition. Returns Err if the transition is invalid.
    pub fn transition(&mut self, transition: Transition) -> Result<SessionState, StateError> {
        let next_state = self.validate_transition(&transition)?;

        let record = TransitionRecord {
            from: self.current_state,
            to: next_state,
            transition,
            timestamp: chrono::Utc::now(),
        };

        // Special handling for Error → Retry
        if next_state == SessionState::Error {
            self.previous_state = Some(self.current_state);
        }

        self.current_state = next_state;
        self.transition_history.push(record);

        Ok(self.current_state)
    }

    fn validate_transition(&self, transition: &Transition) -> Result<SessionState, StateError> {
        use SessionState::*;
        use Transition::*;

        match (&self.current_state, transition) {
            // Valid transitions
            (Idle, SubmitGoal) => Ok(GoalInput),
            (GoalInput, SubmitGoal) => Ok(FeasibilityCheck),
            (FeasibilityCheck, GoalFeasible) => Ok(ProfileCollection),
            (FeasibilityCheck, GoalInfeasible) => Ok(GoalInput),
            (ProfileCollection, ProfileComplete) => Ok(CurriculumPlanning),
            (CurriculumPlanning, CurriculumReady) => Ok(ChapterLearning),
            (ChapterLearning, ChapterComplete) => Ok(ChapterLearning),  // More chapters
            (ChapterLearning, AllChaptersDone) => Ok(Completed),
            (Completed, Reset) => Ok(Idle),

            // Error transitions
            (_, ErrorOccurred) => Ok(Error),
            (Error, Retry) => Ok(self.previous_state.unwrap_or(Idle)),
            (Error, Reset) => Ok(Idle),

            // Everything else is invalid
            _ => Err(StateError::InvalidTransition {
                from: self.current_state,
                transition: transition.clone(),
            }),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum StateError {
    #[error("Invalid transition: cannot {transition:?} from {from:?}")]
    InvalidTransition {
        from: SessionState,
        transition: Transition,
    },
}
```

### Session Store

```rust
// state/session.rs (conceptual)
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

pub struct Session {
    pub id: Uuid,
    pub state_machine: StateMachine,
    pub goal: Option<LearningGoal>,
    pub feasibility_result: Option<FeasibilityResult>,
    pub profile: Option<UserProfile>,
    pub curriculum: Option<CurriculumPlan>,
    pub current_chapter_id: Option<String>,
    pub chapter_progress: HashMap<String, ChapterProgress>,
    pub messages: Vec<Message>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[async_trait]
pub trait SessionStore: Send + Sync {
    async fn create(&self) -> Result<Session, SessionStoreError>;
    async fn get(&self, id: Uuid) -> Result<Option<Session>, SessionStoreError>;
    async fn update(&self, session: Session) -> Result<(), SessionStoreError>;
    async fn delete(&self, id: Uuid) -> Result<(), SessionStoreError>;
}

/// In-memory session store for Phase 1
pub struct InMemorySessionStore {
    sessions: Arc<RwLock<HashMap<Uuid, Session>>>,
}

#[async_trait]
impl SessionStore for InMemorySessionStore {
    async fn create(&self) -> Result<Session, SessionStoreError> {
        let id = Uuid::new_v4();
        let session = Session {
            id,
            state_machine: StateMachine::new(),
            goal: None,
            feasibility_result: None,
            profile: None,
            curriculum: None,
            current_chapter_id: None,
            chapter_progress: HashMap::new(),
            messages: Vec::new(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        self.sessions.write().await.insert(id, session.clone());
        Ok(session)
    }

    async fn get(&self, id: Uuid) -> Result<Option<Session>, SessionStoreError> {
        Ok(self.sessions.read().await.get(&id).cloned())
    }

    async fn update(&self, session: Session) -> Result<(), SessionStoreError> {
        self.sessions.write().await.insert(session.id, session);
        Ok(())
    }

    async fn delete(&self, id: Uuid) -> Result<(), SessionStoreError> {
        self.sessions.write().await.remove(&id);
        Ok(())
    }
}
```

### LLM Client (Rust → Python Gateway)

The Rust agent-core **does not call OpenAI or Anthropic APIs directly**. Instead, it calls a **Python LLM Gateway** service running on localhost. The Python gateway uses the official `openai` and `anthropic` Python packages. This separation ensures:

- AI SDK compatibility: Using the official packages guarantees compatibility with API changes, new features (prompt caching, extended thinking, structured outputs), and bug fixes.
- Provider abstraction: The Python gateway handles the differences between OpenAI and Anthropic APIs. Agent-core sends a unified request format.
- Security isolation: API keys live only in the Python gateway process, never in the Rust process memory.

```rust
// llm/client.rs (conceptual) — Thin HTTP client calling Python gateway
use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Clone)]
pub struct LlmClient {
    http: Client,
    gateway_url: String,       // http://127.0.0.1:9000
    gateway_secret: String,    // Shared secret for gateway auth
}

#[derive(Debug, Serialize)]
pub struct GatewayRequest {
    pub model: String,                      // "gpt-4o", "claude-sonnet-4-6", etc.
    pub messages: Vec<GatewayMessage>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<ResponseFormat>,  // "json_object"
    // Provider-agnostic: the gateway maps model name → provider
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GatewayMessage {
    pub role: String,   // "system" | "user" | "assistant"
    pub content: String,
}

#[derive(Debug, Serialize)]
pub struct ResponseFormat {
    #[serde(rename = "type")]
    pub format_type: String,
}

#[derive(Debug, Deserialize)]
pub struct GatewayResponse {
    pub content: String,
    pub model: String,
    pub provider: String,        // "openai" or "anthropic"
    pub usage: GatewayUsage,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct GatewayUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
    // Anthropic prompt caching (when available)
    pub cache_read_tokens: Option<u32>,
    pub cache_write_tokens: Option<u32>,
}

impl LlmClient {
    pub fn new(gateway_url: String, gateway_secret: String) -> Self {
        Self {
            http: Client::new(),
            gateway_url,
            gateway_secret,
        }
    }

    /// Non-streaming completion via Python gateway.
    pub async fn complete(
        &self,
        request: GatewayRequest,
    ) -> Result<GatewayResponse, LlmError> {
        let response = self
            .http
            .post(format!("{}/v1/gateway/complete", self.gateway_url))
            .header("X-Gateway-Secret", &self.gateway_secret)
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let body = response.text().await?;
            return Err(LlmError::GatewayError {
                status: response.status().as_u16(),
                body,
            });
        }

        Ok(response.json().await?)
    }

    /// Streaming completion via Python gateway (returns SSE stream).
    pub async fn complete_stream(
        &self,
        request: GatewayRequest,
    ) -> Result<impl Stream<Item = Result<StreamChunk, LlmError>>, LlmError> {
        let response = self
            .http
            .post(format!("{}/v1/gateway/complete", self.gateway_url))
            .header("X-Gateway-Secret", &self.gateway_secret)
            .json(&request)
            .send()
            .await?;

        // Gateway returns SSE stream; parse chunk events
        Ok(parse_gateway_sse_stream(response))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum LlmError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Gateway error {status}: {body}")]
    GatewayError { status: u16, body: String },

    #[error("Stream ended unexpectedly")]
    StreamEnded,

    #[error("Response validation failed: {0}")]
    Validation(String),

    #[error("Gateway rate limited; retry after {retry_after_seconds}s")]
    RateLimited { retry_after_seconds: u64 },

    #[error("Gateway unavailable: {0}")]
    GatewayUnavailable(String),
}
```

### Python LLM Gateway Service

The Python LLM Gateway is a lightweight HTTP service (FastAPI) that wraps the official `openai` and `anthropic` packages. It runs as a sidecar process on localhost.

#### File Structure

```
services/llm-gateway/
├── pyproject.toml                  # Python project config
├── requirements.txt                # Pinned dependencies
├── src/
│   ├── __init__.py
│   ├── main.py                     # FastAPI entry point
│   ├── config.py                   # Gateway configuration
│   ├── providers/
│   │   ├── __init__.py
│   │   ├── base.py                 # Abstract provider interface
│   │   ├── openai_provider.py      # OpenAI Chat Completions via `openai` package
│   │   └── anthropic_provider.py   # Anthropic Messages via `anthropic` package
│   ├── routes/
│   │   ├── __init__.py
│   │   ├── complete.py             # POST /v1/gateway/complete
│   │   └── health.py               # GET /health
│   ├── stream.py                   # SSE streaming helpers
│   └── retry.py                    # Retry + circuit breaker
└── tests/
    ├── test_openai_provider.py
    ├── test_anthropic_provider.py
    └── test_gateway.py
```

#### requirements.txt

```
openai>=1.55.0,<2.0.0       # Official OpenAI Python SDK
anthropic>=0.40.0,<1.0.0    # Official Anthropic Python SDK
fastapi>=0.115.0             # HTTP framework
uvicorn>=0.30.0              # ASGI server
httpx>=0.27.0                # Async HTTP client for streaming
pydantic>=2.8.0              # Request/response validation
tenacity>=8.5.0              # Retry library
structlog>=24.0.0            # Structured logging
python-dotenv>=1.0.0         # Environment variable loading
```

#### Main Entry Point

```python
# main.py
import uvicorn
from fastapi import FastAPI
from src.routes import complete, health

app = FastAPI(title="Blup LLM Gateway", version="0.1.0")
app.include_router(health.router)
app.include_router(complete.router)

if __name__ == "__main__":
    uvicorn.run(app, host="127.0.0.1", port=9000, log_level="info")
```

#### Provider Abstraction

```python
# providers/base.py
from abc import ABC, abstractmethod
from typing import AsyncIterator
from pydantic import BaseModel

class GatewayRequest(BaseModel):
    model: str
    messages: list[dict]
    temperature: float | None = None
    max_tokens: int | None = 1024
    stream: bool = False
    response_format: dict | None = None

class GatewayResponse(BaseModel):
    content: str
    model: str
    provider: str           # "openai" | "anthropic"
    usage: dict
    finish_reason: str | None = None

class StreamChunk(BaseModel):
    content: str
    index: int
    finish_reason: str | None = None

class BaseProvider(ABC):
    @abstractmethod
    def provider_name(self) -> str: ...

    @abstractmethod
    def supports_model(self, model: str) -> bool: ...

    @abstractmethod
    async def complete(self, request: GatewayRequest) -> GatewayResponse: ...

    @abstractmethod
    async def complete_stream(self, request: GatewayRequest) -> AsyncIterator[StreamChunk]: ...
```

#### OpenAI Provider

```python
# providers/openai_provider.py
from openai import AsyncOpenAI

class OpenAIProvider(BaseProvider):
    def __init__(self, api_key: str):
        self.client = AsyncOpenAI(api_key=api_key)

    def provider_name(self) -> str:
        return "openai"

    def supports_model(self, model: str) -> bool:
        return model.startswith(("gpt-", "o1", "o3", "o4"))

    async def complete(self, request: GatewayRequest) -> GatewayResponse:
        kwargs = {
            "model": request.model,
            "messages": request.messages,
            "max_tokens": request.max_tokens or 1024,
        }
        if request.temperature is not None:
            kwargs["temperature"] = request.temperature
        if request.response_format:
            kwargs["response_format"] = request.response_format

        response = await self.client.chat.completions.create(**kwargs)

        return GatewayResponse(
            content=response.choices[0].message.content or "",
            model=response.model,
            provider="openai",
            usage={
                "prompt_tokens": response.usage.prompt_tokens if response.usage else 0,
                "completion_tokens": response.usage.completion_tokens if response.usage else 0,
                "total_tokens": response.usage.total_tokens if response.usage else 0,
            },
            finish_reason=response.choices[0].finish_reason,
        )

    async def complete_stream(self, request: GatewayRequest) -> AsyncIterator[StreamChunk]:
        kwargs = {
            "model": request.model,
            "messages": request.messages,
            "max_tokens": request.max_tokens or 4096,
            "stream": True,
        }
        if request.temperature is not None:
            kwargs["temperature"] = request.temperature

        stream = await self.client.chat.completions.create(**kwargs)

        async for chunk in stream:
            if chunk.choices and chunk.choices[0].delta:
                content = chunk.choices[0].delta.content or ""
                if content:
                    yield StreamChunk(
                        content=content,
                        index=chunk.choices[0].index,
                        finish_reason=chunk.choices[0].finish_reason,
                    )
```

#### Anthropic Provider

```python
# providers/anthropic_provider.py
from anthropic import AsyncAnthropic

class AnthropicProvider(BaseProvider):
    def __init__(self, api_key: str):
        self.client = AsyncAnthropic(api_key=api_key)

    def provider_name(self) -> str:
        return "anthropic"

    def supports_model(self, model: str) -> bool:
        return model.startswith(("claude-", "anthropic."))

    async def complete(self, request: GatewayRequest) -> GatewayResponse:
        # Anthropic Messages API: system is a top-level param, not a message role
        system_msgs = [m["content"] for m in request.messages if m["role"] == "system"]
        messages = [m for m in request.messages if m["role"] != "system"]

        kwargs = {
            "model": request.model,
            "messages": messages,
            "max_tokens": request.max_tokens or 1024,
        }
        if system_msgs:
            kwargs["system"] = "\n\n".join(system_msgs)
        if request.temperature is not None:
            kwargs["temperature"] = request.temperature

        response = await self.client.messages.create(**kwargs)

        # Anthropic response: content is a list of blocks; extract text
        text_blocks = [b.text for b in response.content if b.type == "text"]
        content = "\n".join(text_blocks)

        return GatewayResponse(
            content=content,
            model=response.model,
            provider="anthropic",
            usage={
                "prompt_tokens": response.usage.input_tokens,
                "completion_tokens": response.usage.output_tokens,
                "total_tokens": response.usage.input_tokens + response.usage.output_tokens,
                "cache_read_tokens": getattr(response.usage, 'cache_read_input_tokens', None),
                "cache_write_tokens": getattr(response.usage, 'cache_creation_input_tokens', None),
            },
            finish_reason=response.stop_reason,
        )

    async def complete_stream(self, request: GatewayRequest) -> AsyncIterator[StreamChunk]:
        system_msgs = [m["content"] for m in request.messages if m["role"] == "system"]
        messages = [m for m in request.messages if m["role"] != "system"]

        kwargs = {
            "model": request.model,
            "messages": messages,
            "max_tokens": request.max_tokens or 4096,
            "stream": True,
        }
        if system_msgs:
            kwargs["system"] = "\n\n".join(system_msgs)
        if request.temperature is not None:
            kwargs["temperature"] = request.temperature

        async with self.client.messages.stream(**kwargs) as stream:
            async for event in stream:
                if event.type == "text_delta":
                    yield StreamChunk(
                        content=event.text,
                        index=event.index,
                        finish_reason=None,
                    )
                elif event.type == "message_stop":
                    # Final event — no text content
                    pass
```

#### Gateway Router

```python
# routes/complete.py
from fastapi import APIRouter, HTTPException, Header, Request
from fastapi.responses import StreamingResponse
from src.providers.openai_provider import OpenAIProvider
from src.providers.anthropic_provider import AnthropicProvider
from src.config import settings
import structlog

logger = structlog.get_logger()
router = APIRouter()

# Initialize providers with API keys from environment
providers = []
if settings.openai_api_key:
    providers.append(OpenAIProvider(settings.openai_api_key))
if settings.anthropic_api_key:
    providers.append(AnthropicProvider(settings.anthropic_api_key))

def select_provider(model: str):
    for p in providers:
        if p.supports_model(model):
            return p
    raise HTTPException(400, f"No provider available for model: {model}")

@router.post("/v1/gateway/complete")
async def gateway_complete(
    request: GatewayRequest,
    x_gateway_secret: str = Header(...),
):
    if x_gateway_secret != settings.gateway_secret:
        raise HTTPException(401, "Invalid gateway secret")

    provider = select_provider(request.model)

    if request.stream:
        async def generate():
            try:
                async for chunk in provider.complete_stream(request):
                    yield f"event: chunk\ndata: {chunk.model_dump_json()}\n\n"
                yield "event: done\ndata: {}\n\n"
            except Exception as e:
                logger.error("stream_error", error=str(e), model=request.model)
                yield f"event: error\ndata: {{\"code\": \"STREAM_ERROR\", \"message\": \"{str(e)}\"}}\n\n"

        return StreamingResponse(generate(), media_type="text/event-stream")
    else:
        response = await provider.complete(request)
        return response.model_dump()
```

#### Health Check and Startup

```python
# routes/health.py
from fastapi import APIRouter

router = APIRouter()

@router.get("/health")
async def health():
    return {"status": "ok", "version": "0.1.0"}

@router.get("/health/providers")
async def provider_health():
    return {
        "providers": [
            {"name": p.provider_name(), "models": p.supported_models()}
            for p in providers
        ]
    }
```

#### Agent-Core Startup with Gateway

In the Rust `main.rs`, agent-core spawns the Python gateway as a child process:

```rust
// main.rs (conceptual) — Startup sequence
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 1. Initialize tracing
    init_tracing();

    // 2. Load configuration
    let config = Config::from_env();

    // 3. Start Python LLM Gateway as a child process
    let gateway_process = std::process::Command::new("python")
        .args(["-m", "uvicorn", "services.llm_gateway.main:app"])
        .arg("--host").arg("127.0.0.1")
        .arg("--port").arg("9000")
        .env("OPENAI_API_KEY", &config.openai_api_key)
        .env("ANTHROPIC_API_KEY", &config.anthropic_api_key)
        .env("GATEWAY_SECRET", &config.llm_gateway_secret)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .context("Failed to start Python LLM Gateway")?;

    // 4. Wait for gateway health check
    wait_for_health("http://127.0.0.1:9000/health", Duration::from_secs(10)).await?;

    tracing::info!("Python LLM Gateway started");

    // 5. Initialize services
    let store = InMemorySessionStore::new();
    let prompts = PromptLoader::new(&config.prompts_dir)?;
    let validator = SchemaValidator::new(&config.schemas_dir)?;
    let llm = LlmClient::new(config.llm_gateway_url, config.llm_gateway_secret);

    let app_state = AppState { config, store, prompts, validator, llm, gateway_process };

    // 6. Build and start server
    let router = build_router(app_state);
    let listener = tokio::net::TcpListener::bind(&format!("{}:{}", app_state.config.host, app_state.config.port)).await?;

    tracing::info!("Agent-core listening on {}", listener.local_addr()?);
    axum::serve(listener, router).await?;

    Ok(())
}
```

### Why Python for the LLM Gateway?

| Factor | Python Gateway | Rust reqwest |
|--------|---------------|-------------|
| SDK compatibility | `openai` and `anthropic` are official, always up-to-date | Must manually track API changes |
| Streaming | Built-in async streaming in both SDKs | Manual SSE parsing |
| Prompt caching | Anthropic SDK supports `cache_control` natively | Must implement cache breakpoints manually |
| Extended thinking | Anthropic SDK supports thinking blocks natively | Must parse thinking content blocks manually |
| Retry logic | `tenacity` library; both SDKs have built-in retry | Must implement from scratch |
| Token counting | SDKs return precise token counts | Must call separate tokenizer endpoints |
| New features | Available day 1 via SDK update | Requires reverse-engineering API changes |
| Maintenance | SDK handles breaking API changes with deprecation paths | Manual migration for each provider API change |
| Performance | Python overhead for I/O-bound HTTP calls is negligible | Marginal latency advantage |

The Python gateway is an I/O-bound service — its job is to wait for HTTP responses from AI providers. Python's async performance is more than adequate for this workload. The Rust agent-core remains responsible for CPU-bound work: state machine transitions, schema validation, and prompt template rendering.

### Gateway Operational Lifecycle

#### Startup Sequence

```
Agent-Core (Rust) Startup:
  │
  ├─ 1. Load configuration (env vars, CLI flags)
  │
  ├─ 2. Spawn Python LLM Gateway as child process
  │     $ python -m uvicorn src.main:app --host 127.0.0.1 --port 9000
  │     Env: OPENAI_API_KEY, ANTHROPIC_API_KEY, GATEWAY_SECRET
  │
  ├─ 3. Poll GET /health every 500ms, timeout after 10s
  │     ├─ Success → Gateway ready
  │     └─ Timeout → Fatal error, agent-core exits
  │
  ├─ 4. Initialize remaining services (store, prompts, validator)
  │
  ├─ 5. Start Axum HTTP server
  │
  └─ 6. Ready to accept requests
```

#### Shutdown Sequence

```
Agent-Core receives SIGTERM / SIGINT:
  │
  ├─ 1. Stop accepting new HTTP requests (drain connections)
  │
  ├─ 2. Wait for in-flight requests to complete (grace period: 5s)
  │
  ├─ 3. Send SIGTERM to Python gateway child process
  │
  ├─ 4. Wait for gateway to exit (timeout: 5s)
  │     ├─ Exits cleanly → Continue
  │     └─ Timeout → Send SIGKILL
  │
  ├─ 5. Flush logs, close session store
  │
  └─ 6. Exit
```

#### Gateway Health Monitoring

Agent-core monitors the gateway's health on an ongoing basis:

```rust
// observability/gateway_health.rs (conceptual)
use std::time::Duration;
use tokio::time::interval;

struct GatewayHealthMonitor {
    gateway_url: String,
    interval: Duration,       // 10 seconds
    unhealthy_threshold: u32, // 3 consecutive failures
    consecutive_failures: AtomicU32,
}

impl GatewayHealthMonitor {
    async fn run(&self, shutdown: tokio::sync::watch::Receiver<bool>) {
        let mut ticker = interval(self.interval);

        loop {
            tokio::select! {
                _ = ticker.tick() => {
                    match self.check_health().await {
                        Ok(true) => {
                            self.consecutive_failures.store(0, Ordering::Relaxed);
                        }
                        Ok(false) | Err(_) => {
                            let failures = self.consecutive_failures
                                .fetch_add(1, Ordering::Relaxed) + 1;
                            tracing::warn!(
                                consecutive_failures = failures,
                                "LLM Gateway health check failed"
                            );
                            if failures >= self.unhealthy_threshold {
                                tracing::error!(
                                    "LLM Gateway unhealthy after {} consecutive failures. \
                                     Attempting restart...",
                                    failures
                                );
                                // Signal to restart gateway
                                // (handled by parent process supervisor)
                            }
                        }
                    }
                }
                _ = shutdown.changed() => {
                    break;
                }
            }
        }
    }

    async fn check_health(&self) -> Result<bool, reqwest::Error> {
        let response = reqwest::get(format!("{}/health", self.gateway_url))
            .timeout(Duration::from_secs(3))
            .await?;
        Ok(response.status().is_success())
    }
}
```

#### Error Propagation

Errors flow from the AI provider → Python gateway → Rust agent-core → Web UI:

```
Anthropic API returns 429 (Rate Limited)
  │
  ▼
Python Gateway: tenacity retries 3x with exponential backoff
  │ (if all retries fail)
  ▼
Python Gateway returns HTTP 502 with:
  { "error": { "code": "LLM_RATE_LIMITED", "message": "Rate limited after 3 retries",
    "provider": "anthropic", "retry_after_seconds": 30 } }
  │
  ▼
Rust LlmClient receives 502 → maps to LlmError::RateLimited
  │
  ▼
API handler maps to SSE error event:
  event: error
  data: {"code": "LLM_RATE_LIMITED", "message": "AI service temporarily unavailable..."}
  │
  ▼
Web UI displays error with retry timer ("Try again in 30s")
```

Error codes from the gateway:

| Gateway HTTP Status | Gateway Error Code | Rust LlmError Variant | UI Treatment |
|---------------------|-------------------|----------------------|-------------|
| 502 | `LLM_RATE_LIMITED` | `RateLimited { retry_after }` | Retry timer |
| 502 | `LLM_TIMEOUT` | `GatewayError` | "Taking too long" + retry |
| 502 | `PROVIDER_UNAVAILABLE` | `GatewayError` | "Service unavailable" + retry |
| 502 | `INVALID_RESPONSE` | `Validation` | Retry (LLM output issue) |
| 500 | `GATEWAY_INTERNAL` | `GatewayUnavailable` | "Internal error" |
| 503 | (connection refused) | `GatewayUnavailable` | Gateway restart triggered |

### Gateway Security Isolation

| Boundary | Enforcement |
|----------|-------------|
| API keys | Only in Python gateway environment variables. Never passed to Rust process. Never logged. |
| Network | Rust ↔ Gateway: localhost only (127.0.0.1). Gateway ↔ Provider: HTTPS with TLS validation. |
| Authentication | Shared secret (`GATEWAY_SECRET`) in `X-Gateway-Secret` header. Generated on startup or from env. |
| Memory | Separate processes. Rust crash ≠ Python crash. Python crash ≠ Rust crash. |
| Logs | Gateway logs to stdout/stderr, captured by Rust process. Rust redacts before writing to disk. |
| File system | Gateway has no access to learner data files. Only reads prompt templates if needed. |
| Process | Gateway runs as child process. Rust monitors PID. Killed on agent-core shutdown. |

### Environment Configuration

```bash
# .env (development, NEVER committed)
# Python LLM Gateway
OPENAI_API_KEY=sk-...           # Required if using OpenAI models
ANTHROPIC_API_KEY=sk-ant-...    # Required if using Anthropic models
GATEWAY_SECRET=dev-secret-change-in-prod

# Agent-core (Rust)
BLUP_LLM_GATEWAY_URL=http://127.0.0.1:9000
BLUP_LLM_GATEWAY_SECRET=dev-secret-change-in-prod
BLUP_LLM_MODEL=gpt-4o           # Default model

# Optional
BLUP_LLM_GATEWAY_PORT=9000      # Override default port
BLUP_LOG_FORMAT=json            # "json" or "pretty"
```

### Logging Coordination

Both processes use structured logging with correlated request IDs:

```
# Python gateway log (stdout, captured by Rust)
{"timestamp": "...", "level": "info", "request_id": "abc-123",
 "event": "llm_call", "provider": "openai", "model": "gpt-4o",
 "duration_ms": 1234, "prompt_tokens": 500, "completion_tokens": 200}

# Rust agent-core log (tracing)
{"timestamp": "...", "level": "INFO", "request_id": "abc-123",
 "session_id": "def-456", "state": "FEASIBILITY_CHECK",
 "event": "llm_call_complete", "provider": "openai", "model": "gpt-4o",
 "duration_ms": 1300, "tokens": 700, "validation_status": "passed"}
```

The `request_id` is generated by agent-core and passed to the gateway via HTTP header `X-Request-Id`. Both processes include it in their logs, enabling correlation.

### Development vs Production

| Aspect | Development | Production |
|--------|-------------|-----------|
| Gateway startup | `python -m uvicorn --reload` (hot reload) | `python -m uvicorn` (no reload) |
| Gateway port | 9000 | 9000 (configurable) |
| Logging | Pretty format, DEBUG level | JSON format, INFO level |
| API keys | From `.env` file | From environment (K8s secrets, systemd env) |
| Health check interval | 30 seconds | 10 seconds |
| Gateway restart on failure | Manual (`scripts/dev` restart) | Automatic (supervisor / K8s) |

### API Implementation Pattern

Using a helper for SSE streaming responses:

```rust
// api/goal.rs (conceptual pattern)
use axum::{
    extract::{Path, State},
    response::Sse,
    Json,
};

pub async fn submit_goal(
    State(app): State<AppState>,
    Path(session_id): Path<Uuid>,
    Json(body): Json<LearningGoal>,
) -> Result<Sse<impl Stream<Item = Result<Event, axum::Error>>>, AppError> {
    // 1. Load session
    let mut session = app.store.get(session_id).await?.ok_or(AppError::NotFound)?;

    // 2. Validate state transition
    session.state_machine.transition(Transition::SubmitGoal)?;
    session.goal = Some(body.clone());

    // 3. Render prompt
    let rendered_prompt = app.prompts.render("feasibility_check", 1, &hashmap! {
        "learning_goal" => body.description,
        "domain" => body.domain,
        "context" => body.context.unwrap_or_default(),
    })?;

    // 4. Call LLM
    let request = ChatCompletionRequest {
        model: app.config.llm_model.clone(),
        messages: vec![
            ChatMessage { role: "system".into(), content: rendered_prompt },
            ChatMessage { role: "user".into(), content: serde_json::to_string(&body)? },
        ],
        temperature: Some(0.3),
        max_tokens: Some(2000),
        stream: true,
        response_format: Some(ResponseFormat { format_type: "json_object".into() }),
    };

    // 5. Stream response
    let stream = app.llm.complete_stream(request).await?;
    let sse_stream = llm_stream_to_sse(stream, |full_text| {
        // On stream complete: validate against schema, update session state
        let result: FeasibilityResult = serde_json::from_str(&full_text)?;
        app.validator.validate(&result, "feasibility_result.v1")?;
        Ok(())
    });

    // 6. Save session
    session.updated_at = chrono::Utc::now();
    app.store.update(session).await?;

    Ok(Sse::new(sse_stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(Duration::from_secs(app.config.sse_ping_interval_secs))
    ))
}
```

### SSE Event Stream Builder

```rust
// Helper to build SSE events from LLM stream
use axum::response::sse::Event;

fn build_sse_events(stream: impl Stream<Item = LlmDelta>) -> impl Stream<Item = Result<Event, axum::Error>> {
    stream
        .map(|delta| {
            match delta {
                LlmDelta::Chunk { content, index } => {
                    Event::default()
                        .event("chunk")
                        .data(serde_json::json!({ "content": content, "index": index }).to_string())
                }
                LlmDelta::Status { state, message } => {
                    Event::default()
                        .event("status")
                        .data(serde_json::json!({ "state": state, "message": message }).to_string())
                }
                LlmDelta::Done { result } => {
                    Event::default()
                        .event("done")
                        .data(result)
                }
                LlmDelta::Ping => {
                    Event::default().event("ping").data("{}")
                }
            }
        })
        .map(Ok)
}
```

### HTTP API Routes

```rust
// server/router.rs (conceptual)
use axum::{routing::{get, post}, Router};

pub fn build_router(state: AppState) -> Router {
    Router::new()
        // Session
        .route("/api/session", post(api::session::create_session))
        // Learning flow
        .route("/api/session/:session_id/goal", post(api::goal::submit_goal))
        .route("/api/session/:session_id/profile/answer", post(api::profile::submit_answer))
        .route("/api/session/:session_id/curriculum", get(api::curriculum::get_curriculum))
        // Chapter
        .route("/api/session/:session_id/chapter/:chapter_id", get(api::chapter::start_chapter))
        .route("/api/session/:session_id/chapter/:chapter_id/ask", post(api::ask::ask_question))
        .route("/api/session/:session_id/chapter/:chapter_id/complete", post(api::complete::complete_chapter))
        // Health
        .route("/health", get(|| async { "ok" }))
        // Global middleware
        .layer(tower_http::cors::CorsLayer::permissive())  // Phase 1: permissive; tighten later
        .layer(tower_http::trace::TraceLayer::new_for_http())
        .with_state(state)
}
```

### Error Handling

```rust
// api/error.rs (conceptual)
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Session not found")]
    NotFound,

    #[error("Invalid state transition")]
    InvalidStateTransition(#[from] state::machine::StateError),

    #[error("LLM error: {0}")]
    LlmError(#[from] llm::client::LlmError),

    #[error("Schema validation error: {0}")]
    ValidationError(String),

    #[error("Prompt render error: {0}")]
    PromptError(#[from] prompts::renderer::RenderError),

    #[error("Session store error: {0}")]
    StoreError(#[from] state::session::SessionStoreError),

    #[error("JSON parse error: {0}")]
    JsonError(#[from] serde_json::Error),
}

impl axum::response::IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let (status, code, message) = match &self {
            AppError::NotFound => (StatusCode::NOT_FOUND, "NOT_FOUND", self.to_string()),
            AppError::InvalidStateTransition(_) => (StatusCode::CONFLICT, "INVALID_STATE_TRANSITION", self.to_string()),
            AppError::LlmError(e) => (StatusCode::BAD_GATEWAY, "LLM_ERROR", e.to_string()),
            AppError::ValidationError(_) => (StatusCode::UNPROCESSABLE_ENTITY, "VALIDATION_ERROR", self.to_string()),
            AppError::PromptError(_) => (StatusCode::INTERNAL_SERVER_ERROR, "PROMPT_ERROR", "Prompt rendering failed".into()),
            AppError::StoreError(_) => (StatusCode::INTERNAL_SERVER_ERROR, "STORE_ERROR", "Session store error".into()),
            AppError::JsonError(_) => (StatusCode::BAD_REQUEST, "INVALID_JSON", self.to_string()),
        };

        let body = serde_json::json!({
            "error": {
                "code": code,
                "message": message,
            }
        });

        (status, Json(body)).into_response()
    }
}
```

### Main Entry Point (Updated for Python LLM Gateway)

```rust
// main.rs (conceptual)
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use std::time::Duration;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 1. Initialize tracing
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| "agent_core=debug,tower_http=info".into()))
        .with(tracing_subscriber::fmt::layer().json())
        .init();

    // 2. Load configuration
    let config = config::Config::from_env();

    tracing::info!(
        host = %config.host,
        port = %config.port,
        model = %config.llm_model,
        gateway_url = %config.llm_gateway_url,
        "Starting agent-core"
    );

    // 3. Spawn Python LLM Gateway (if not already running externally)
    let gateway_handle = if config.gateway_managed {
        Some(spawn_python_gateway(&config)?)
    } else {
        None
    };

    // 4. Wait for gateway health
    wait_for_gateway_health(&config.llm_gateway_url, Duration::from_secs(10)).await?;
    tracing::info!("Python LLM Gateway healthy");

    // 5. Initialize services
    let store = state::session::InMemorySessionStore::new();
    let prompts = prompts::loader::PromptLoader::new(&config.prompts_dir)?;
    let validator = validation::schema_validator::SchemaValidator::new(&config.schemas_dir)?;
    let llm = llm::client::LlmClient::new(
        config.llm_gateway_url.clone(),
        config.llm_gateway_secret.clone(),
    );

    let app_state = AppState {
        config: config.clone(),
        store,
        prompts,
        validator,
        llm,
        gateway_handle,  // Held for shutdown
    };

    // 6. Build and start server
    let router = server::router::build_router(app_state);
    let addr = format!("{}:{}", config.host, config.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    tracing::info!("Listening on {}", addr);
    axum::serve(listener, router)
        .with_graceful_shutdown(async {
            tokio::signal::ctrl_c().await.ok();
            tracing::info!("Shutting down...");
            // Gateway child process is killed on Drop of gateway_handle
        })
        .await?;

    Ok(())
}

fn spawn_python_gateway(config: &Config) -> anyhow::Result<std::process::Child> {
    std::process::Command::new("python3")
        .args(["-m", "uvicorn", "services.llm_gateway.src.main:app"])
        .arg("--host").arg("127.0.0.1")
        .arg("--port").arg(config.llm_gateway_url.split(':').last().unwrap_or("9000"))
        .env("OPENAI_API_KEY", &config.openai_api_key)
        .env("ANTHROPIC_API_KEY", &config.anthropic_api_key)
        .env("GATEWAY_SECRET", &config.llm_gateway_secret)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .context("Failed to start Python LLM Gateway")
}
```

## Testing Strategy

| Test Category | Method | Description |
|---------------|--------|-------------|
| State machine transitions | Unit test | All valid transitions succeed; all invalid transitions return errors |
| Session CRUD | Unit test | Create, get, update, delete in-memory sessions |
| LLM client with mock | Integration test | Mock HTTP server returns predefined responses; verify parsing |
| SSE parsing | Unit test | Parse raw SSE text into events; handle partial chunks |
| Prompt loading | Unit test | Load templates, include partials, substitute variables |
| Schema validation | Unit test | Valid JSON passes; invalid JSON fails with specific error paths |
| API endpoints | Integration test | Axum test helpers; verify status codes, response bodies, state transitions |
| Error responses | Integration test | Verify structured error format for all error types |
| LLM retry logic | Unit test | Mock validation failure then success; verify retry count |
| Replay buffer | Unit test | SSE replay buffer holds correct number of events; Last-Event-ID recovery |

### Key Test Fixtures

1. **Mock LLM responses** — in `tests/fixtures/mock_llm_responses/`:
   - `feasibility_check_viable.json` — a valid feasibility result.
   - `feasibility_check_infeasible.json` — an infeasible result with suggestions.
   - `profile_question_round1.json` — first profile question.
   - `curriculum_plan_valid.json` — a complete curriculum plan.
   - `chapter_content_valid.json` — chapter with Markdown content.
   - `malformed_json.txt` — LLM returns invalid JSON.
   - `empty_response.txt` — LLM returns empty body.

2. **Mock SSE streams** — in `tests/fixtures/mock_sse_streams/`:
   - `normal_stream.txt` — chunk, chunk, status, done.
   - `error_mid_stream.txt` — chunk, chunk, error.
   - `empty_stream.txt` — done immediately (no chunks).

## Logging

```rust
// Example structured log events
tracing::info!(
    request_id = %request_id,
    session_id = %session_id,
    state = %session.state_machine.current_state(),
    event = "state_transition",
    from = %from_state,
    to = %to_state,
    transition = ?transition,
    duration_ms = %elapsed_ms,
);

tracing::info!(
    request_id = %request_id,
    session_id = %session_id,
    model = %config.llm_model,
    prompt_tokens = %usage.prompt_tokens,
    completion_tokens = %usage.completion_tokens,
    total_tokens = %usage.total_tokens,
    retry_count = %retry_count,
    validation_status = "passed",
    duration_ms = %llm_duration_ms,
    event = "llm_call_complete",
);
// Redacted: API key, full prompt content, full response content, private user data
```

## Quality Gates

- [ ] `cargo fmt --check` passes
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` passes
- [ ] `cargo test` passes all unit and integration tests
- [ ] All API endpoints respond correctly (status, headers, body shape)
- [ ] All valid state transitions work; all invalid transitions return structured errors
- [ ] SSE streams deliver all event types (chunk, status, error, done, ping)
- [ ] LLM mock tests cover success, validation failure, retry, and streaming
- [ ] Schema validator catches malformed JSON, missing fields, and wrong types
- [ ] Prompt loader detects missing variables and missing partials
- [ ] Structured logs include all required fields
- [ ] No API keys in logs or error responses
- [ ] No internal stack traces in error responses
- [ ] Server starts within 2 seconds on developer hardware

## Monitoring and Metrics

### Prometheus Metrics

Agent-core exposes metrics at `GET /metrics` in Prometheus text format:

```rust
// observability/metrics.rs (conceptual)
use metrics::{counter, gauge, histogram};
use metrics_exporter_prometheus::PrometheusBuilder;

pub fn init_metrics() {
    PrometheusBuilder::new()
        .with_http_listener(([127, 0, 0, 1], 9090))  // Separate port from main API
        .install()
        .expect("Failed to install Prometheus recorder");
}

// ── Session metrics ──
pub fn record_session_created() {
    counter!("blup_sessions_created_total").increment(1);
    gauge!("blup_sessions_active").increment(1.0);
}

pub fn record_session_destroyed() {
    gauge!("blup_sessions_active").decrement(1.0);
}

// ── State transition metrics ──
pub fn record_state_transition(from: &str, to: &str, duration_ms: u64) {
    counter!("blup_state_transitions_total", "from" => from.to_string(), "to" => to.to_string())
        .increment(1);
    histogram!("blup_state_transition_duration_ms", "transition" => format!("{}→{}", from, to))
        .record(duration_ms as f64);
}

// ── LLM call metrics ──
pub fn record_llm_call(provider: &str, model: &str, duration_ms: u64,
                        prompt_tokens: u64, completion_tokens: u64,
                        status: &str) {
    counter!("blup_llm_calls_total", "provider" => provider.to_string(),
             "model" => model.to_string(), "status" => status.to_string())
        .increment(1);
    histogram!("blup_llm_call_duration_ms", "provider" => provider.to_string())
        .record(duration_ms as f64);
    counter!("blup_llm_prompt_tokens_total", "model" => model.to_string())
        .increment(prompt_tokens);
    counter!("blup_llm_completion_tokens_total", "model" => model.to_string())
        .increment(completion_tokens);
}

pub fn record_llm_retry(attempt: u32) {
    counter!("blup_llm_retries_total", "attempt" => attempt.to_string()).increment(1);
}

// ── SSE connection metrics ──
pub fn record_sse_connection_opened() {
    gauge!("blup_sse_connections_active").increment(1.0);
}

pub fn record_sse_connection_closed(duration_ms: u64) {
    gauge!("blup_sse_connections_active").decrement(1.0);
    histogram!("blup_sse_connection_duration_ms").record(duration_ms as f64);
}

// ── Schema validation metrics ──
pub fn record_schema_validation(schema: &str, passed: bool, duration_ms: u64) {
    let status = if passed { "passed" } else { "failed" };
    counter!("blup_schema_validations_total", "schema" => schema.to_string(), "status" => status.to_string())
        .increment(1);
    histogram!("blup_schema_validation_duration_ms", "schema" => schema.to_string())
        .record(duration_ms as f64);
}

// ── Gateway metrics ──
pub fn record_gateway_health_check(passed: bool) {
    let status = if passed { "healthy" } else { "unhealthy" };
    gauge!("blup_gateway_healthy").set(if passed { 1.0 } else { 0.0 });
}
```

### Key Metrics Reference

| Metric | Type | Labels | Description |
|--------|------|--------|-------------|
| `blup_sessions_active` | Gauge | — | Current active session count |
| `blup_sessions_created_total` | Counter | — | Total sessions created |
| `blup_state_transitions_total` | Counter | from, to | State machine transitions |
| `blup_state_transition_duration_ms` | Histogram | transition | Time per transition |
| `blup_llm_calls_total` | Counter | provider, model, status | LLM API calls |
| `blup_llm_call_duration_ms` | Histogram | provider | LLM round-trip time |
| `blup_llm_prompt_tokens_total` | Counter | model | Prompt tokens consumed |
| `blup_llm_completion_tokens_total` | Counter | model | Completion tokens generated |
| `blup_llm_retries_total` | Counter | attempt | LLM retry count |
| `blup_sse_connections_active` | Gauge | — | Open SSE connections |
| `blup_schema_validations_total` | Counter | schema, status | Schema validation results |
| `blup_schema_validation_duration_ms` | Histogram | schema | Validation time |
| `blup_gateway_healthy` | Gauge | — | 1 = healthy, 0 = unhealthy |
| `blup_http_requests_total` | Counter | method, path, status | HTTP request count |
| `blup_http_request_duration_ms` | Histogram | method, path | HTTP request latency |

### Alert Rules

```yaml
# prometheus/alerts.yml
groups:
  - name: blup-agent-core
    rules:
      - alert: GatewayUnhealthy
        expr: blup_gateway_healthy == 0
        for: 1m
        severity: critical
        annotations:
          summary: "Python LLM Gateway is unhealthy"
          description: "Gateway health check has been failing for 1 minute. All LLM calls will fail."

      - alert: HighLLMErrorRate
        expr: rate(blup_llm_calls_total{status="error"}[5m]) / rate(blup_llm_calls_total[5m]) > 0.2
        for: 5m
        severity: warning
        annotations:
          summary: "LLM error rate > 20% over 5 minutes"
          description: "{{ $value | humanizePercentage }} of LLM calls are failing."

      - alert: HighLLMLatency
        expr: histogram_quantile(0.95, rate(blup_llm_call_duration_ms_bucket[5m])) > 30000
        for: 10m
        severity: warning
        annotations:
          summary: "P95 LLM latency > 30s"
          description: "LLM calls are taking > 30 seconds at P95. Check provider status."

      - alert: SchemaValidationFailureRate
        expr: rate(blup_schema_validations_total{status="failed"}[5m]) > 5
        for: 5m
        severity: warning
        annotations:
          summary: "Schema validation failures detected"
          description: "LLM is producing schema-invalid output. May indicate prompt regression."

      - alert: HighActiveSessions
        expr: blup_sessions_active > 800
        for: 5m
        severity: warning
        annotations:
          summary: "High session count: {{ $value }}"
          description: "Approaching max sessions limit (1000). Consider scaling."

      - alert: SSEConnectionLeak
        expr: blup_sse_connections_active > blup_sessions_active * 2
        for: 10m
        severity: warning
        annotations:
          summary: "SSE connection count exceeds 2× session count"
          description: "Possible SSE connection leak. Orphaned connections detected."

      - alert: ServiceDown
        expr: up{job="blup-agent-core"} == 0
        for: 1m
        severity: critical
        annotations:
          summary: "Agent-core is down"
          description: "The service is not responding to health checks."
```

### Dashboard Design

```
┌─────────────────────────────────────────────────────────┐
│  Blup Agent-Core Dashboard                               │
├────────────────────┬────────────────────────────────────┤
│                    │                                    │
│  Active Sessions   │  LLM Call Rate (by provider)       │
│  [gauge: 42]       │  [stacked bar: openai vs anthropic]│
│                    │                                    │
├────────────────────┼────────────────────────────────────┤
│                    │                                    │
│  State Distribution│  LLM Latency (P50/P95/P99)         │
│  [pie: idle,       │  [line chart, 1h window]           │
│   goal_input,      │                                    │
│   feasibility, ...]│                                    │
│                    │                                    │
├────────────────────┼────────────────────────────────────┤
│                    │                                    │
│  SSE Connections   │  Schema Validation (pass/fail)     │
│  [line chart]      │  [stacked bar by schema]           │
│                    │                                    │
├────────────────────┴────────────────────────────────────┤
│                                                         │
│  Recent Errors                                           │
│  [table: timestamp, error_code, session_id, message]     │
│                                                         │
└─────────────────────────────────────────────────────────┘
```

### Troubleshooting Runbooks

#### Runbook 1: Gateway Unhealthy

**Alert:** `GatewayUnhealthy` (blup_gateway_healthy == 0)

**Symptoms:**
- All LLM calls failing with `LlmError::GatewayUnavailable`
- `curl http://127.0.0.1:9000/health` returns error or timeout
- Agent-core logs show "Gateway unhealthy after N consecutive failures"

**Diagnosis:**
```bash
# 1. Check if gateway process is running
ps aux | grep uvicorn | grep llm_gateway

# 2. Check gateway logs
journalctl -u blup-llm-gateway -n 50

# 3. Check if port is in use
lsof -i :9000

# 4. Verify API keys are valid
curl -H "Authorization: Bearer $OPENAI_API_KEY" https://api.openai.com/v1/models
curl -H "x-api-key: $ANTHROPIC_API_KEY" https://api.anthropic.com/v1/messages
```

**Resolution:**
1. If process is dead: restart gateway (`systemctl restart blup-llm-gateway` or respawn via agent-core)
2. If API key invalid: update `.env` and restart
3. If port conflict: kill conflicting process or change `BLUP_LLM_GATEWAY_PORT`
4. If out of memory: increase container memory limit; check for memory leak in gateway

#### Runbook 2: High LLM Error Rate

**Alert:** `HighLLMErrorRate` (error rate > 20% over 5 minutes)

**Diagnosis:**
```bash
# Check error breakdown by provider
curl http://localhost:9090/metrics | grep blup_llm_calls_total

# Check gateway logs for specific errors
journalctl -u blup-llm-gateway --since "5 min ago" | grep ERROR

# Test with a simple completion
curl -X POST http://127.0.0.1:9000/v1/gateway/complete \
  -H "X-Gateway-Secret: $GATEWAY_SECRET" \
  -H "Content-Type: application/json" \
  -d '{"model":"gpt-4o-mini","messages":[{"role":"user","content":"Hello"}],"max_tokens":10}'
```

**Resolution:**
1. If `LLM_RATE_LIMITED` errors: reduce request rate or upgrade API tier
2. If `LLM_TIMEOUT` errors: check provider status page (status.openai.com, status.anthropic.com)
3. If `INVALID_RESPONSE` errors: check schema validation error details; may indicate prompt regression
4. If only one provider failing: switch default model to healthy provider temporarily

#### Runbook 3: High LLM Latency

**Alert:** `HighLLMLatency` (P95 > 30s)

**Diagnosis:**
```bash
# Check latency by model
curl http://localhost:9090/metrics | grep blup_llm_call_duration_ms

# Check if specific model is slow
curl -X POST http://127.0.0.1:9000/v1/gateway/complete \
  -H "X-Gateway-Secret: $GATEWAY_SECRET" \
  -d '{"model":"gpt-4o","messages":[{"role":"user","content":"test"}],"max_tokens":5}'
```

**Resolution:**
1. If all models slow: check network connectivity to provider APIs
2. If specific model slow: switch to faster model (e.g., gpt-4o → gpt-4o-mini) via config
3. If max_tokens very high: reduce default max_tokens for the affected prompt type
4. If concurrent requests high: check `blup_sessions_active` gauge; add connection pooling

#### Runbook 4: Schema Validation Spike

**Alert:** `SchemaValidationFailureRate` (failures > 5/minute)

**Diagnosis:**
```bash
# Check which schema is failing
curl http://localhost:9090/metrics | grep blup_schema_validations_total | grep failed

# Check recent prompt changes
git log --oneline --since="1 hour ago" -- prompts/

# Review LLM raw output (TEMPORARILY enable debug logging)
# WARNING: debug mode may log PII — disable immediately after diagnosis
```

**Resolution:**
1. If single schema failing: check prompt template for that schema — may have drifted
2. If all schemas failing: check if LLM model changed or provider API changed
3. Roll back recent prompt changes
4. Run `prompt-tester test-all --gateway` to identify failing prompts

#### Runbook 5: Memory Growth

**Symptom:** Agent-core RSS growing steadily (detected via process monitoring, not Prometheus alert)

**Diagnosis:**
```bash
# Check session count
curl http://localhost:9090/metrics | grep blup_sessions_active

# Check for leaked SSE connections
curl http://localhost:9090/metrics | grep blup_sse_connections_active

# Check process memory
ps -o pid,rss,vsz,comm -p $(pgrep agent-core)
```

**Resolution:**
1. If sessions growing: check for sessions never transitioning to COMPLETED; add session TTL
2. If SSE connections > sessions: check for clients not closing connections; add idle timeout
3. If steady growth with stable sessions: possible memory leak — take heap profile, restart
4. Phase 1 mitigation: restart agent-core (in-memory sessions will be lost; acceptable for Phase 1)

#### Runbook 6: Startup Failure

**Symptom:** Agent-core exits immediately after start with error

**Diagnosis:**
```bash
# Check startup validation
./scripts/bootstrap

# Check environment variables
env | grep BLUP_

# Check port availability
lsof -i :3000

# Check prompts directory
ls prompts/*.v1.prompt.md

# Check schemas directory
ls schemas/*.v1.schema.json
```

**Resolution:**
1. Missing API keys: set `OPENAI_API_KEY` and/or `ANTHROPIC_API_KEY` in `.env`
2. Port in use: change `BLUP_PORT` or kill conflicting process
3. Missing prompts: ensure `BLUP_PROMPTS_DIR` points to correct directory
4. Missing schemas: ensure `BLUP_SCHEMAS_DIR` points to correct directory
5. Gateway won't start: check Python venv, check `requirements.txt` installed

### Cost Tracking

```rust
// observability/cost_tracker.rs (conceptual)
pub struct CostTracker {
    // Approximate costs per 1M tokens (USD), updated periodically
    model_costs: HashMap<String, ModelCost>,
}

struct ModelCost {
    prompt_per_mtok: f64,      // Cost per 1M prompt tokens
    completion_per_mtok: f64,  // Cost per 1M completion tokens
}

impl CostTracker {
    pub fn estimate_cost(&self, model: &str, prompt_tokens: u64, completion_tokens: u64) -> f64 {
        let costs = self.model_costs.get(model).unwrap_or(&ModelCost {
            prompt_per_mtok: 0.0,
            completion_per_mtok: 0.0,
        });
        (prompt_tokens as f64 / 1_000_000.0) * costs.prompt_per_mtok
            + (completion_tokens as f64 / 1_000_000.0) * costs.completion_per_mtok
    }

    pub fn default_costs() -> HashMap<String, ModelCost> {
        let mut costs = HashMap::new();
        costs.insert("gpt-4o".into(), ModelCost { prompt_per_mtok: 2.50, completion_per_mtok: 10.00 });
        costs.insert("gpt-4o-mini".into(), ModelCost { prompt_per_mtok: 0.15, completion_per_mtok: 0.60 });
        costs.insert("claude-sonnet-4-6".into(), ModelCost { prompt_per_mtok: 3.00, completion_per_mtok: 15.00 });
        costs.insert("claude-haiku-4-5".into(), ModelCost { prompt_per_mtok: 0.25, completion_per_mtok: 1.25 });
        costs
    }
}
```

## Data Privacy and PII Handling

### Data Classification

| Category | Examples | Storage | Logging | Retention |
|----------|---------|---------|---------|-----------|
| **Public** | Chapter content, exercise questions | Database | Full | Indefinite |
| **Internal** | Session IDs, state machine states, token counts | Database | Full | 90 days |
| **Sensitive** | Learning goals, user profile answers, chat messages | Database | Redacted | Session lifetime + 30 days |
| **Restricted** | API keys, gateway secrets, email addresses | Never stored | NEVER logged | N/A |

### PII Detection and Redaction

Agent-core scans all user-provided text for potential PII before logging or storing:

```rust
// privacy/pii_detector.rs (conceptual)
use regex::Regex;
use once_cell::sync::Lazy;

static EMAIL_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}").unwrap()
});
static PHONE_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\+?[\d]{1,4}?[-.\s]?\(?\d{1,4}\)?[-.\s]?\d{1,9}[-.\s]?\d{1,9}").unwrap()
});
static API_KEY_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)(sk-[a-zA-Z0-9]{20,}|sk-ant-[a-zA-Z0-9_-]{20,}|AIza[a-zA-Z0-9_-]{30,})").unwrap()
});
static IP_ADDR_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\b\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}\b").unwrap()
});
static CREDIT_CARD_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\b\d{4}[- ]?\d{4}[- ]?\d{4}[- ]?\d{4}\b").unwrap()
});

pub struct PiiDetector;

impl PiiDetector {
    /// Check if a string contains potential PII.
    pub fn contains_pii(text: &str) -> bool {
        EMAIL_RE.is_match(text)
            || PHONE_RE.is_match(text)
            || API_KEY_RE.is_match(text)
            || CREDIT_CARD_RE.is_match(text)
    }

    /// Redact PII from a string, replacing with type markers.
    pub fn redact(text: &str) -> String {
        let mut result = text.to_string();
        result = EMAIL_RE.replace_all(&result, "[EMAIL_REDACTED]").to_string();
        result = PHONE_RE.replace_all(&result, "[PHONE_REDACTED]").to_string();
        result = API_KEY_RE.replace_all(&result, "[API_KEY_REDACTED]").to_string();
        result = IP_ADDR_RE.replace_all(&result, "[IP_REDACTED]").to_string();
        result = CREDIT_CARD_RE.replace_all(&result, "[CARD_REDACTED]").to_string();
        result
    }

    /// Check if user-provided content is safe to log (no PII found).
    pub fn safe_to_log(text: &str) -> bool {
        !Self::contains_pii(text)
    }
}
```

### Log Redaction Middleware

All log events pass through a redaction layer before being written:

```rust
// observability/redaction.rs (conceptual)
use tracing_subscriber::layer::Layer;
use std::collections::HashSet;

/// Fields that must never appear in logs. If detected, the entire log
/// event is downgraded to a redacted summary.
const REDACTED_FIELD_NAMES: &[&str] = &[
    "api_key", "secret", "password", "token", "credential",
    "user_email", "user_phone", "user_address",
    "raw_prompt", "raw_response",  // Full prompt/response content
    "learner_input",               // Raw learner text (may contain PII)
];

/// Fields that are truncated or hashed in logs.
const HASHED_FIELD_NAMES: &[&str] = &[
    "session_id",   // Log full for debugging, hash for analytics
];

struct LogRedactionLayer;

impl<S: tracing::Subscriber> Layer<S> for LogRedactionLayer {
    fn on_event(&self, event: &tracing::Event<'_>, _ctx: tracing_subscriber::layer::Context<'_, S>) {
        // Check all field names against REDACTED_FIELD_NAMES
        // If a restricted field is found:
        //   1. Replace value with "[REDACTED]"
        //   2. Record a metric: blup_log_redactions_total
        // If PII detected in a string field value:
        //   1. Run PiiDetector::redact() on the value
        //   2. Record a metric: blup_pii_detections_total
    }
}
```

### Session Data Lifecycle

```
Session Created
  │
  ├─ Session data stored in memory (Phase 1) or SQLite (Phase 2)
  │
  ├─ Session ACTIVE: data retained in full
  │
  ├─ Session COMPLETED: data retained for 30 days
  │    (Learner can export their data during this period)
  │
  ├─ After 30 days:
  │    ├─ Chat messages: anonymized (session_id removed, timestamps rounded to day)
  │    ├─ Profile data: deleted
  │    ├─ Curriculum + progress: anonymized, kept for aggregate analytics
  │    └─ API keys, secrets: NEVER stored
  │
  └─ After 90 days: all session data deleted
```

### GDPR / Privacy Compliance Checklist

| Requirement | Implementation | Status |
|-------------|---------------|--------|
| Data minimization | Only collect profile fields necessary for curriculum personalization | Phase 1 design |
| Purpose limitation | Learning goal + profile used ONLY for teaching; never for other purposes | Enforced in prompts |
| Right to access | `GET /api/session/{id}/export` returns all learner data as JSON | Phase 2 |
| Right to deletion | `DELETE /api/session/{id}` purges all session data immediately | Phase 2 |
| Data portability | Export format is structured JSON matching schemas | Phase 2 |
| Consent | Phase 1: implicit (single-user, no auth). Phase 2+: explicit consent screen | Phase 2 |
| Breach notification | All PII detections in logs trigger an alert within 1 hour | Phase 2 |
| Data residency | Phase 1: localhost only. Phase 2+: configurable storage region | Phase 2 |
| Cookie consent | Phase 1: session_id only (functional cookie, no consent needed) | Phase 1 |
| Privacy policy | Document linked from Web UI footer | Phase 1 |

### Data Export Format

```json
{
  "exported_at": "2025-06-01T12:00:00Z",
  "session": {
    "id": "uuid",
    "state": "COMPLETED",
    "created_at": "...",
    "updated_at": "..."
  },
  "learning_goal": { "...": "..." },
  "user_profile": { "...": "..." },
  "curriculum": { "...": "..." },
  "chapters_progress": [ { "...": "..." } ],
  "messages": [ { "...": "..." } ]
}
```

## Risk Register

| Risk | Impact | Mitigation |
|------|--------|------------|
| LLM returns invalid JSON despite JSON mode | Flow broken, UX dead end | Retry with validation error feedback; fallback to asking LLM to fix |
| In-memory sessions lost on restart | Learner loses all progress | JSON file snapshot every N minutes; clear documentation that Phase 1 is ephemeral |
| SSE connection leaks | Memory exhaustion on many concurrent sessions | Connection tracking with timeout; max concurrent sessions limit |
| Prompt filesystem reads block async runtime | Increased latency | Load and cache at startup; watch for changes in dev mode |
| `jsonschema` crate performance on large responses | Slow validation | Validate only required fields; streaming validation for large content |
| reqwest connection pool exhaustion | LLM calls fail under load | Connection pool tuning; circuit breaker for gateway |
| Python gateway process crash | All LLM calls fail | Health monitor with auto-restart; agent-core detects unhealthy → restart gateway |
| Gateway and agent-core config drift | Gateway uses different API keys than intended | Single .env file as config source; startup validation checks provider health |

## Deployment

### Development (Single Machine)

```bash
# Terminal 1: Start Python LLM Gateway
cd services/llm-gateway
pip install -r requirements.txt
python -m uvicorn src.main:app --host 127.0.0.1 --port 9000 --reload

# Terminal 2: Start agent-core
cd crates/agent-core
cargo run -- --port 3000

# Terminal 3: Start web UI
cd apps/web-ui
npm run dev
```

### Docker Compose (Recommended for Phase 1)

```yaml
# docker-compose.yml (Phase 1)
version: "3.9"
services:
  llm-gateway:
    build:
      context: ./services/llm-gateway
      dockerfile: Dockerfile
    ports:
      - "127.0.0.1:9000:9000"
    environment:
      - OPENAI_API_KEY=${OPENAI_API_KEY}
      - ANTHROPIC_API_KEY=${ANTHROPIC_API_KEY}
      - GATEWAY_SECRET=${GATEWAY_SECRET}
      - LOG_FORMAT=json
    restart: unless-stopped
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:9000/health"]
      interval: 10s
      timeout: 5s
      retries: 3
    # No network access except to external AI providers (handled by Docker DNS)

  agent-core:
    build:
      context: ./crates/agent-core
      dockerfile: Dockerfile
    ports:
      - "127.0.0.1:3000:3000"
    environment:
      - BLUP_LLM_GATEWAY_URL=http://llm-gateway:9000
      - BLUP_LLM_GATEWAY_SECRET=${GATEWAY_SECRET}
      - BLUP_LLM_MODEL=${BLUP_LLM_MODEL:-gpt-4o}
      - BLUP_LOG_FORMAT=json
      - BLUP_PROMPTS_DIR=/app/prompts
      - BLUP_SCHEMAS_DIR=/app/schemas
    volumes:
      - ./prompts:/app/prompts:ro
      - ./schemas:/app/schemas:ro
    depends_on:
      llm-gateway:
        condition: service_healthy
    restart: unless-stopped
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:3000/health"]
      interval: 15s
      timeout: 5s
      retries: 3

  web-ui:
    build:
      context: ./apps/web-ui
      dockerfile: Dockerfile
    ports:
      - "127.0.0.1:5173:80"
    environment:
      - VITE_API_URL=http://agent-core:3000
    depends_on:
      - agent-core
```

### Python LLM Gateway Dockerfile

```dockerfile
# services/llm-gateway/Dockerfile
FROM python:3.12-slim

# Create non-root user
RUN useradd --create-home --shell /bin/bash gateway && \
    mkdir -p /app && chown gateway:gateway /app

WORKDIR /app

# Install dependencies (cached layer)
COPY requirements.txt .
RUN pip install --no-cache-dir -r requirements.txt

# Copy application code
COPY src/ ./src/

# Switch to non-root user
USER gateway

# Health check
HEALTHCHECK --interval=10s --timeout=5s --retries=3 \
    CMD python -c "import urllib.request; urllib.request.urlopen('http://localhost:9000/health')"

# Run the gateway
EXPOSE 9000
CMD ["python", "-m", "uvicorn", "src.main:app", "--host", "0.0.0.0", "--port", "9000", "--no-access-log"]
```

### Agent-Core Dockerfile

```dockerfile
# crates/agent-core/Dockerfile — Multi-stage Rust build
FROM rust:1.80-slim AS builder

WORKDIR /app
COPY crates/agent-core/ ./crates/agent-core/
COPY schemas/ ./schemas/
COPY prompts/ ./prompts/

RUN cargo build --release -p agent-core

FROM debian:bookworm-slim

RUN useradd --create-home --shell /bin/bash blup && \
    apt-get update && apt-get install -y --no-install-recommends ca-certificates curl && \
    rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/agent-core /usr/local/bin/agent-core
COPY prompts/ /app/prompts/
COPY schemas/ /app/schemas/

USER blup
WORKDIR /app

HEALTHCHECK --interval=15s --timeout=5s --retries=3 \
    CMD curl -f http://localhost:3000/health || exit 1

EXPOSE 3000
ENV BLUP_PROMPTS_DIR=/app/prompts
ENV BLUP_SCHEMAS_DIR=/app/schemas
ENV BLUP_LLM_GATEWAY_URL=http://llm-gateway:9000

CMD ["agent-core"]
```

### Performance Budgets

| Metric | Phase 1 Target | Measurement |
|--------|---------------|-------------|
| Agent-core startup time | < 3s (including gateway health check) | `time cargo run` |
| Gateway health check latency | < 100ms (localhost) | `curl -o /dev/null -w '%{time_total}' localhost:9000/health` |
| LLM round-trip (gateway overhead) | < 50ms added to provider latency | Gateway timing log |
| Schema validation | < 10ms per validation | `tracing` span duration |
| Prompt render | < 5ms per render | `tracing` span duration |
| SSE event delivery | < 100ms from LLM chunk to client | End-to-end timing |
| Memory (idle) | < 100MB (Rust) + < 100MB (Python) | Process RSS |
| Memory (under load, 50 sessions) | < 500MB (Rust) + < 200MB (Python) | Process RSS |

### Environment Validation at Startup

Agent-core validates its environment before accepting requests:

```rust
// startup validation checklist:
// 1. [ ] prompts_dir exists and contains all 5 Phase 1 templates
// 2. [ ] schemas_dir exists and contains all 7 Phase 1 schemas
// 3. [ ] Python gateway is reachable at llm_gateway_url/health
// 4. [ ] Gateway reports at least one provider as healthy (/health/providers)
// 5. [ ] llm_gateway_secret is non-empty (refuse to start with default)
// 6. [ ] llm_model is supported by at least one gateway provider
// 7. [ ] Port is available (bind check before starting Axum)

// If any check fails, agent-core exits with a clear diagnostic message
// and exit code 1. It never starts in a degraded state.
```

### Phase 1 → Phase 2 Migration Path

| Component | Phase 1 | Phase 2 | Migration |
|-----------|---------|---------|-----------|
| Session storage | InMemorySessionStore | SQLite via `storage` crate | Replace trait implementation; no API change |
| LLM calls | Python gateway (basic) | Python gateway (with caching, circuit breaker) | Gateway upgrade; Rust client unchanged |
| Prompts | 5 templates | 5 + assessment generation + evaluation | Add new templates; existing ones continue |
| Schemas | 7 schemas | 7 + AssessmentSpec, Exercise, etc. | Add new schemas; no breaking changes to v1 |
| State machine | 8 states | 8 states (extended with exercise states) | Add states; existing transitions unchanged |
| API endpoints | 7 endpoints | 7 + exercise submit + progress | Add routes; existing endpoints continue |
