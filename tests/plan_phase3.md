# Tests Module — Phase 3: Plugin System and Bevy Scene Tests

## Module Overview

Phase 3 adds tests for the plugin system, tool router, and Bevy scene protocol. These tests verify plugin isolation, permission enforcement, lifecycle management, tool dispatch, and scene rendering correctness.

## Phase 3 Test Scope

| Test Category | Purpose | Coverage Target |
|---------------|---------|-----------------|
| Plugin manifest tests | Manifest validation, schema compliance | All manifest variations |
| Plugin lifecycle tests | Load, init, activate, execute, pause, unload | All lifecycle transitions |
| Plugin permission tests | Permission denial, consent requirements | All permissions |
| Plugin isolation tests | Filesystem, network, process isolation | All isolation boundaries |
| Plugin crash tests | Crash detection, recovery, cleanup | Crash in each lifecycle state |
| Tool router tests | Correct dispatch to sandbox, plugin, or math engine | All tool kinds |
| Bevy scene tests | SceneSpec loading, entity creation, camera setup | All scene types |
| Bevy protocol tests | Command/event serialization, scene lifecycle | All commands/events |

## File Structure

```
tests/
├── plugin/
│   ├── mod.rs
│   ├── manifest_validation_test.rs
│   ├── lifecycle_test.rs
│   ├── permission_test.rs
│   ├── isolation_test.rs
│   ├── capability_routing_test.rs
│   ├── crash_recovery_test.rs
│   ├── concurrent_plugins_test.rs
│   └── fixtures/
│       ├── valid-plugin/
│       │   ├── manifest.v1.json
│       │   └── src/main.py
│       ├── no-manifest-plugin/
│       │   └── src/main.py
│       ├── invalid-manifest-plugin/
│       │   └── manifest.v1.json    # Missing required fields
│       ├── bad-permissions-plugin/
│       │   └── manifest.v1.json    # Declares forbidden permissions
│       └── crashing-plugin/
│           ├── manifest.v1.json
│           └── src/main.py         # Crashes on startup
├── tool_router/
│   ├── mod.rs
│   ├── routing_test.rs
│   ├── sandbox_dispatch_test.rs
│   ├── math_tool_test.rs
│   └── unavailable_tool_test.rs
└── bevy/
    ├── mod.rs
    ├── scene_spec_validation_test.rs
    ├── scene_builder_test.rs
    ├── camera_controls_test.rs
    ├── interaction_test.rs
    └── fixtures/
        ├── molecule-scene.json
        ├── invalid-scene-missing-entity.json
        └── large-scene-1000-entities.json
```

## Plugin Tests

### Manifest Validation Tests

```rust
// plugin/manifest_validation_test.rs (conceptual)
#[tokio::test]
async fn test_valid_manifest_passes_validation() {
    let host = PluginHost::new(test_config());
    let result = host.load_manifest("tests/plugin/fixtures/valid-plugin/manifest.v1.json").await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_missing_manifest_file_returns_error() {
    let host = PluginHost::new(test_config());
    let result = host.load_manifest("tests/plugin/fixtures/no-manifest-plugin/manifest.v1.json").await;
    assert!(result.is_err());
    // Error should mention that the file doesn't exist
}

#[tokio::test]
async fn test_manifest_missing_required_fields_returns_specific_error() {
    let result = host.load_manifest("tests/plugin/fixtures/invalid-manifest-plugin/manifest.v1.json").await;
    let err = result.unwrap_err();
    assert!(err.to_string().contains("plugin_id"));
    assert!(err.to_string().contains("version"));
}

#[tokio::test]
async fn test_manifest_with_unknown_capability_fails() {
    // Capability references a schema that doesn't exist
}

#[tokio::test]
async fn test_manifest_with_forbidden_permission_fails() {
    // Manifest declares "direct:filesystem" → should be rejected
    let result = host.load_manifest("tests/plugin/fixtures/bad-permissions-plugin/manifest.v1.json").await;
    assert!(result.is_err());
}
```

### Lifecycle Tests

