from dataclasses import dataclass
from pathlib import Path


@dataclass
class Config:
    prompts_dir: str = "../prompts"
    schemas_dir: str = "../schemas"
    gateway_url: str = "http://127.0.0.1:9000"
    use_gateway: bool = False
    verbose: bool = False
    json_output: bool = False

    @property
    def prompts_path(self) -> Path:
        return Path(self.prompts_dir)

    @property
    def schemas_path(self) -> Path:
        return Path(self.schemas_dir)

    def validate(self) -> bool:
        """Validate configuration."""
        if not self.prompts_path.exists():
            return False
        if not self.schemas_path.exists():
            return False
        return True
