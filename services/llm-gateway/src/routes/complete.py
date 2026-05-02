import asyncio
import json
import hmac
import time
from collections import defaultdict

import structlog
from fastapi import APIRouter, HTTPException, Header
from fastapi.responses import StreamingResponse

from src.config import settings
from src.providers.base import GatewayRequest
from src.providers.openai_provider import OpenAIProvider
from src.providers.anthropic_provider import AnthropicProvider
from src.middleware.circuit_breaker import CircuitBreaker
from src.middleware.cost_tracker import CostTracker
from src.routing.fallback import FallbackRouter

logger = structlog.get_logger()
router = APIRouter()

providers: list = []
if settings.openai_api_key:
    providers.append(
        OpenAIProvider(settings.openai_api_key, base_url=settings.openai_base_url)
    )
if settings.anthropic_api_key:
    providers.append(
        AnthropicProvider(
            settings.anthropic_api_key, base_url=settings.anthropic_base_url
        )
    )

# Middleware instances
circuit_breaker = CircuitBreaker(
    failure_threshold=settings.circuit_breaker_failure_threshold,
    recovery_timeout=settings.circuit_breaker_recovery_timeout_secs,
)
fallback_router = FallbackRouter(
    providers=providers,
    fallback_chains=settings.fallback_chains,
)
cost_tracker = CostTracker()


def select_provider(model: str):
    if not providers:
        raise HTTPException(
            503,
            "No LLM providers configured. Set OPENAI_API_KEY or ANTHROPIC_API_KEY.",
        )
    p = fallback_router.select_provider(model)
    if p is not None:
        return p
    raise HTTPException(400, f"No provider available for model: {model}")


# ---------------------------------------------------------------------------
# Simple in-process rate limiter (token bucket per second, reset per minute)
# ---------------------------------------------------------------------------


class RateLimiter:
    """Token-bucket rate limiter keyed by a string (e.g. IP or secret hash)."""

    def __init__(self, max_per_minute: int) -> None:
        self._max = max_per_minute
        self._buckets: defaultdict[str, list[float]] = defaultdict(list)

    def _prune(self, key: str, now: float) -> None:
        cutoff = now - 60.0
        self._buckets[key] = [t for t in self._buckets[key] if t > cutoff]

    def allow(self, key: str) -> bool:
        now = time.monotonic()
        self._prune(key, now)
        bucket = self._buckets[key]
        if len(bucket) >= self._max:
            return False
        bucket.append(now)
        return True

    def remaining(self, key: str) -> int:
        now = time.monotonic()
        self._prune(key, now)
        return max(0, self._max - len(self._buckets[key]))


rate_limiter = RateLimiter(settings.rate_limit_per_minute)


# ---------------------------------------------------------------------------
# Retry helper (retries within a single provider call)
# ---------------------------------------------------------------------------


async def complete_with_retry(provider, request: GatewayRequest) -> "GatewayResponse":
    """Call provider.complete with exponential-backoff retries on transient errors."""
    last_exc: Exception | None = None
    for attempt in range(settings.max_retries + 1):
        try:
            return await provider.complete(request)
        except Exception as exc:
            last_exc = exc
            if attempt < settings.max_retries:
                delay = settings.retry_base_delay_secs * (2**attempt)
                logger.warning(
                    "retry_attempt",
                    attempt=attempt + 1,
                    max_retries=settings.max_retries,
                    delay=delay,
                    error=str(exc),
                    model=request.model,
                )
                await asyncio.sleep(delay)
            else:
                break
    raise last_exc  # type: ignore[misc]


async def complete_stream_with_retry(provider, request: GatewayRequest):
    """Stream with retry on initial connection failure. Mid-stream failures
    are reported via the SSE error event, not retried."""
    last_exc: Exception | None = None
    for attempt in range(settings.max_retries + 1):
        try:
            async for chunk in provider.complete_stream(request):
                yield chunk
            return  # success — stream completed
        except Exception as exc:
            last_exc = exc
            if attempt < settings.max_retries:
                delay = settings.retry_base_delay_secs * (2**attempt)
                logger.warning(
                    "stream_retry_attempt",
                    attempt=attempt + 1,
                    max_retries=settings.max_retries,
                    delay=delay,
                    error=str(exc),
                    model=request.model,
                )
                await asyncio.sleep(delay)
            else:
                break
    raise last_exc  # type: ignore[misc]


# ---------------------------------------------------------------------------
# Circuit-breaker-wrapped provider call with fallback support
# ---------------------------------------------------------------------------


