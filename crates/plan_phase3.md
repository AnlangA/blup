# Crates Module — Phase 3: Plugin Host, Tool Router, Bevy Protocol

## Module Overview

Phase 3 adds the plugin and rendering infrastructure. Three new crates join the workspace: `plugin-host` manages plugin lifecycle and permissions, `tool-router` dispatches tool requests to sandboxes and plugins, and `bevy-protocol` bridges Agent Core with the Bevy viewer.

## Phase 3 Scope

| Crate | Purpose | Status |
|-------|---------|--------|
| `agent-core` | Core orchestration (continues), now delegates to plugin-host, tool-router, bevy-protocol | Evolving |
| `plugin-host` | Plugin lifecycle, permission engine, capability routing | Planned |
| `tool-router` | Tool dispatch, sandbox request routing, capability-to-tool mapping | Planned |
| `bevy-protocol` | SceneSpec ↔ Bevy ECS protocol, camera/input events, rendering commands | Planned |

## Crate: plugin-host

### Purpose

Manage the lifecycle of domain-specific learning plugins. Load manifests, enforce permissions, route capability calls, and isolate plugin processes.

### File Structure

```
crates/plugin-host/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── config.rs                  # Plugin directory, timeout, resource limits
│   ├── manifest/
│   │   ├── mod.rs
│   │   ├── loader.rs             # Load and validate plugin manifests
│   │   └── validator.rs          # Manifest schema validation, dependency check
│   ├── lifecycle/
│   │   ├── mod.rs
│   │   ├── manager.rs            # Plugin lifecycle state machine
│   │   ├── states.rs             # Loaded, Init, Active, Executing, Paused, Error, Unloaded
│   │   └── process_manager.rs    # Start/stop plugin processes
│   ├── permissions/
│   │   ├── mod.rs
│   │   ├── engine.rs             # Permission check engine
│   │   ├── policy.rs             # Permission policies
│   │   └── types.rs              # Permission enum
│   ├── runtime/
│   │   ├── mod.rs
│   │   ├── http_microservice.rs  # Plugin as HTTP microservice
│   │   ├── stdin_stdout.rs       # Plugin over stdin/stdout
│   │   └── wasm.rs               # Future: WASM runtime (wasmtime)
│   ├── capability/
│   │   ├── mod.rs
│   │   ├── router.rs             # Route capability calls to correct plugin
│   │   └── types.rs              # PluginRequest, PluginResponse
│   ├── models/
│   │   ├── mod.rs
│   │   └── types.rs              # Plugin, PluginState, PluginConfig
│   └── error.rs
└── tests/
    ├── manifest_validation_test.rs
    ├── lifecycle_test.rs
    ├── permission_test.rs
    ├── capability_routing_test.rs
    └── isolation_test.rs
```

### Plugin Registry

```rust
// models/types.rs (conceptual)
use std::collections::HashMap;
use tokio::sync::RwLock;
use std::sync::Arc;

pub struct PluginHost {
    plugins: Arc<RwLock<HashMap<String, LoadedPlugin>>>,
    permission_engine: PermissionEngine,
    lifecycle_manager: LifecycleManager,
    config: PluginHostConfig,
}

pub struct LoadedPlugin {
    pub manifest: PluginManifest,
    pub state: PluginLifecycleState,
    pub process: Option<PluginProcess>,
    pub health_check_url: Option<String>,
    pub loaded_at: DateTime<Utc>,
    pub metrics: PluginMetrics,
}

pub struct PluginHostConfig {
    pub plugins_dir: PathBuf,            // plugins/ directory
    pub plugin_start_timeout_secs: u64,  // 30
    pub health_check_interval_secs: u64, // 10
    pub max_restart_attempts: u32,       // 3
    pub plugin_port_range: (u16, u16),   // (9000, 9999)
}
```

### Lifecycle Manager

