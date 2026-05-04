// AUTO-GENERATED from sandboxes/definitions/registry.yaml — do not edit by hand.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolKind {
    PythonExec,
    NodeExec,
    TypescriptCompileRun,
    RustCompileRun,
    GoCompileRun,
    CCompileRun,
    CppCompileRun,
    JavaCompileRun,
    RubyExec,
    BashExec,
    TypstCompile,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionModel {
    Interpreted,
    Compiled,
}

#[derive(Debug, Clone, Copy)]
pub struct LanguageLimits {
    pub compile_timeout_secs: u64,
    pub run_timeout_secs: u64,
    pub memory_mb: u64,
}

impl ToolKind {
    pub fn from_language(lang: &str) -> Option<Self> {
        match lang.to_lowercase().as_str() {
            "python" | "py" | "python3" => Some(Self::PythonExec),
            "javascript" | "js" | "node" => Some(Self::NodeExec),
            "typescript" | "ts" => Some(Self::TypescriptCompileRun),
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

    pub fn to_image(&self) -> &str {
        match self {
            Self::PythonExec => "sandbox-python:latest",
            Self::NodeExec => "sandbox-node:latest",
            Self::TypescriptCompileRun => "sandbox-typescript:latest",
            Self::RustCompileRun => "sandbox-rust:latest",
            Self::GoCompileRun => "sandbox-go:latest",
            Self::CCompileRun => "sandbox-c:latest",
            Self::CppCompileRun => "sandbox-cpp:latest",
            Self::JavaCompileRun => "sandbox-java:latest",
            Self::RubyExec => "sandbox-ruby:latest",
            Self::BashExec => "sandbox-bash:latest",
            Self::TypstCompile => "sandbox-typst:latest",
        }
    }

    pub fn to_language(&self) -> &str {
        match self {
            Self::PythonExec => "python",
            Self::NodeExec => "javascript",
            Self::TypescriptCompileRun => "typescript",
            Self::RustCompileRun => "rust",
            Self::GoCompileRun => "go",
            Self::CCompileRun => "c",
            Self::CppCompileRun => "cpp",
            Self::JavaCompileRun => "java",
            Self::RubyExec => "ruby",
            Self::BashExec => "bash",
            Self::TypstCompile => "typst",
        }
    }

    pub fn execution_model(&self) -> ExecutionModel {
        match self {
            Self::PythonExec | Self::NodeExec | Self::RubyExec | Self::BashExec => {
                ExecutionModel::Interpreted
            }
            Self::TypescriptCompileRun
            | Self::RustCompileRun
            | Self::GoCompileRun
            | Self::CCompileRun
            | Self::CppCompileRun
            | Self::JavaCompileRun
            | Self::TypstCompile => ExecutionModel::Compiled,
        }
    }

    pub fn entrypoint(&self) -> Option<&[&str]> {
        match self {
            Self::PythonExec => Some(&["python", "-c"]),
            Self::NodeExec => Some(&["node", "-e"]),
            Self::TypescriptCompileRun => None,
            Self::RustCompileRun => None,
            Self::GoCompileRun => None,
            Self::CCompileRun => None,
            Self::CppCompileRun => None,
            Self::JavaCompileRun => None,
            Self::RubyExec => Some(&["ruby", "-e"]),
            Self::BashExec => Some(&["bash", "-c"]),
            Self::TypstCompile => None,
        }
    }

    pub fn runner_script(&self) -> Option<&str> {
        match self {
            Self::PythonExec => None,
            Self::NodeExec => None,
            Self::TypescriptCompileRun => Some("sandbox-run-ts"),
            Self::RustCompileRun => Some("sandbox-run-rust"),
            Self::GoCompileRun => Some("sandbox-run-go"),
            Self::CCompileRun => Some("sandbox-run-c"),
            Self::CppCompileRun => Some("sandbox-run-cpp"),
            Self::JavaCompileRun => Some("sandbox-run-java"),
            Self::RubyExec => None,
            Self::BashExec => None,
            Self::TypstCompile => Some("sandbox-run-typst"),
        }
    }

    pub fn display_name(&self) -> &str {
        match self {
            Self::PythonExec => "Python",
            Self::NodeExec => "JS",
            Self::TypescriptCompileRun => "TS",
            Self::RustCompileRun => "Rust",
            Self::GoCompileRun => "Go",
            Self::CCompileRun => "C",
            Self::CppCompileRun => "C++",
            Self::JavaCompileRun => "Java",
            Self::RubyExec => "Ruby",
            Self::BashExec => "Bash",
            Self::TypstCompile => "Typst",
        }
    }

    pub fn default_limits(&self) -> LanguageLimits {
        match self {
            Self::PythonExec => LanguageLimits {
                compile_timeout_secs: 0,
                run_timeout_secs: 10,
                memory_mb: 512,
            },
            Self::NodeExec => LanguageLimits {
                compile_timeout_secs: 0,
                run_timeout_secs: 10,
                memory_mb: 512,
            },
            Self::TypescriptCompileRun => LanguageLimits {
                compile_timeout_secs: 30,
                run_timeout_secs: 10,
                memory_mb: 512,
            },
            Self::RustCompileRun => LanguageLimits {
                compile_timeout_secs: 60,
                run_timeout_secs: 10,
                memory_mb: 1024,
            },
            Self::GoCompileRun => LanguageLimits {
                compile_timeout_secs: 30,
                run_timeout_secs: 10,
                memory_mb: 512,
            },
            Self::CCompileRun => LanguageLimits {
                compile_timeout_secs: 20,
                run_timeout_secs: 10,
                memory_mb: 256,
            },
            Self::CppCompileRun => LanguageLimits {
                compile_timeout_secs: 30,
                run_timeout_secs: 10,
                memory_mb: 512,
            },
            Self::JavaCompileRun => LanguageLimits {
                compile_timeout_secs: 30,
                run_timeout_secs: 10,
                memory_mb: 512,
            },
            Self::RubyExec => LanguageLimits {
                compile_timeout_secs: 0,
                run_timeout_secs: 10,
                memory_mb: 512,
            },
            Self::BashExec => LanguageLimits {
                compile_timeout_secs: 0,
                run_timeout_secs: 10,
                memory_mb: 128,
            },
            Self::TypstCompile => LanguageLimits {
                compile_timeout_secs: 60,
                run_timeout_secs: 10,
                memory_mb: 1024,
            },
        }
    }
}

/// Returns all registered ToolKind variants.
pub fn all_tool_kinds() -> Vec<ToolKind> {
    vec![
        ToolKind::PythonExec,
        ToolKind::NodeExec,
        ToolKind::TypescriptCompileRun,
        ToolKind::RustCompileRun,
        ToolKind::GoCompileRun,
        ToolKind::CCompileRun,
        ToolKind::CppCompileRun,
        ToolKind::JavaCompileRun,
        ToolKind::RubyExec,
        ToolKind::BashExec,
        ToolKind::TypstCompile,
    ]
}

/// Language info for display and lookup.
pub struct LanguageInfo {
    pub language: &'static str,
    pub display: &'static str,
    pub tool_kind: ToolKind,
    pub execution_model: ExecutionModel,
}

pub fn all_languages_info() -> Vec<LanguageInfo> {
    vec![
        LanguageInfo {
            language: "python",
            display: "Python",
            tool_kind: ToolKind::PythonExec,
            execution_model: ExecutionModel::Interpreted,
        },
        LanguageInfo {
            language: "javascript",
            display: "JS",
            tool_kind: ToolKind::NodeExec,
            execution_model: ExecutionModel::Interpreted,
        },
        LanguageInfo {
            language: "typescript",
            display: "TS",
            tool_kind: ToolKind::TypescriptCompileRun,
            execution_model: ExecutionModel::Compiled,
        },
        LanguageInfo {
            language: "rust",
            display: "Rust",
            tool_kind: ToolKind::RustCompileRun,
            execution_model: ExecutionModel::Compiled,
        },
        LanguageInfo {
            language: "go",
            display: "Go",
            tool_kind: ToolKind::GoCompileRun,
            execution_model: ExecutionModel::Compiled,
        },
        LanguageInfo {
            language: "c",
            display: "C",
            tool_kind: ToolKind::CCompileRun,
            execution_model: ExecutionModel::Compiled,
        },
        LanguageInfo {
            language: "cpp",
            display: "C++",
            tool_kind: ToolKind::CppCompileRun,
            execution_model: ExecutionModel::Compiled,
        },
        LanguageInfo {
            language: "java",
            display: "Java",
            tool_kind: ToolKind::JavaCompileRun,
            execution_model: ExecutionModel::Compiled,
        },
        LanguageInfo {
            language: "ruby",
            display: "Ruby",
            tool_kind: ToolKind::RubyExec,
            execution_model: ExecutionModel::Interpreted,
        },
        LanguageInfo {
            language: "bash",
            display: "Bash",
            tool_kind: ToolKind::BashExec,
            execution_model: ExecutionModel::Interpreted,
        },
        LanguageInfo {
            language: "typst",
            display: "Typst",
            tool_kind: ToolKind::TypstCompile,
            execution_model: ExecutionModel::Compiled,
        },
    ]
}