async def complete_with_circuit_breaker(
    request: GatewayRequest,
) -> "GatewayResponse":
    """Call the primary provider through the circuit breaker, falling back
    through the chain on failures or open circuits."""
    chain = fallback_router.get_fallback_chain(request.model)
    last_error: Exception | None = None

    for model_name in chain:
        provider = fallback_router.select_provider(model_name)
        if provider is None:
            logger.warn("fallback_skip_no_provider", model=model_name)
            continue

        provider_key = provider.provider_name()
        if not circuit_breaker.allow(provider_key):
            logger.warn("circuit_breaker_rejected", provider=provider_key, model=model_name)
            continue

        try:
            fallback_request = request.model_copy(update={"model": model_name})
            response = await complete_with_retry(provider, fallback_request)
            circuit_breaker.record_success(provider_key)

            if model_name != request.model:
                logger.info(
                    "fallback_success",
                    original_model=request.model,
                    fallback_model=model_name,
                )

            # Record cost
            usage = response.usage or {}
            input_tokens = usage.get("input_tokens", 0) or usage.get("prompt_tokens", 0)
            output_tokens = usage.get("output_tokens", 0) or usage.get("completion_tokens", 0)
            cost_tracker.record(model_name, input_tokens, output_tokens)

            return response
        except Exception as exc:
            last_error = exc
            circuit_breaker.record_failure(provider_key)
            logger.warn(
                "provider_call_failed",
                provider=provider_key,
                model=model_name,
                error=str(exc),
            )
            continue

    raise last_error or HTTPException(
        502, f"All providers failed or unavailable for: {request.model}"
    )


# ---------------------------------------------------------------------------
# Routes
# ---------------------------------------------------------------------------


@router.post("/v1/gateway/complete")
async def gateway_complete(
    request: GatewayRequest,
    x_gateway_secret: str = Header(...),
):
    if not hmac.compare_digest(x_gateway_secret, settings.gateway_secret):
        raise HTTPException(401, "Invalid gateway secret")

    if not request.messages:
        raise HTTPException(400, "messages must not be empty")

    logger.info(
        "gateway_request",
        model=request.model,
        messages_count=len(request.messages),
        stream=request.stream,
    )

    # Rate limit
    rate_key = str(hash(x_gateway_secret))
    if not rate_limiter.allow(rate_key):
        remaining_secs = 60
        raise HTTPException(
            429,
            f"Rate limit exceeded ({settings.rate_limit_per_minute}/min). Retry after {remaining_secs}s.",
        )

    if request.stream:

        async def generate():
            try:
                provider = select_provider(request.model)
                provider_key = provider.provider_name()
                if not circuit_breaker.allow(provider_key):
                    error_data = json.dumps(
                        {
                            "code": "CIRCUIT_OPEN",
                            "message": f"Circuit breaker open for provider: {provider_key}",
                        }
                    )
                    yield f"event: error\ndata: {error_data}\n\n"
                    return

                async for chunk in complete_stream_with_retry(provider, request):
                    yield f"event: chunk\ndata: {chunk.model_dump_json()}\n\n"
                yield "event: done\ndata: {}\n\n"
                circuit_breaker.record_success(provider_key)
            except asyncio.CancelledError:
                logger.info("stream_cancelled", model=request.model)
            except Exception as e:
                logger.error("stream_error", error=str(e), model=request.model)
                try:
                    circuit_breaker.record_failure(provider_key)
                except NameError:
                    pass
                error_data = json.dumps(
                    {
                        "code": "STREAM_ERROR",
                        "message": "An error occurred during streaming.",
                    }
                )
                yield f"event: error\ndata: {error_data}\n\n"

        return StreamingResponse(
            generate(),
            media_type="text/event-stream",
        )
    else:
        try:
            response = await complete_with_circuit_breaker(request)
            logger.info(
                "gateway_response",
                model=request.model,
                content_length=len(response.content),
            )
            return response.model_dump()
        except HTTPException:
            raise
        except Exception as e:
            logger.error("completion_error", error=str(e), model=request.model)
            error_message = str(e)
            if hasattr(e, "response"):
                try:
                    error_body = await e.response.text()
                    if error_body:
                        error_message = f"{error_message}: {error_body}"
                except Exception:
                    pass
            raise HTTPException(502, f"LLM provider error: {error_message}")


@router.get("/v1/gateway/stats")
async def gateway_stats(
    x_gateway_secret: str = Header(...),
):
    """Return cost and circuit breaker statistics (read-only, same auth)."""
    if not hmac.compare_digest(x_gateway_secret, settings.gateway_secret):
        raise HTTPException(401, "Invalid gateway secret")

    cb_states = {}
    for p in providers:
        key = p.provider_name()
        cb_states[key] = circuit_breaker.get_state(key).value

    return {
        "cost": cost_tracker.get_stats(),
        "circuit_breakers": cb_states,
        "providers": [p.provider_name() for p in providers],
        "rate_limiter": {
            "max_per_minute": settings.rate_limit_per_minute,
        },
    }
