"""Token-bucket rate limiter per provider and per model."""

import time
from collections import defaultdict

import structlog

logger = structlog.get_logger()


class TokenBucket:
    """Token-bucket rate limiter for a single key."""

    def __init__(self, capacity: int, refill_rate: float):
        self.capacity = capacity
        self.refill_rate = refill_rate  # tokens per second
        self._tokens: float = float(capacity)
        self._last_refill: float = time.monotonic()

    def _refill(self) -> None:
        now = time.monotonic()
        elapsed = now - self._last_refill
        self._tokens = min(self.capacity, self._tokens + elapsed * self.refill_rate)
        self._last_refill = now

    def allow(self, tokens: int = 1) -> bool:
        self._refill()
        if self._tokens >= tokens:
            self._tokens -= tokens
            return True
        return False

    @property
    def remaining(self) -> float:
        self._refill()
        return self._tokens


class RateLimiterManager:
    """Manages rate limiters per provider and model.

    Configurable per provider and per model with separate limits.
    """

    def __init__(
        self,
        default_per_minute: int = 60,
        per_provider_limits: dict[str, int] | None = None,
        per_model_limits: dict[str, int] | None = None,
    ):
        self._default_capacity = default_per_minute
        self._per_provider = per_provider_limits or {}
        self._per_model = per_model_limits or {}
        self._provider_buckets: dict[str, TokenBucket] = {}
        self._model_buckets: dict[str, TokenBucket] = {}

    def _get_provider_bucket(self, provider: str) -> TokenBucket:
        if provider not in self._provider_buckets:
            limit = self._per_provider.get(provider, self._default_capacity)
            self._provider_buckets[provider] = TokenBucket(
                capacity=limit, refill_rate=limit / 60.0
            )
        return self._provider_buckets[provider]

    def _get_model_bucket(self, model: str) -> TokenBucket:
        if model not in self._model_buckets:
            limit = self._per_model.get(model, self._default_capacity)
            self._model_buckets[model] = TokenBucket(
                capacity=limit, refill_rate=limit / 60.0
            )
        return self._model_buckets[model]

    def allow(self, provider: str, model: str) -> bool:
        """Check if a request is allowed for the given provider and model."""
        provider_ok = self._get_provider_bucket(provider).allow()
        model_ok = self._get_model_bucket(model).allow()

        if not provider_ok:
            logger.warn("rate_limited", key_type="provider", key=provider)
        if not model_ok:
            logger.warn("rate_limited", key_type="model", key=model)

        return provider_ok and model_ok

    def remaining(self, provider: str, model: str) -> dict[str, float]:
        return {
            "provider_remaining": self._get_provider_bucket(provider).remaining,
            "model_remaining": self._get_model_bucket(model).remaining,
        }
