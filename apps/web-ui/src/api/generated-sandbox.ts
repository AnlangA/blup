// AUTO-GENERATED from sandboxes/definitions/registry.yaml — do not edit by hand.

export const SUPPORTED_LANGUAGES: Record<string, string> = {
  "python": "python",
  "py": "python",
  "python3": "python",
  "javascript": "javascript",
  "js": "javascript",
  "node": "javascript",
  "typescript": "typescript",
  "ts": "typescript",
  "rust": "rust",
  "rs": "rust",
  "go": "go",
  "golang": "go",
  "c": "c",
  "cpp": "cpp",
  "c++": "cpp",
  "java": "java",
  "ruby": "ruby",
  "rb": "ruby",
  "bash": "bash",
  "sh": "bash",
  "shell": "bash",
  "zsh": "bash",
  "typst": "typst",
} as const;

export type SandboxLanguage = "bash" | "c" | "cpp" | "go" | "java" | "javascript" | "python" | "ruby" | "rust" | "typescript" | "typst";

export const LANGUAGE_DISPLAY: Record<string, string> = {
  "python": "Python",
  "javascript": "JS",
  "typescript": "TS",
  "rust": "Rust",
  "go": "Go",
  "c": "C",
  "cpp": "C++",
  "java": "Java",
  "ruby": "Ruby",
  "bash": "Bash",
  "typst": "Typst",
};

export const TOOL_KIND_VALUES = ["bash_exec", "c_compile_run", "cpp_compile_run", "go_compile_run", "java_compile_run", "node_exec", "python_exec", "ruby_exec", "rust_compile_run", "typescript_compile_run", "typst_compile"] as const;