```rust
// lifecycle/manager.rs (conceptual)
impl LifecycleManager {
    pub async fn load(&self, manifest_path: &Path) -> Result<LoadedPlugin, PluginError> {
        // 1. Read and validate manifest
        // 2. Check dependencies (system tools, schema files)
        // 3. Allocate port (HTTP microservice mode)
        // 4. Create plugin instance
        // 5. State: Loaded
    }

    pub async fn init(&self, plugin_id: &str) -> Result<(), PluginError> {
        // 1. Start plugin process
        // 2. Wait for health check to pass
        // 3. State: Init → Initialized
    }

    pub async fn activate(&self, plugin_id: &str) -> Result<(), PluginError> {
        // 1. Enable permissions declared in manifest
        // 2. Start health check polling
        // 3. State: Initialized → Active
    }

    pub async fn execute_capability(
        &self,
        plugin_id: &str,
        request: PluginRequest,
    ) -> Result<PluginResponse, PluginError> {
        // 1. Verify plugin is Active
        // 2. Check permission for this capability
        // 3. Route request to plugin (HTTP POST or stdin)
        // 4. Enforce timeout
        // 5. Collect and validate response
        // 6. State: Active → Executing → Active
    }

    pub async fn pause(&self, plugin_id: &str) -> Result<(), PluginError> {
        // 1. Stop sending new capability requests
        // 2. Wait for in-flight requests to complete (with timeout)
        // 3. Send pause signal to plugin
        // 4. State: Active → Paused
    }

    pub async fn unload(&self, plugin_id: &str) -> Result<(), PluginError> {
        // 1. Pause if active
        // 2. Send shutdown signal to plugin process
        // 3. Force kill if unresponsive after timeout
        // 4. Release port
        // 5. Remove from registry
        // 6. State: * → Unloaded
    }
}
```

### Permission Engine

```rust
// permissions/engine.rs (conceptual)
impl PermissionEngine {
    pub fn check(
        &self,
        plugin: &LoadedPlugin,
        capability_id: &str,
        session_id: &Uuid,
    ) -> Result<PermissionDecision, PermissionError> {
        // 1. Find capability in manifest
        let capability = plugin.manifest.capabilities
            .iter()
            .find(|c| c.id == capability_id)
            .ok_or(PermissionError::UnknownCapability)?;

        // 2. Check each required permission against manifest
        for permission in &capability.required_permissions {
            if !plugin.manifest.permissions.contains(permission) {
                return Ok(PermissionDecision::Denied {
                    reason: format!("Plugin lacks permission: {}", permission),
                });
            }

            // 3. For high-risk permissions, check user consent
            if permission.is_high_risk() && !self.has_user_consent(session_id, permission)? {
                return Ok(PermissionDecision::ConsentRequired {
                    permission: permission.clone(),
                });
            }
        }

        // 4. Check rate limits
        // 5. Log decision
        Ok(PermissionDecision::Granted)
    }
}

pub enum PermissionDecision {
    Granted,
    Denied { reason: String },
    ConsentRequired { permission: Permission },
}
```

### HTTP Microservice Runtime

```rust
// runtime/http_microservice.rs (conceptual)
pub struct HttpPluginRuntime {
    client: reqwest::Client,
    base_url: String,   // http://127.0.0.1:{port}
}

impl HttpPluginRuntime {
    pub async fn health_check(&self) -> Result<bool, PluginError> {
        let response = self.client
            .get(format!("{}/health", self.base_url))
            .timeout(Duration::from_secs(5))
            .send()
            .await?;
        Ok(response.status().is_success())
    }

    pub async fn call_capability(
        &self,
        capability_id: &str,
        request: &PluginRequest,
    ) -> Result<PluginResponse, PluginError> {
        let response = self.client
            .post(format!("{}/capability/{}", self.base_url, capability_id))
            .json(request)
            .timeout(Duration::from_secs(30))
            .send()
            .await?;

        let body: PluginResponse = response.json().await?;
        Ok(body)
    }
}
```

