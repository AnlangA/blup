"""Per-request cost tracking and attribution."""

import time
from collections import defaultdict

import structlog

logger = structlog.get_logger()

# Cost per 1K tokens (USD) — approximate as of 2025
COST_TABLE: dict[str, dict[str, float]] = {
    "gpt-4o": {"input": 0.0025, "output": 0.01},
    "gpt-4o-mini": {"input": 0.00015, "output": 0.0006},
    "gpt-4.1": {"input": 0.002, "output": 0.008},
    "gpt-4.1-mini": {"input": 0.0004, "output": 0.0016},
    "gpt-4.1-nano": {"input": 0.0001, "output": 0.0004},
    "o1": {"input": 0.015, "output": 0.06},
    "o1-mini": {"input": 0.003, "output": 0.012},
    "o3-mini": {"input": 0.0011, "output": 0.0044},
    "claude-sonnet-4-20250514": {"input": 0.003, "output": 0.015},
    "claude-3-5-sonnet-20241022": {"input": 0.003, "output": 0.015},
    "claude-3-5-haiku-20241022": {"input": 0.0008, "output": 0.004},
}

# Default fallback cost
DEFAULT_COST = {"input": 0.002, "output": 0.008}


class CostTracker:
    """Tracks cost per session and provides aggregate stats."""

    def __init__(self) -> None:
        self._session_costs: dict[str, float] = defaultdict(float)
        self._model_costs: dict[str, float] = defaultdict(float)
        self._total_cost: float = 0.0
        self._request_count: int = 0

    def record(
        self,
        model: str,
        input_tokens: int,
        output_tokens: int,
        session_id: str | None = None,
    ) -> dict[str, float]:
        """Record a request's cost and return the cost breakdown."""
        costs = COST_TABLE.get(model, DEFAULT_COST)
        input_cost = (input_tokens / 1000.0) * costs["input"]
        output_cost = (output_tokens / 1000.0) * costs["output"]
        request_cost = input_cost + output_cost

        self._total_cost += request_cost
        self._model_costs[model] += request_cost
        self._request_count += 1

        if session_id:
            self._session_costs[session_id] += request_cost

        logger.info(
            "cost_recorded",
            model=model,
            input_tokens=input_tokens,
            output_tokens=output_tokens,
            request_cost=round(request_cost, 6),
            total_cost=round(self._total_cost, 4),
        )

        return {
            "input_cost": round(input_cost, 6),
            "output_cost": round(output_cost, 6),
            "request_cost": round(request_cost, 6),
        }

    def get_session_cost(self, session_id: str) -> float:
        return self._session_costs.get(session_id, 0.0)

    def get_stats(self) -> dict:
        return {
            "total_cost": round(self._total_cost, 4),
            "total_requests": self._request_count,
            "by_model": {k: round(v, 4) for k, v in self._model_costs.items()},
        }