```rust
// plugin/lifecycle_test.rs (conceptual)
#[tokio::test]
async fn test_full_lifecycle_load_to_unload() {
    let host = PluginHost::new(test_config());
    let plugin_id = "valid-plugin";

    // Load
    let plugin = host.load(plugin_id).await.unwrap();
    assert_eq!(plugin.state, PluginLifecycleState::Loaded);

    // Init
    host.init(plugin_id).await.unwrap();
    let plugin = host.get(plugin_id).unwrap();
    assert_eq!(plugin.state, PluginLifecycleState::Initialized);

    // Activate
    host.activate(plugin_id).await.unwrap();
    let plugin = host.get(plugin_id).unwrap();
    assert_eq!(plugin.state, PluginLifecycleState::Active);

    // Execute a capability
    let response = host.execute_capability(plugin_id, test_request()).await.unwrap();
    assert_eq!(response.status, "success");

    // Pause
    host.pause(plugin_id).await.unwrap();
    let plugin = host.get(plugin_id).unwrap();
    assert_eq!(plugin.state, PluginLifecycleState::Paused);

    // Unload
    host.unload(plugin_id).await.unwrap();
    assert!(host.get(plugin_id).is_none()); // Removed from registry
}

#[tokio::test]
async fn test_cannot_execute_on_paused_plugin() {
    // Activate → Pause → try Execute → should fail
}

#[tokio::test]
async fn test_can_activate_from_paused() {
    // Activate → Pause → Activate → Execute → should work
}

#[tokio::test]
async fn test_invalid_lifecycle_transition_returns_error() {
    // Try to Activate before Init → error
    let host = PluginHost::new(test_config());
    host.load("valid-plugin").await.unwrap();
    // Skip Init
    let result = host.activate("valid-plugin").await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Invalid transition"));
}
```

### Permission Tests

```rust
// plugin/permission_test.rs (conceptual)
#[tokio::test]
async fn test_capability_with_granted_permission_succeeds() {
    let host = setup_plugin_with_permissions(&["generate:content"]).await;
    let result = host.execute_capability("test-plugin", PluginRequest {
        capability_id: "generate:content".into(),
        ..default()
    }).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_capability_without_required_permission_is_denied() {
    let host = setup_plugin_with_permissions(&["generate:content"]).await;

    // Request a capability that requires tool:code_run (not granted)
    let result = host.execute_capability("test-plugin", PluginRequest {
        capability_id: "request:tool".into(),
        parameters: json!({"tool": "code_run"}),
        ..default()
    }).await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("permission"));
}

#[tokio::test]
async fn test_high_risk_permission_requires_user_consent() {
    let host = setup_plugin_with_permissions(&["read:user_profile"]).await;

    // Without consent flag
    let result = host.execute_capability("test-plugin", PluginRequest {
        capability_id: "read:user_profile".into(),
        ..default()
    }).await;

    // Should require consent
    assert!(result.is_err() || matches!(result.unwrap().status, "consent_required"));
}

#[tokio::test]
async fn test_forbidden_capability_always_denied() {
    // direct:filesystem is hard-coded denied, even if manifest claims it
}
```

### Isolation Tests

```rust
// plugin/isolation_test.rs (conceptual)
#[tokio::test]
async fn test_plugin_cannot_access_filesystem() {
    let host = setup_active_plugin("filesystem-test-plugin").await;

    // Plugin tries to read /etc/passwd via its capability
    let result = host.execute_capability("filesystem-test-plugin", PluginRequest {
        capability_id: "read_file".into(),
        parameters: json!({"path": "/etc/passwd"}),
        ..default()
    }).await;

    // Should be denied at the permission level (plugin has no fs permission)
    assert!(result.is_err());
}

#[tokio::test]
async fn test_plugin_cannot_make_external_network_requests() {
    // Plugin process has no network access (except localhost to Core)
    // Test by having plugin try to connect to external IP
    // This is enforced at the OS/process level, not just permission level
}

#[tokio::test]
async fn test_plugin_cannot_access_other_plugin_process() {
    // Plugin A should not be able to communicate with Plugin B's process
}
```

