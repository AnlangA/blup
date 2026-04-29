from typing import AsyncIterator

from anthropic import AsyncAnthropic

from .base import BaseProvider, GatewayRequest, GatewayResponse, StreamChunk


class AnthropicProvider(BaseProvider):
    def __init__(self, api_key: str, base_url: str = ""):
        kwargs: dict = {"api_key": api_key}
        if base_url:
            kwargs["base_url"] = base_url
        self.client = AsyncAnthropic(**kwargs)
        self._has_custom_base = bool(base_url)

    def provider_name(self) -> str:
        return "anthropic"

    def supports_model(self, model: str) -> bool:
        if model.startswith(("claude-", "anthropic.")):
            return True
        if self._has_custom_base:
            return True
        return False

    def _split_messages(self, messages: list[dict]) -> tuple[str | None, list[dict]]:
        system_parts = [m["content"] for m in messages if m.get("role") == "system"]
        non_system = [m for m in messages if m.get("role") != "system"]
        system_text = "\n\n".join(system_parts) if system_parts else None
        return system_text, non_system

    async def complete(self, request: GatewayRequest) -> GatewayResponse:
        system_text, messages = self._split_messages(request.messages)

        kwargs: dict = {
            "model": request.model,
            "messages": messages,
            "max_tokens": request.max_tokens or 1024,
        }
        if system_text:
            kwargs["system"] = system_text
        if request.temperature is not None:
            kwargs["temperature"] = request.temperature

        response = await self.client.messages.create(**kwargs)

        text_blocks = [b.text for b in response.content if b.type == "text"]
        content = "\n".join(text_blocks)

        return GatewayResponse(
            content=content,
            model=response.model,
            provider="anthropic",
            usage={
                "prompt_tokens": response.usage.input_tokens,
                "completion_tokens": response.usage.output_tokens,
                "total_tokens": response.usage.input_tokens
                + response.usage.output_tokens,
            },
            finish_reason=response.stop_reason,
        )

    async def complete_stream(
        self, request: GatewayRequest
    ) -> AsyncIterator[StreamChunk]:
        system_text, messages = self._split_messages(request.messages)

        kwargs: dict = {
            "model": request.model,
            "messages": messages,
            "max_tokens": request.max_tokens or 1024,
            "stream": True,
        }
        if system_text:
            kwargs["system"] = system_text
        if request.temperature is not None:
            kwargs["temperature"] = request.temperature

        async with self.client.messages.stream(**kwargs) as stream:
            async for event in stream:
                if event.type == "text_delta":
                    yield StreamChunk(
                        content=event.text,
                        index=0,
                        finish_reason=None,
                    )
