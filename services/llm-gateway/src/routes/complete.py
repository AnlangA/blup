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

logger = structlog.get_logger()
router = APIRouter()

providers: list = []
if settings.openai_api_key:
    providers.append(OpenAIProvider(settings.openai_api_key, base_url=settings.openai_base_url))
if settings.anthropic_api_key:
    providers.append(AnthropicProvider(settings.anthropic_api_key, base_url=settings.anthropic_base_url))


def select_provider(model: str):
    if not providers:
        raise HTTPException(
            503,
            "No LLM providers configured. Set OPENAI_API_KEY or ANTHROPIC_API_KEY.",
        )
    for p in providers:
        if p.supports_model(model):
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
# Retry helper
# ---------------------------------------------------------------------------

async def complete_with_retry(provider, request: GatewayRequest) -> GatewayRequest:
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
# Route
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

    # Rate limit by secret hash (not the secret itself, since we don't want
    # to keep secrets in memory structures longer than needed).
    rate_key = str(hash(x_gateway_secret))
    if not rate_limiter.allow(rate_key):
        remaining_secs = 60  # coarse — reset each minute window
        raise HTTPException(
            429,
            f"Rate limit exceeded ({settings.rate_limit_per_minute}/min). Retry after {remaining_secs}s.",
        )

    provider = select_provider(request.model)

    if request.stream:

        async def generate():
            try:
                async for chunk in complete_stream_with_retry(provider, request):
                    yield f"event: chunk\ndata: {chunk.model_dump_json()}\n\n"
                yield "event: done\ndata: {}\n\n"
            except asyncio.CancelledError:
                logger.info("stream_cancelled", model=request.model)
            except Exception as e:
                logger.error("stream_error", error=str(e), model=request.model)
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
            response = await complete_with_retry(provider, request)
            return response.model_dump()
        except Exception as e:
            logger.error("completion_error", error=str(e), model=request.model)
            raise HTTPException(
                502, "LLM provider returned an error. Please try again."
            )
