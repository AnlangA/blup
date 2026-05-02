"""Circuit breaker for LLM provider failover."""

import time
from enum import Enum

import structlog

logger = structlog.get_logger()


class CircuitState(Enum):
    CLOSED = "closed"           # Normal operation
    OPEN = "open"               # Failing, reject requests
    HALF_OPEN = "half_open"     # Testing if service recovered


class CircuitBreaker:
    """Circuit breaker that tracks failures per provider.

    - CLOSED: normal operation, requests pass through
    - OPEN: too many failures, requests are rejected immediately
    - HALF_OPEN: testing recovery, allows limited requests
    """

    def __init__(
        self,
        failure_threshold: int = 5,
        recovery_timeout: float = 30.0,
        half_open_max: int = 1,
    ):
        self._failure_threshold = failure_threshold
        self._recovery_timeout = recovery_timeout
        self._half_open_max = half_open_max
        self._circuits: dict[str, _ProviderCircuit] = {}

    def _get_circuit(self, provider: str) -> "_ProviderCircuit":
        if provider not in self._circuits:
            self._circuits[provider] = _ProviderCircuit(
                failure_threshold=self._failure_threshold,
                recovery_timeout=self._recovery_timeout,
                half_open_max=self._half_open_max,
            )
        return self._circuits[provider]

    def allow(self, provider: str) -> bool:
        """Check if a request is allowed for the given provider."""
        return self._get_circuit(provider).allow()

    def record_success(self, provider: str) -> None:
        self._get_circuit(provider).record_success()

    def record_failure(self, provider: str) -> None:
        self._get_circuit(provider).record_failure()

    def get_state(self, provider: str) -> CircuitState:
        return self._get_circuit(provider).state


class _ProviderCircuit:
    def __init__(
        self,
        failure_threshold: int,
        recovery_timeout: float,
        half_open_max: int,
    ):
        self._failure_threshold = failure_threshold
        self._recovery_timeout = recovery_timeout
        self._half_open_max = half_open_max
        self.state = CircuitState.CLOSED
        self._failure_count: int = 0
        self._last_failure_time: float = 0.0
        self._half_open_count: int = 0

    def allow(self) -> bool:
        if self.state == CircuitState.CLOSED:
            return True
        if self.state == CircuitState.OPEN:
            if time.monotonic() - self._last_failure_time > self._recovery_timeout:
                self.state = CircuitState.HALF_OPEN
                self._half_open_count = 0
                logger.info("circuit_half_open")
                return True
            return False
        if self.state == CircuitState.HALF_OPEN:
            if self._half_open_count < self._half_open_max:
                self._half_open_count += 1
                return True
            return False
        return True

    def record_success(self) -> None:
        if self.state == CircuitState.HALF_OPEN:
            self.state = CircuitState.CLOSED
            self._failure_count = 0
            logger.info("circuit_closed")
        elif self.state == CircuitState.OPEN:
            pass  # shouldn't happen, but no-op

    def record_failure(self) -> None:
        self._failure_count += 1
        self._last_failure_time = time.monotonic()
        if self.state == CircuitState.HALF_OPEN:
            self.state = CircuitState.OPEN
            logger.warn("circuit_reopened")
        elif self._failure_count >= self._failure_threshold:
            self.state = CircuitState.OPEN
            logger.warn(
                "circuit_opened",
                failure_count=self._failure_count,
                threshold=self._failure_threshold,
            )
