use std::path::PathBuf;
use std::process;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let schemas_dir = args
        .get(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("schemas"));

    let fixtures_dir = schemas_dir.join("fixtures");
    let mut errors = 0u32;

    println!("Schema Validator — {}\n", schemas_dir.display());

    let entries = match std::fs::read_dir(&schemas_dir) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("Error reading schemas dir: {e}");
            process::exit(1);
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if !name.ends_with(".v1.schema.json") {
            continue;
        }

        let schema_name = name.replace(".v1.schema.json", "");
        let schema_content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("  FAIL {name}: cannot read ({e})");
                errors += 1;
                continue;
            }
        };

        let schema_json: serde_json::Value = match serde_json::from_str(&schema_content) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("  FAIL {name}: invalid JSON ({e})");
                errors += 1;
                continue;
            }
        };

        let validator = match jsonschema::Validator::options()
            .with_draft(jsonschema::Draft::Draft202012)
            .build(&schema_json)
        {
            Ok(v) => v,
            Err(e) => {
                eprintln!("  FAIL {name}: cannot compile ({e})");
                errors += 1;
                continue;
            }
        };

        println!("  PASS {name}");

        // Validate fixtures
        let fixture_dir = fixtures_dir.join(&schema_name);
        if fixture_dir.exists() {
            if let Ok(fixture_entries) = std::fs::read_dir(&fixture_dir) {
                for f_entry in fixture_entries.flatten() {
                    let f_path = f_entry.path();
                    let Some(f_name) = f_path.file_name().and_then(|n| n.to_str()) else {
                        continue;
                    };

                    let fixture_content = match std::fs::read_to_string(&f_path) {
                        Ok(c) => c,
                        Err(_) => continue,
                    };

                    let fixture_json: serde_json::Value =
                        match serde_json::from_str(&fixture_content) {
                            Ok(v) => v,
                            Err(e) => {
                                eprintln!("    FAIL {schema_name}/{f_name}: invalid JSON ({e})");
                                errors += 1;
                                continue;
                            }
                        };

                    let validation_errors: Vec<_> = validator.iter_errors(&fixture_json).collect();

                    if f_name.starts_with("valid-") && !validation_errors.is_empty() {
                        eprintln!(
                            "    FAIL {schema_name}/{f_name}: expected valid, got errors: {:?}",
                            validation_errors
                                .iter()
                                .map(|e| e.to_string())
                                .collect::<Vec<_>>()
                        );
                        errors += 1;
                    } else if f_name.starts_with("invalid-") && validation_errors.is_empty() {
                        eprintln!("    FAIL {schema_name}/{f_name}: expected invalid, but passed");
                        errors += 1;
                    } else {
                        println!("    PASS {schema_name}/{f_name}");
                    }
                }
            }
        }
    }

    println!();
    if errors > 0 {
        eprintln!("{errors} validation error(s) found.");
        process::exit(1);
    } else {
        println!("All schemas and fixtures valid.");
    }
}