### Crash Recovery Tests

```rust
// plugin/crash_recovery_test.rs (conceptual)
#[tokio::test]
async fn test_plugin_crash_during_execution_is_detected() {
    let host = setup_active_plugin("crashing-plugin").await;

    let result = host.execute_capability("crashing-plugin", PluginRequest {
        capability_id: "trigger_crash".into(),
        ..default()
    }).await;

    assert!(result.is_err());
    // Plugin should be in Error state
    let plugin = host.get("crashing-plugin").unwrap();
    assert_eq!(plugin.state, PluginLifecycleState::Error);
}

#[tokio::test]
async fn test_plugin_can_restart_after_crash() {
    let host = setup_active_plugin("crashing-plugin").await;

    // Trigger crash
    let _ = host.execute_capability("crashing-plugin", crash_request()).await;

    // Restart: Init → Activate
    host.init("crashing-plugin").await.unwrap();
    host.activate("crashing-plugin").await.unwrap();

    // Should work again
    let result = host.execute_capability("crashing-plugin", valid_request()).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_plugin_crash_does_not_affect_other_plugins() {
    let host = PluginHost::new(test_config());
    host.load_and_activate("plugin-a").await.unwrap();
    host.load_and_activate("plugin-b").await.unwrap();

    // Crash plugin A
    let _ = host.execute_capability("plugin-a", crash_request()).await;

    // Plugin B should still work
    let result = host.execute_capability("plugin-b", valid_request()).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_plugin_exceeding_restart_limit_is_unloaded() {
    let config = PluginHostConfig { max_restart_attempts: 2, ..default() };
    let host = PluginHost::new(config);
    host.load_and_activate("crashing-plugin").await.unwrap();

    // Crash 3 times
    for _ in 0..3 {
        let _ = host.execute_capability("crashing-plugin", crash_request()).await;
        if host.get("crashing-plugin").is_none() {
            break; // Unloaded as expected
        }
        // Try to restart
        let _ = host.init("crashing-plugin").await;
        let _ = host.activate("crashing-plugin").await;
    }

    // After 3 crashes, plugin should be unloaded
    assert!(host.get("crashing-plugin").is_none());
}
```

## Tool Router Tests

```rust
// tool_router/routing_test.rs (conceptual)
#[tokio::test]
async fn test_code_execution_routes_to_sandbox() {
    let router = ToolRouter::new()
        .with_sandbox(mock_sandbox())
        .build();

    let result = router.route(ToolRequest {
        tool_kind: ToolKind::CodeExecution { language: "python".into() },
        parameters: json!({"code": "print(1+1)"}),
        ..default()
    }).await.unwrap();

    assert_eq!(result.status, ToolStatus::Success);
    assert_eq!(result.output["stdout"], "2\n");
}

#[tokio::test]
async fn test_math_eval_routes_to_math_engine() {
    let router = ToolRouter::new().build(); // Math engine doesn't need sandbox

    let result = router.route(ToolRequest {
        tool_kind: ToolKind::MathEvaluation { engine: MathEngine::SymPy },
        parameters: json!({"expression": "integrate(x**2, x)"}),
        ..default()
    }).await.unwrap();

    assert_eq!(result.status, ToolStatus::Success);
    assert_eq!(result.output["result"], "x**3/3");
}

#[tokio::test]
async fn test_unavailable_tool_returns_error() {
    let router = ToolRouter::new().build(); // No sandbox registered

    let result = router.route(ToolRequest {
        tool_kind: ToolKind::CodeExecution { language: "python".into() },
        ..default()
    }).await;

    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), ToolRouterError::ToolUnavailable));
}

#[tokio::test]
async fn test_plugin_capability_routes_to_plugin_host() {
    let router = ToolRouter::new()
        .with_plugin_host(mock_plugin_host())
        .build();

    let result = router.route(ToolRequest {
        tool_kind: ToolKind::PluginCapability {
            plugin_id: "math-engine".into(),
            capability_id: "generate:math_exercise".into(),
        },
        parameters: json!({"topic": "algebra"}),
        ..default()
    }).await.unwrap();

    assert_eq!(result.status, ToolStatus::Success);
}

#[tokio::test]
async fn test_tool_permission_denied() {
    // Request tool that requires permission the session doesn't have
}
```

