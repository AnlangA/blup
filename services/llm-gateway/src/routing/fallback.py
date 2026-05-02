"""Fallback routing: primary model -> secondary models on failure."""

import structlog

from src.providers.base import BaseProvider, GatewayRequest, GatewayResponse

logger = structlog.get_logger()


class FallbackRouter:
    """Routes requests with a fallback chain.

    If the primary model fails, tries each fallback model in order.
    Example chain: {"gpt-4o": ["gpt-4o", "claude-sonnet-4-20250514", "gpt-4o-mini"]}
    """

    def __init__(
        self,
        providers: list[BaseProvider],
        fallback_chains: dict[str, list[str]] | None = None,
    ):
        self._providers = providers
        self._fallback_chains = fallback_chains or {}

    def select_provider(self, model: str) -> BaseProvider | None:
        """Select a provider for the given model."""
        for p in self._providers:
            if p.supports_model(model):
                return p
        return None

    def get_fallback_chain(self, model: str) -> list[str]:
        """Get the fallback chain for a model."""
        return self._fallback_chains.get(model, [model])

    async def route_with_fallback(
        self,
        request: GatewayRequest,
        complete_fn,  # async (provider, request) -> GatewayResponse
    ) -> GatewayResponse:
        """Try the primary model, then fallback models on failure."""
        chain = self.get_fallback_chain(request.model)
        last_error: Exception | None = None

        for model_name in chain:
            provider = self.select_provider(model_name)
            if provider is None:
                logger.warn(
                    "fallback_skip_no_provider",
                    model=model_name,
                )
                continue

            try:
                # Create a modified request with the fallback model
                fallback_request = request.model_copy(update={"model": model_name})
                response = await complete_fn(provider, fallback_request)

                if model_name != request.model:
                    logger.info(
                        "fallback_success",
                        original_model=request.model,
                        fallback_model=model_name,
                    )

                return response
            except Exception as e:
                last_error = e
                logger.warn(
                    "fallback_triggered",
                    model=model_name,
                    error=str(e),
                )
                continue

        raise last_error or RuntimeError(
            f"All providers failed for chain: {chain}"
        )
