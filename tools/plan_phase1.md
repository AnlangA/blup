# Tools Module — Phase 1: Schema Validator

## Module Overview

Phase 1 adds the `schema-validator` tool — a Rust binary that validates JSON Schema files for syntax correctness and validates fixture files against their schemas. This is the programmatic equivalent of the Phase 0 shell-based `schema-check` script, providing proper JSON Schema validation using the `jsonschema` crate.

## Phase 1 Deliverable

| Deliverable | Description | Status |
|-------------|-------------|--------|
| `tools/schema-validator/` | Rust CLI that validates schemas and fixtures | Planned |

## File Structure

```
tools/schema-validator/
├── Cargo.toml
├── src/
│   ├── main.rs              # CLI entry point (clap)
│   ├── validator.rs         # Schema validation logic
│   ├── fixture.rs           # Fixture loading and validation
│   ├── reporter.rs          # Output formatting (text, JSON)
│   └── error.rs             # Error types
└── tests/
    ├── validator_test.rs
    ├── fixture_test.rs
    └── fixtures/
        ├── valid-schema.schema.json
        ├── invalid-schema.schema.json
        ├── valid-fixture.json
        └── invalid-fixture.json
```

## CLI Specification

```
schema-validator 0.1.0
Blup schema validation tool

USAGE:
    schema-validator <COMMAND>

COMMANDS:
    validate     Validate schemas and fixtures
    check-naming Check naming conventions
    list         List all schemas and their versions

FLAGS (for validate):
    --all                Validate all schemas and fixtures (default)
    --schema <name>      Validate a specific schema only
    --fixtures-only      Only validate fixtures, skip schema syntax
    --schemas-only       Only validate schema syntax, skip fixtures
    --json               Output results as JSON (default: human-readable)
    --quiet              Only print errors
```

### Usage Examples

```bash
# Validate everything
schema-validator validate --all

# Validate a specific schema and its fixtures
schema-validator validate --schema learning_goal.v1

# Validate only that schema files are syntactically valid
schema-validator validate --schemas-only

# JSON output for CI
schema-validator validate --all --json

# Check naming conventions
schema-validator check-naming

# List all schemas
schema-validator list
```

## Implementation

### Cargo.toml

```toml
[package]
name = "schema-validator"
version = "0.1.0"
edition = "2024"
description = "JSON Schema validation tool for Blup"

[[bin]]
name = "schema-validator"
path = "src/main.rs"

[dependencies]
clap = { version = "4", features = ["derive"] }
jsonschema = "0.18"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
walkdir = "2"
thiserror = "1"
```

### Main Entry Point

```rust
// main.rs (conceptual)
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "schema-validator")]
#[command(version = "0.1.0")]
#[command(about = "Blup schema validation tool")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Validate(ValidateArgs),
    CheckNaming,
    List,
}

#[derive(clap::Args)]
struct ValidateArgs {
    #[arg(long)]
    all: bool,

    #[arg(long)]
    schema: Option<String>,

    #[arg(long)]
    fixtures_only: bool,

    #[arg(long)]
    schemas_only: bool,

    #[arg(long)]
    json: bool,

    #[arg(long)]
    quiet: bool,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Validate(args) => cmd_validate(args),
        Command::CheckNaming => cmd_check_naming(),
        Command::List => cmd_list(),
    }
}
```

### Validator Core