### Process Isolation

```rust
// lifecycle/process_manager.rs (conceptual)
impl ProcessManager {
    pub fn spawn_plugin(&self, plugin: &LoadedPlugin) -> Result<Child, PluginError> {
        // Spawn as a child process:
        // - Working directory: plugin's directory
        // - No environment variables from parent (clean env)
        // - Only pass: PORT (if HTTP), PLUGIN_ID, SESSION_TOKEN (short-lived)
        // - stdout/stderr piped for logging
        // - Resource limits via cgroups (Linux) or setrlimit

        let mut cmd = std::process::Command::new("python");
        cmd.arg(&plugin.manifest.runtime.entrypoint)
            .env_clear()
            .env("PORT", port.to_string())
            .env("PLUGIN_ID", &plugin.manifest.plugin_id)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        // Security: no shell, no PATH, no HOME
        // Plugin only sees its own directory

        cmd.spawn().map_err(PluginError::ProcessSpawn)
    }
}
```

## Crate: tool-router

### Purpose

Dispatch tool requests from Agent Core to the correct executor: sandbox for code execution, math engine for computation, plugins for domain-specific tools. Provides a unified tool-calling interface.

### File Structure

```
crates/tool-router/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── config.rs                # Tool registry, routing rules
│   ├── router.rs                # Main router: ToolRequest → executor
│   ├── tools/
│   │   ├── mod.rs
│   │   ├── sandbox.rs          # Route to sandbox (code execution)
│   │   ├── math.rs             # Route to math engine
│   │   ├── typst.rs            # Route to Typst compiler
│   │   └── plugin.rs           # Route to plugin capability
│   ├── models/
│   │   ├── mod.rs
│   │   ├── tool_request.rs
│   │   ├── tool_result.rs
│   │   └── tool_registry.rs
│   └── error.rs
└── tests/
    ├── router_test.rs
    └── tool_registry_test.rs
```

### Unified Tool Interface

```rust
// models/tool_request.rs (conceptual)
pub struct ToolRequest {
    pub request_id: Uuid,
    pub session_id: Uuid,
    pub tool_kind: ToolKind,
    pub parameters: serde_json::Value,
    pub limits: Option<ToolLimits>,
}

pub enum ToolKind {
    CodeExecution { language: String },
    MathEvaluation { engine: MathEngine },
    TypstCompilation,
    PluginCapability { plugin_id: String, capability_id: String },
}

pub enum MathEngine {
    SymPy,
    SageMath,
    NumericApproximation,
}

pub struct ToolLimits {
    pub timeout_secs: u64,
    pub memory_mb: u64,
    pub network_enabled: bool,
}

pub struct ToolResult {
    pub request_id: Uuid,
    pub status: ToolStatus,
    pub output: serde_json::Value,
    pub duration_ms: u64,
    pub resource_usage: Option<ResourceUsage>,
}

pub enum ToolStatus {
    Success,
    Timeout,
    ResourceExceeded { resource: String },
    ExecutionError { code: String, message: String },
    PermissionDenied,
    ToolUnavailable,
}
```

### Router

