"""Tests for the LLM gateway middleware and routing modules."""

import time
import pytest

from src.middleware.rate_limiter import RateLimiterManager, TokenBucket
from src.middleware.cost_tracker import CostTracker
from src.middleware.circuit_breaker import CircuitBreaker, CircuitState
from src.cache import PromptCacheManager
from src.routing.fallback import FallbackRouter


# ── TokenBucket ──


class TestTokenBucket:
    def test_allows_within_capacity(self):
        bucket = TokenBucket(capacity=5, refill_rate=5.0)
        for _ in range(5):
            assert bucket.allow()
        assert not bucket.allow()

    def test_refills_over_time(self):
        bucket = TokenBucket(capacity=1, refill_rate=1000.0)  # very fast refill
        assert bucket.allow()
        time.sleep(0.01)
        assert bucket.allow()

    def test_remaining(self):
        bucket = TokenBucket(capacity=10, refill_rate=10.0)
        assert bucket.remaining == 10.0
        bucket.allow()
        assert bucket.remaining < 10.0


# ── RateLimiterManager ──


class TestRateLimiterManager:
    def test_default_limits(self):
        limiter = RateLimiterManager(default_per_minute=2)
        assert limiter.allow("openai", "gpt-4o")
        assert limiter.allow("openai", "gpt-4o")
        assert not limiter.allow("openai", "gpt-4o")

    def test_per_provider_limits(self):
        limiter = RateLimiterManager(
            default_per_minute=100,
            per_provider_limits={"openai": 1},
        )
        assert limiter.allow("openai", "gpt-4o")
        assert not limiter.allow("openai", "gpt-4o")
        # Anthropic should use default
        assert limiter.allow("anthropic", "claude-3")

    def test_remaining(self):
        limiter = RateLimiterManager(default_per_minute=10)
        remaining = limiter.remaining("openai", "gpt-4o")
        assert remaining["provider_remaining"] > 0
        assert remaining["model_remaining"] > 0


# ── CostTracker ──


class TestCostTracker:
    def test_records_cost(self):
        tracker = CostTracker()
        result = tracker.record("gpt-4o", input_tokens=1000, output_tokens=500)
        assert result["request_cost"] > 0
        assert tracker.get_stats()["total_requests"] == 1

    def test_session_cost(self):
        tracker = CostTracker()
        tracker.record("gpt-4o", 1000, 500, session_id="sess-1")
        tracker.record("gpt-4o", 500, 200, session_id="sess-1")
        assert tracker.get_session_cost("sess-1") > 0
        assert tracker.get_session_cost("sess-2") == 0.0

    def test_unknown_model_uses_default(self):
        tracker = CostTracker()
        result = tracker.record("unknown-model", 1000, 1000)
        assert result["request_cost"] > 0

    def test_stats(self):
        tracker = CostTracker()
        tracker.record("gpt-4o", 1000, 500)
        tracker.record("claude-sonnet-4-20250514", 1000, 500)
        stats = tracker.get_stats()
        assert stats["total_requests"] == 2
        assert "gpt-4o" in stats["by_model"]
        assert "claude-sonnet-4-20250514" in stats["by_model"]


# ── CircuitBreaker ──


class TestCircuitBreaker:
    def test_starts_closed(self):
        cb = CircuitBreaker()
        assert cb.get_state("openai") == CircuitState.CLOSED
        assert cb.allow("openai")

    def test_opens_after_failures(self):
        cb = CircuitBreaker(failure_threshold=3)
        for _ in range(3):
            cb.record_failure("openai")
        assert cb.get_state("openai") == CircuitState.OPEN
        assert not cb.allow("openai")

    def test_success_resets_failures(self):
        cb = CircuitBreaker(failure_threshold=3)
        cb.record_failure("openai")
        cb.record_failure("openai")
        cb.record_success("openai")
        # Should still be closed since we didn't reach threshold
        assert cb.get_state("openai") == CircuitState.CLOSED

    def test_half_open_allows_limited(self):
        cb = CircuitBreaker(failure_threshold=2, recovery_timeout=0.0)
        cb.record_failure("openai")
        cb.record_failure("openai")
        assert cb.get_state("openai") == CircuitState.OPEN
        # With recovery_timeout=0, should transition to half_open immediately
        assert cb.allow("openai")
        assert cb.get_state("openai") == CircuitState.HALF_OPEN

    def test_half_open_success_closes(self):
        cb = CircuitBreaker(failure_threshold=2, recovery_timeout=0.0)
        cb.record_failure("openai")
        cb.record_failure("openai")
        cb.allow("openai")  # triggers half_open
        cb.record_success("openai")
        assert cb.get_state("openai") == CircuitState.CLOSED

    def test_independent_per_provider(self):
        cb = CircuitBreaker(failure_threshold=1)
        cb.record_failure("openai")
        assert cb.get_state("openai") == CircuitState.OPEN
        assert cb.get_state("anthropic") == CircuitState.CLOSED


# ── PromptCacheManager ──


class TestPromptCacheManager:
    def test_injects_cache_control(self):
        pcm = PromptCacheManager(max_cache_points=2)
        messages = [
            {"role": "system", "content": "You are helpful."},
            {"role": "user", "content": "Hello"},
            {"role": "assistant", "content": "Hi"},
        ]
        result = pcm.prepare_messages(messages)
        # First 2 messages should have cache_control
        assert isinstance(result[0]["content"], list)
        assert result[0]["content"][0].get("cache_control") == {"type": "ephemeral"}
        assert isinstance(result[1]["content"], list)
        # Third message should be unchanged
        assert isinstance(result[2], dict)
        assert isinstance(result[2]["content"], str)

    def test_no_cache_points_beyond_limit(self):
        pcm = PromptCacheManager(max_cache_points=0)
        messages = [{"role": "user", "content": "Hello"}]
        result = pcm.prepare_messages(messages)
        # With 0 cache points, message should be unchanged
        assert result[0] == messages[0]


# ── FallbackRouter ──


class TestFallbackRouter:
    def test_select_provider(self):
        router = FallbackRouter(providers=[], fallback_chains={})
        # No providers configured
        assert router.select_provider("gpt-4o") is None

    def test_get_fallback_chain(self):
        router = FallbackRouter(
            providers=[],
            fallback_chains={"gpt-4o": ["gpt-4o", "gpt-4o-mini"]},
        )
        chain = router.get_fallback_chain("gpt-4o")
        assert chain == ["gpt-4o", "gpt-4o-mini"]
        # Unknown model returns itself
        assert router.get_fallback_chain("unknown") == ["unknown"]