```rust
// validator.rs (conceptual)
use jsonschema::{Draft, JSONSchema, CompilationOptions};
use serde_json::Value;
use std::path::{Path, PathBuf};

pub struct SchemaValidator {
    schemas_dir: PathBuf,
}

#[derive(Debug)]
pub struct ValidationReport {
    pub schema_name: String,
    pub schema_version: String,
    pub schema_valid: bool,
    pub schema_errors: Vec<String>,
    pub fixtures_valid: Vec<FixtureResult>,
    pub fixtures_invalid: Vec<FixtureResult>,
}

#[derive(Debug)]
pub struct FixtureResult {
    pub file_name: String,
    pub valid_json: bool,
    pub matches_schema: Option<bool>,  // None for invalid fixtures that shouldn't match
    pub errors: Vec<String>,
}

pub struct GlobalReport {
    pub schemas_checked: u32,
    pub schemas_valid: u32,
    pub schemas_invalid: u32,
    pub fixtures_valid_count: u32,
    pub fixtures_invalid_count: u32,
    pub details: Vec<ValidationReport>,
}

impl SchemaValidator {
    pub fn new(schemas_dir: &Path) -> Self {
        Self { schemas_dir: schemas_dir.to_path_buf() }
    }

    /// Validate all schemas in the schemas directory.
    pub fn validate_all(&self) -> GlobalReport {
        let mut report = GlobalReport::default();

        for entry in walkdir::WalkDir::new(&self.schemas_dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().ends_with(".schema.json"))
        {
            match self.validate_schema(entry.path()) {
                Ok(schema_report) => {
                    if schema_report.schema_valid {
                        report.schemas_valid += 1;
                    } else {
                        report.schemas_invalid += 1;
                    }
                    report.schemas_checked += 1;
                    report.fixtures_valid_count += schema_report.fixtures_valid.len() as u32;
                    report.fixtures_invalid_count += schema_report.fixtures_invalid.len() as u32;
                    report.details.push(schema_report);
                }
                Err(e) => {
                    // Schema file couldn't be read or parsed
                    report.schemas_checked += 1;
                    report.schemas_invalid += 1;
                }
            }
        }

        report
    }

    /// Validate a single schema and its fixtures.
    pub fn validate_schema(&self, schema_path: &Path) -> Result<ValidationReport, anyhow::Error> {
        let schema_json: Value = serde_json::from_reader(std::fs::File::open(schema_path)?)?;

        let schema_name = schema_path.file_stem()
            .unwrap()
            .to_string_lossy()
            .to_string();

        let mut report = ValidationReport {
            schema_name: schema_name.clone(),
            schema_version: schema_json.get("version")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string(),
            schema_valid: false,
            schema_errors: Vec::new(),
            fixtures_valid: Vec::new(),
            fixtures_invalid: Vec::new(),
        };

        // Validate schema syntax against JSON Schema meta-schema
        match JSONSchema::options()
            .with_draft(Draft::Draft202012)
            .compile(&schema_json)
        {
            Ok(_) => {
                report.schema_valid = true;
            }
            Err(errors) => {
                report.schema_errors = errors.map(|e| e.to_string()).collect();
            }
        }

        // Validate fixtures
        let fixture_dir = self.schemas_dir
            .join("fixtures")
            .join(schema_name.replace(".schema", ""));

        if fixture_dir.exists() {
            let compiled = JSONSchema::options()
                .with_draft(Draft::Draft202012)
                .compile(&schema_json)?;

            for entry in walkdir::WalkDir::new(&fixture_dir)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_file() && e.path().extension().map_or(false, |ext| ext == "json"))
            {
                let fixture_json: Value = serde_json::from_reader(
                    std::fs::File::open(entry.path())?
                )?;

                let file_name = entry.file_name().to_string_lossy().to_string();
                let is_valid_fixture = file_name.starts_with("valid-");

                let validation_result = compiled.validate(&fixture_json);
                let errors: Vec<String> = validation_result
                    .map_err(|errors| errors.map(|e| e.to_string()).collect())
                    .err()
                    .unwrap_or_default();

                let matches = errors.is_empty();

                let result = FixtureResult {
                    file_name: file_name.clone(),
                    valid_json: true,
                    matches_schema: Some(matches),
                    errors,
                };

                if is_valid_fixture {
                    report.fixtures_valid.push(result);
                } else {
                    report.fixtures_invalid.push(result);
                }
            }
        }

        Ok(report)
    }
}
```

### Output Reporter