```rust
// router.rs (conceptual)
pub struct ToolRouter {
    sandbox_manager: Option<Arc<SandboxManager>>,   // None if sandbox not available
    plugin_host: Option<Arc<PluginHost>>,            // None if plugin host not available
    tool_registry: ToolRegistry,
}

impl ToolRouter {
    pub async fn route(&self, request: ToolRequest) -> Result<ToolResult, ToolRouterError> {
        // 1. Look up tool in registry
        let tool_def = self.tool_registry.lookup(&request.tool_kind)?;

        // 2. Check permissions
        if !tool_def.is_available() {
            return Err(ToolRouterError::ToolUnavailable);
        }

        // 3. Route to executor
        match &request.tool_kind {
            ToolKind::CodeExecution { language } => {
                let sandbox = self.sandbox_manager.as_ref()
                    .ok_or(ToolRouterError::ToolUnavailable)?;
                self.execute_in_sandbox(sandbox, request, language).await
            }
            ToolKind::MathEvaluation { engine } => {
                self.execute_math(request, engine).await
            }
            ToolKind::TypstCompilation => {
                let sandbox = self.sandbox_manager.as_ref()
                    .ok_or(ToolRouterError::ToolUnavailable)?;
                self.execute_typst(sandbox, request).await
            }
            ToolKind::PluginCapability { plugin_id, capability_id } => {
                let host = self.plugin_host.as_ref()
                    .ok_or(ToolRouterError::ToolUnavailable)?;
                self.execute_plugin_capability(host, plugin_id, capability_id, request).await
            }
        }
    }

    async fn execute_in_sandbox(
        &self,
        sandbox: &SandboxManager,
        request: ToolRequest,
        language: &str,
    ) -> Result<ToolResult, ToolRouterError> {
        // Validate language is supported
        let supported = ["python", "javascript", "rust"];
        if !supported.contains(&language) {
            return Err(ToolRouterError::UnsupportedLanguage(language.to_string()));
        }

        // Extract code from parameters; validate non-empty
        let code = request.parameters.get("code")
            .and_then(|v| v.as_str())
            .filter(|s| !s.trim().is_empty())
            .ok_or(ToolRouterError::InvalidParameters("'code' is required and must be non-empty".into()))?;

        // Enforce max code size (prevent abuse)
        if code.len() > 100_000 {
            return Err(ToolRouterError::InvalidParameters("Code exceeds maximum size of 100KB".into()));
        }

        let sandbox_request = SandboxRequest {
            request_id: request.request_id,
            session_id: request.session_id,
            tool_kind: ToolKind::CodeExecution,
            code: code.to_string(),
            language: language.to_string(),
            stdin: request.parameters.get("stdin").and_then(|v| v.as_str()).map(String::from),
            limits: request.limits.map(Into::into).unwrap_or_default(),
        };

        let start = std::time::Instant::now();
        let result = sandbox.execute(sandbox_request).await?;

        Ok(ToolResult {
            request_id: request.request_id,
            status: map_sandbox_status(result.status),
            output: serde_json::json!({
                "stdout": result.stdout,
                "stderr": result.stderr,
                "exit_code": result.exit_code,
                "stdout_truncated": result.stdout_truncated,
                "stderr_truncated": result.stderr_truncated,
            }),
            duration_ms: start.elapsed().as_millis() as u64,
            resource_usage: Some(ResourceUsage {
                peak_memory_mb: result.resource_usage.peak_memory_mb,
                cpu_time_ms: result.resource_usage.cpu_time_ms,
            }),
        })
    }

    async fn execute_math(
        &self,
        request: ToolRequest,
        engine: &MathEngine,
    ) -> Result<ToolResult, ToolRouterError> {
        let expression = request.parameters.get("expression")
            .and_then(|v| v.as_str())
            .ok_or(ToolRouterError::InvalidParameters("'expression' is required".into()))?;

        let operation = request.parameters.get("operation")
            .and_then(|v| v.as_str())
            .unwrap_or("evaluate");

        // Route to local SymPy or sandbox-based SageMath
        match engine {
            MathEngine::SymPy => {
                // Local SymPy execution (Python subprocess, not Docker)
                let output = tokio::task::spawn_blocking(move || {
                    Self::run_sympy(expression, operation)
                }).await??;
                Ok(output)
            }
            MathEngine::SageMath => {
                // SageMath runs in sandbox (heavier)
                let sandbox = self.sandbox_manager.as_ref()
                    .ok_or(ToolRouterError::ToolUnavailable)?;
                self.execute_math_in_sandbox(sandbox, expression, operation).await
            }
            MathEngine::NumericApproximation => {
                // Fast numeric eval (local)
                let output = Self::numeric_approx(expression);
                Ok(output)
            }
        }
    }

    fn run_sympy(expression: &str, operation: &str) -> Result<ToolResult, ToolRouterError> {
        let script = format!(
            "from sympy import *; x=symbols('x'); import json; \
             expr = {}; \
             result = {}(expr); \
             print(json.dumps({{'result': str(result), 'latex': latex(result)}}))",
            expression,
            match operation {
                "evaluate" => "N",
                "diff" => "diff",
                "integrate" => "integrate",
                "solve" => "solve",
                "simplify" => "simplify",
                _ => return Err(ToolRouterError::InvalidParameters(format!("Unknown math operation: {}", operation))),
            }
        );

        let output = std::process::Command::new("python3")
            .args(["-c", &script])
            .output()
            .map_err(|e| ToolRouterError::ExecutionError(e.to_string()))?;

        let result: serde_json::Value = serde_json::from_slice(&output.stdout)
            .map_err(|e| ToolRouterError::ExecutionError(format!("Failed to parse SymPy output: {}", e)))?;

        Ok(ToolResult {
            request_id: Uuid::new_v4(),
            status: ToolStatus::Success,
            output: result,
            duration_ms: 0,
            resource_usage: None,
        })
    }

    async fn execute_plugin_capability(
        &self,
        host: &PluginHost,
        plugin_id: &str,
        capability_id: &str,
        request: ToolRequest,
    ) -> Result<ToolResult, ToolRouterError> {
        // Verify plugin is loaded and active
        let plugin = host.get(plugin_id)
            .ok_or(ToolRouterError::PluginNotFound(plugin_id.to_string()))?;

        if plugin.state != PluginLifecycleState::Active {
            return Err(ToolRouterError::PluginNotActive {
                plugin_id: plugin_id.to_string(),
                state: format!("{:?}", plugin.state),
            });
        }

        // Delegate to plugin host
        let plugin_request = PluginRequest {
            request_id: request.request_id,
            session_id: request.session_id,
            capability_id: capability_id.to_string(),
            parameters: request.parameters,
            context: request.context,
        };

        let response = host.execute_capability(plugin_id, plugin_request).await?;

        Ok(ToolResult {
            request_id: request.request_id,
            status: map_plugin_status(&response.status),
            output: response.result,
            duration_ms: response.metadata.duration_ms,
            resource_usage: None,
        })
    }
}

fn map_sandbox_status(status: ExecutionStatus) -> ToolStatus {
    match status {
        ExecutionStatus::Success => ToolStatus::Success,
        ExecutionStatus::TimeoutCompile | ExecutionStatus::TimeoutRun => ToolStatus::Timeout,
        ExecutionStatus::MemoryExceeded | ExecutionStatus::CpuExceeded
            | ExecutionStatus::DiskExceeded => ToolStatus::ResourceExceeded {
                resource: format!("{:?}", status)
            },
        ExecutionStatus::NonZeroExit => ToolStatus::ExecutionError {
            code: "NON_ZERO_EXIT".into(),
            message: "Process exited with non-zero status".into(),
        },
        ExecutionStatus::NetworkBlocked => ToolStatus::PermissionDenied,
        ExecutionStatus::InternalError => ToolStatus::ExecutionError {
            code: "INTERNAL".into(),
            message: "Internal sandbox error".into(),
        },
    }
}
```