## Bevy Scene Tests

```rust
// bevy/scene_builder_test.rs (conceptual)
#[test]
fn test_load_valid_scene_creates_entities() {
    let spec: SceneSpec = serde_json::from_str(
        include_str!("fixtures/molecule-scene.json")
    ).unwrap();

    let mut app = App::new();
    app.add_plugins(MinimalPlugins);

    SceneBuilder::build(app.world_mut(), spec).unwrap();

    // Verify entities were created
    let entity_count = app.world().query::<Entity>().iter(app.world()).count();
    assert!(entity_count > 0);

    // Verify specific entities: oxygen atom
    let oxygen = app.world().query_filtered::<Entity, With<Name>>()
        .iter(app.world())
        .find(|e| {
            app.world().get::<Name>(*e).map(|n| n.as_str() == "O").unwrap_or(false)
        });
    assert!(oxygen.is_some());
}

#[test]
fn test_invalid_scene_spec_returns_error() {
    let spec = serde_json::from_str::<SceneSpec>(
        include_str!("fixtures/invalid-scene-missing-entity.json")
    ).unwrap();

    let mut app = App::new();
    app.add_plugins(MinimalPlugins);

    let result = SceneBuilder::build(app.world_mut(), spec);
    assert!(result.is_err());
}

#[test]
fn test_large_scene_builds_within_time_budget() {
    // 1000 entities should build in < 500ms
    let spec = generate_large_scene(1000);
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);

    let start = std::time::Instant::now();
    SceneBuilder::build(app.world_mut(), spec).unwrap();
    let duration = start.elapsed();

    assert!(duration.as_millis() < 500, "Scene building took {:?}", duration);
}

#[test]
fn test_clear_scene_removes_all_entities() {
    // Load scene → clear → verify no scene entities remain
}

#[test]
fn test_scene_entities_have_correct_transforms() {
    let spec = create_test_spec_with_known_positions();
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    SceneBuilder::build(app.world_mut(), spec).unwrap();

    // Check oxygen is at origin
    let oxygen_pos = get_entity_position(&app, "O");
    assert_eq!(oxygen_pos, Vec3::ZERO);

    // Check hydrogen is at the right offset
    let h1_pos = get_entity_position(&app, "H1");
    assert!((h1_pos - Vec3::new(0.8, 0.6, 0.0)).length() < 0.01);
}
```

### Bevy Protocol Tests

```rust
// bevy/interaction_test.rs (conceptual)
#[test]
fn test_entity_click_produces_event() {
    // Simulate click on entity → verify SceneEvent::EntityClicked is emitted
}

#[test]
fn test_camera_orbit_command_updates_transform() {
    // Send orbit command → verify camera transform updates correctly
}
```

## Coverage Targets

| Area | Phase 3 Target |
|------|----------------|
| Plugin manifest validation | 100% of validation rules |
| Plugin lifecycle transitions | 100% of transitions |
| Plugin permission enforcement | 100% of permissions |
| Plugin crash recovery | All crash scenarios |
| Tool router dispatch | 100% of tool kinds |
| SceneSpec loading | All scene types + error cases |
| Overall plugin system coverage | ≥80% |

## Quality Gates

- [ ] All plugin manifest validation rules are tested
- [ ] All lifecycle state transitions are tested (valid + invalid)
- [ ] Permission engine denies all forbidden capabilities
- [ ] Plugin crash is detected within 5 seconds
- [ ] Plugin crash does not affect other plugins or Core
- [ ] Plugin exceeding restart limit is unloaded
- [ ] Tool router dispatches correctly to all executor types
- [ ] Unavailable tools return clear errors
- [ ] SceneSpec loading produces correct Bevy entities
- [ ] Scene clearing removes all entities
- [ ] All tests use mock/synthetic fixtures — no real plugins with real API keys