```rust
// reporter.rs (conceptual)
impl GlobalReport {
    pub fn print_human(&self) {
        println!("=== Schema Validation Report ===\n");
        println!("Schemas: {} checked, {} valid, {} invalid",
            self.schemas_checked, self.schemas_valid, self.schemas_invalid);
        println!("Fixtures: {} valid, {} should-be-invalid checked",
            self.fixtures_valid_count, self.fixtures_invalid_count);
        println!();

        for detail in &self.details {
            let status = if detail.schema_valid { "✓" } else { "✗" };
            println!("{} {} (v{})", status, detail.schema_name, detail.schema_version);

            if !detail.schema_valid {
                for error in &detail.schema_errors {
                    println!("  Error: {}", error);
                }
            }

            for fixture in &detail.fixtures_valid {
                if fixture.matches_schema == Some(true) {
                    println!("  ✓ {}", fixture.file_name);
                } else {
                    println!("  ✗ {} (should be valid but failed)", fixture.file_name);
                    for error in &fixture.errors {
                        println!("    {}", error);
                    }
                }
            }

            for fixture in &detail.fixtures_invalid {
                if fixture.matches_schema == Some(false) {
                    println!("  ✓ {} (correctly rejected)", fixture.file_name);
                } else {
                    println!("  ✗ {} (should be invalid but passed)", fixture.file_name);
                }
            }

            println!();
        }

        let success = self.schemas_invalid == 0
            && self.details.iter().all(|d| {
                d.fixtures_valid.iter().all(|f| f.matches_schema == Some(true))
                    && d.fixtures_invalid.iter().all(|f| f.matches_schema == Some(false))
            });

        if success {
            println!("All validations passed.");
        } else {
            println!("Some validations failed.");
            std::process::exit(1);
        }
    }

    pub fn print_json(&self) {
        println!("{}", serde_json::to_string_pretty(&self).unwrap());
    }
}
```

### Naming Convention Check

```rust
// main.rs command handler (conceptual)
fn cmd_check_naming() -> anyhow::Result<()> {
    let schemas_dir = find_schemas_dir()?;
    let mut errors = 0;

    for entry in walkdir::WalkDir::new(&schemas_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name().to_string_lossy().ends_with(".schema.json"))
    {
        let name = entry.file_name().to_string_lossy();
        // Expected: {schema_name}.v{major}.schema.json
        let pattern = regex::Regex::new(r"^[a-z][a-z0-9_]*\.v\d+\.schema\.json$").unwrap();

        if !pattern.is_match(&name) {
            println!("ERROR: '{}' does not match naming convention", name);
            println!("       Expected: {{schema_name}}.v{{major}}.schema.json");
            println!("       Example: learning_goal.v1.schema.json");
            errors += 1;
        }
    }

    if errors > 0 {
        println!("\n{} naming error(s) found.", errors);
        std::process::exit(1);
    } else {
        println!("All schema files follow naming conventions.");
        Ok(())
    }
}
```

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | All validations passed |
| 1 | One or more validations failed |
| 2 | Tool usage error (missing directory, wrong args) |
| 3 | Internal error (file read failure, etc.) |

## Testing Strategy

| Test Category | Method | Description |
|---------------|--------|-------------|
| Valid schema syntax | Unit test | A known-valid schema file passes |
| Invalid schema syntax | Unit test | A schema with bad JSON fails |
| Valid fixture passes | Unit test | A correct fixture matches its schema |
| Invalid fixture fails | Unit test | An incorrect fixture is rejected |
| Naming conventions | Integration test | Correct names pass; wrong names fail |
| JSON output | Unit test | `--json` produces valid JSON report |
| No fixtures directory | Unit test | Schema without fixtures still validates |
| Empty fixture file | Unit test | Empty file is invalid JSON → error |
| Missing version field | Unit test | Schema without version field → report "unknown" |
| Cross-reference test | Unit test | Schema with `$ref` to another schema resolves correctly |

## Quality Gates

- [ ] `schema-validator validate --all` passes on all committed schemas
- [ ] `schema-validator check-naming` passes
- [ ] Tool exits 1 on any validation failure
- [ ] JSON output is parseable (validates its own output format)
- [ ] FIxtures for all 7 Phase 1 schemas exist and validate
- [ ] CI calls `schema-validator` instead of or in addition to `scripts/schema-check`
- [ ] No API keys, credentials, or private data in fixture files

## Dependency Graph

```
tools/schema-validator
  ├── depends on: schemas/  (reads schema files)
  └── used by: scripts/schema-check, CI pipeline, pre-commit hook
```