### Tool Registry

```rust
// models/tool_registry.rs (conceptual)
pub struct ToolRegistry {
    tools: HashMap<String, ToolDefinition>,
}

pub struct ToolDefinition {
    pub kind: ToolKind,
    pub name: String,
    pub description: String,
    pub required_permissions: Vec<Permission>,
    pub availability: ToolAvailability,
}

pub enum ToolAvailability {
    Always,            // Math evaluation
    SandboxRequired,   // Code execution, Typst
    PluginRequired,    // Domain-specific tools
}

impl ToolRegistry {
    pub fn default() -> Self {
        let mut registry = Self { tools: HashMap::new() };

        registry.register(ToolDefinition {
            kind: ToolKind::MathEvaluation { engine: MathEngine::SymPy },
            name: "math_eval".into(),
            description: "Evaluate mathematical expressions symbolically".into(),
            required_permissions: vec![Permission::ToolMath],
            availability: ToolAvailability::Always,
        });

        registry.register(ToolDefinition {
            kind: ToolKind::CodeExecution { language: "python".into() },
            name: "python_exec".into(),
            description: "Execute Python code in a sandbox".into(),
            required_permissions: vec![Permission::ToolCodeRun],
            availability: ToolAvailability::SandboxRequired,
        });

        // ... Typst, other code languages

        registry
    }
}
```

