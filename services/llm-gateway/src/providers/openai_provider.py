from typing import AsyncIterator

from openai import AsyncOpenAI

from .base import BaseProvider, GatewayRequest, GatewayResponse, StreamChunk


class OpenAIProvider(BaseProvider):
    def __init__(self, api_key: str, base_url: str = ""):
        kwargs: dict = {"api_key": api_key}
        if base_url:
            kwargs["base_url"] = base_url
        self.client = AsyncOpenAI(**kwargs)
        self._has_custom_base = bool(base_url)

    def provider_name(self) -> str:
        return "openai"

    def supports_model(self, model: str) -> bool:
        # Known OpenAI model prefixes
        if model.startswith(("gpt-", "o1", "o3", "o4")):
            return True
        # When using a custom base URL (local LLM, proxy), accept any model
        if self._has_custom_base:
            return True
        return False

    async def complete(self, request: GatewayRequest) -> GatewayResponse:
        kwargs: dict = {
            "model": request.model,
            "messages": request.messages,
            "max_tokens": request.max_tokens or 1024,
        }
        if request.temperature is not None:
            kwargs["temperature"] = request.temperature
        if request.response_format:
            kwargs["response_format"] = request.response_format

        response = await self.client.chat.completions.create(**kwargs)

        if not response.choices:
            raise ValueError("OpenAI returned empty choices")

        return GatewayResponse(
            content=response.choices[0].message.content or "",
            model=response.model,
            provider="openai",
            usage={
                "prompt_tokens": response.usage.prompt_tokens if response.usage else 0,
                "completion_tokens": response.usage.completion_tokens
                if response.usage
                else 0,
                "total_tokens": response.usage.total_tokens if response.usage else 0,
            },
            finish_reason=response.choices[0].finish_reason,
        )

    async def complete_stream(
        self, request: GatewayRequest
    ) -> AsyncIterator[StreamChunk]:
        kwargs: dict = {
            "model": request.model,
            "messages": request.messages,
            "max_tokens": request.max_tokens or 1024,
            "stream": True,
        }
        if request.temperature is not None:
            kwargs["temperature"] = request.temperature

        async with await self.client.chat.completions.create(**kwargs) as stream:
            async for chunk in stream:
                if chunk.choices and chunk.choices[0].delta:
                    content = chunk.choices[0].delta.content or ""
                    if content:
                        yield StreamChunk(
                            content=content,
                            index=chunk.choices[0].index,
                            finish_reason=chunk.choices[0].finish_reason,
                        )
