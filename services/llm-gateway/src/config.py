import os
from pathlib import Path
from dataclasses import dataclass
from dotenv import load_dotenv

# Load .env from project root (services/llm-gateway/../..)
_project_root = Path(__file__).resolve().parent.parent.parent.parent
_dotenv_path = _project_root / ".env"
if _dotenv_path.exists():
    load_dotenv(_dotenv_path)
else:
    # Fall back to default behaviour (current directory)
    load_dotenv()


@dataclass
class Settings:
    openai_api_key: str = os.getenv("OPENAI_API_KEY", "")
    openai_base_url: str = os.getenv("OPENAI_BASE_URL", "")
    anthropic_api_key: str = os.getenv("ANTHROPIC_API_KEY", "")
    anthropic_base_url: str = os.getenv("ANTHROPIC_BASE_URL", "")
    gateway_secret: str = os.getenv("GATEWAY_SECRET", "")
    host: str = os.getenv("GATEWAY_HOST", "127.0.0.1")
    port: int = 9000
    rate_limit_per_minute: int = 60
    max_retries: int = 2
    retry_base_delay_secs: float = 1.0
    streaming_timeout_secs: float = 120.0

    def __post_init__(self):
        port_str = os.getenv("GATEWAY_PORT", "9000")
        try:
            self.port = int(port_str)
        except ValueError:
            raise ValueError(f"Invalid GATEWAY_PORT: {port_str!r}. Must be a number.")

        if not self.gateway_secret:
            raise ValueError(
                "GATEWAY_SECRET environment variable is required. "
                "Set it to a secure random string in .env"
            )

        self.rate_limit_per_minute = int(
            os.getenv("GATEWAY_RATE_LIMIT_PER_MINUTE", str(self.rate_limit_per_minute))
        )
        self.max_retries = int(
            os.getenv("GATEWAY_MAX_RETRIES", str(self.max_retries))
        )
        self.retry_base_delay_secs = float(
            os.getenv("GATEWAY_RETRY_BASE_DELAY_SECS", str(self.retry_base_delay_secs))
        )
        self.streaming_timeout_secs = float(
            os.getenv("GATEWAY_STREAMING_TIMEOUT_SECS", str(self.streaming_timeout_secs))
        )


settings = Settings()