## Crate: bevy-protocol

### Purpose

Bridge between Agent Core and the Bevy viewer. Translates `SceneSpec` JSON into Bevy ECS commands, handles camera and input event synchronization, and manages rendering commands.

### File Structure

```
crates/bevy-protocol/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── config.rs                # Protocol configuration
│   ├── scene/
│   │   ├── mod.rs
│   │   ├── spec.rs             # SceneSpec definition (mirrors schema)
│   │   ├── builder.rs          # SceneSpec → Bevy commands
│   │   └── validator.rs        # Validate SceneSpec before building
│   ├── camera/
│   │   ├── mod.rs
│   │   └── commands.rs         # Camera position, orbit, zoom commands
│   ├── input/
│   │   ├── mod.rs
│   │   └── events.rs           # Input event forwarding (mouse, keyboard, touch)
│   ├── render/
│   │   ├── mod.rs
│   │   └── commands.rs         # Render commands (capture screenshot, etc.)
│   ├── models/
│   │   ├── mod.rs
│   │   ├── scene_spec.rs
│   │   ├── render_command.rs
│   │   └── scene_event.rs      # Events sent back from Bevy (click, hover, animation done)
│   └── error.rs
└── tests/
    ├── scene_builder_test.rs
    └── spec_validation_test.rs
```

### Scene Builder

```rust
// scene/builder.rs (conceptual)
use bevy::prelude::*;

pub struct SceneBuilder;

impl SceneBuilder {
    /// Build a Bevy scene from a SceneSpec.
    /// This runs inside the Bevy app, not in Agent Core.
    pub fn build(world: &mut World, spec: SceneSpec) -> Result<(), BuildError> {
        // 1. Validate spec (no dangling references, valid geometry)
        // 2. Set up camera
        // 3. Set up lights
        // 4. Spawn entities with appropriate components
        // 5. Attach interaction handlers
        // 6. Start animations

        for entity_spec in &spec.entities {
            let mut entity = world.spawn_empty();

            // Transform
            entity.insert(Transform::from_xyz(
                entity_spec.position.x,
                entity_spec.position.y,
                entity_spec.position.z,
            ));

            // Renderable → Mesh + Material
            if let Some(renderable) = &entity_spec.renderable {
                match renderable.shape {
                    Shape::Sphere { radius } => {
                        entity.insert(Mesh3d(meshes.add(Sphere::new(radius))));
                    }
                    Shape::Cube { size } => {
                        entity.insert(Mesh3d(meshes.add(Cuboid::new(size, size, size))));
                    }
                    Shape::Cylinder { radius, height } => {
                        entity.insert(Mesh3d(meshes.add(Cylinder::new(radius, height))));
                    }
                    // ... other shapes
                }

                entity.insert(MeshMaterial3d(materials.add(StandardMaterial {
                    base_color: parse_color(&renderable.color)?,
                    ..default()
                })));
            }

            // Interactable → Interaction component
            if let Some(interactable) = &entity_spec.interactable {
                entity.insert(Interactable {
                    on_click: interactable.on_click.clone(),
                    on_hover: interactable.on_hover.clone(),
                });
            }

            // Label
            if let Some(label) = &entity_spec.label {
                entity.insert(Name::new(label.clone()));
            }
        }

        Ok(())
    }
}
```

