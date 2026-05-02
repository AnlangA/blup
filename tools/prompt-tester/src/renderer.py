from pathlib import Path
from typing import Dict, Any
import re


class PromptRenderer:
    def __init__(self, prompts_dir: Path):
        self.prompts_dir = prompts_dir

    def load(self, prompt_name: str, version: int = 1) -> str:
        prompt_file = self.prompts_dir / f"{prompt_name}.v{version}.prompt.md"
        if not prompt_file.exists():
            raise FileNotFoundError(f"Prompt not found: {prompt_name}")
        return prompt_file.read_text()

    def render(self, template: str, variables: Dict[str, Any]) -> str:
        result = template
        for key, value in variables.items():
            placeholder = f"{{{{{key}}}}}"
            result = result.replace(placeholder, str(value))
        return result

    def extract_variables(self, template: str) -> list:
        pattern = r"\{\{(\w+)\}\}"
        return list(set(re.findall(pattern, template)))
