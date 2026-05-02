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
    # Circuit breaker settings
    circuit_breaker_failure_threshold: int = 5
    circuit_breaker_recovery_timeout_secs: float = 30.0
    # Fallback chains (JSON dict: primary_model → [fallback_models])
    fallback_chains: dict = None  # set in __post_init__

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
        self.circuit_breaker_failure_threshold = int(
            os.getenv(
                "GATEWAY_CB_FAILURE_THRESHOLD",
                str(self.circuit_breaker_failure_threshold),
            )
        )
        self.circuit_breaker_recovery_timeout_secs = float(
            os.getenv(
                "GATEWAY_CB_RECOVERY_TIMEOUT_SECS",
                str(self.circuit_breaker_recovery_timeout_secs),
            )
        )

        # Default fallback chains
        import json

        fallback_raw = os.getenv("GATEWAY_FALLBACK_CHAINS", "")
        if fallback_raw:
            self.fallback_chains = json.loads(fallback_raw)
        else:
            self.fallback_chains = {
                "gpt-4o": ["gpt-4o", "gpt-4o-mini", "claude-sonnet-4-20250514"],
                "gpt-4o-mini": ["gpt-4o-mini", "gpt-4.1-mini"],
                "claude-sonnet-4-20250514": [
                    "claude-sonnet-4-20250514",
                    "claude-3-5-haiku-20241022",
                ],
                "claude-3-5-sonnet-20241022": [
                    "claude-3-5-sonnet-20241022",
                    "claude-3-5-haiku-20241022",
                ],
                "o1": ["o1", "o1-mini"],
                "o1-mini": ["o1-mini", "o3-mini"],
            }


settings = Settings()