### Protocol Communication

Agent Core communicates with the Bevy viewer through two channels:

1. **Commands (Core → Bevy):** Load a new scene, update scene entities, render screenshot.
2. **Events (Bevy → Core):** Entity clicked, hover state changed, animation completed, user interaction occurred.

```rust
// models/render_command.rs (conceptual)
pub enum RenderCommand {
    LoadScene(SceneSpec),
    UpdateEntities(Vec<EntityUpdate>),
    SetCamera(CameraCommand),
    CaptureScreenshot { width: u32, height: u32 },
    ClearScene,
}

pub enum SceneEvent {
    EntityClicked { entity_id: String },
    EntityHovered { entity_id: String },
    EntityUnhovered { entity_id: String },
    AnimationCompleted { animation_id: String },
    SceneLoaded { scene_id: Uuid },
    Error { message: String },
}
```

### Cargo Dependencies

```toml
# bevy-protocol/Cargo.toml
[dependencies]
bevy = { version = "0.15", default-features = false, features = ["bevy_asset", "bevy_scene", "bevy_pbr"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
uuid = { version = "1", features = ["v4", "serde"] }
thiserror = "1"
tracing = "0.1"
glam = "0.28"  # Vec3, Quat math types, shared with Bevy
```

## Cross-Crate Integration

### Phase 3 agent-core Changes

```rust
// agent-core now composes all Phase 3 services:

pub struct AppState {
    pub config: Config,
    pub store: Arc<Storage>,               // Phase 2
    pub prompts: PromptLoader,              // Phase 1
    pub validator: SchemaValidator,         // Phase 1
    pub llm: LlmGateway,                    // Phase 2
    pub assessment: AssessmentEngine,       // Phase 2
    pub content: ContentPipeline,           // Phase 2.5
    pub plugin_host: PluginHost,            // Phase 3
    pub tool_router: ToolRouter,            // Phase 3
    pub bevy_protocol: BevyProtocol,        // Phase 3
}
```

### New API Endpoints

| Method | Path | Purpose |
|--------|------|---------|
| `GET` | `/api/session/{id}/plugins` | List available plugins |
| `POST` | `/api/session/{id}/plugin/{plugin_id}/capability/{cap_id}` | Call plugin capability |
| `GET` | `/api/session/{id}/scene/{scene_id}` | Get scene specification |
| `POST` | `/api/session/{id}/scene/{scene_id}/interact` | Send interaction event to scene |
| `GET` | `/api/session/{id}/tools` | List available tools |
| `POST` | `/api/tools/call` | Call a tool through tool-router |

## Testing Strategy

| Test Category | Crate | Method |
|---------------|-------|--------|
| Manifest loading | plugin-host | Valid/invalid manifests; schema validation |
| Lifecycle transitions | plugin-host | All state transitions; invalid transitions |
| Permission denial | plugin-host | Request capability without permission → denied |
| Plugin crash recovery | plugin-host | Kill plugin process → detect → restart or error |
| Tool routing | tool-router | Request → correct executor; unavailable → error |
| Sandbox tool execution | tool-router | Code exec request → sandbox → result |
| Scene building | bevy-protocol | SceneSpec → Bevy commands (headless test) |
| Scene event handling | bevy-protocol | Mock entity click → SceneEvent |

## Quality Gates

- [ ] Plugin manifests validate against schema
- [ ] All lifecycle transitions work correctly
- [ ] Permission engine denies unauthorized capabilities
- [ ] Plugin crash is detected and handled
- [ ] Plugin process is fully isolated (no host filesystem access)
- [ ] Tool router dispatches to correct executor
- [ ] SceneSpec produces valid Bevy entities
- [ ] No circular dependencies in crate graph
- [ ] All crates pass `cargo fmt`, `cargo clippy`, `cargo test`
