from abc import ABC, abstractmethod
from typing import AsyncIterator

from pydantic import BaseModel


class GatewayRequest(BaseModel):
    model: str
    messages: list[dict]
    temperature: float | None = None
    max_tokens: int | None = 1024
    stream: bool = False
    response_format: dict | None = None


class GatewayResponse(BaseModel):
    content: str
    model: str
    provider: str
    usage: dict
    finish_reason: str | None = None


class StreamChunk(BaseModel):
    content: str
    index: int
    finish_reason: str | None = None


class BaseProvider(ABC):
    @abstractmethod
    def provider_name(self) -> str: ...

    @abstractmethod
    def supports_model(self, model: str) -> bool: ...

    @abstractmethod
    async def complete(self, request: GatewayRequest) -> GatewayResponse: ...

    @abstractmethod
    async def complete_stream(
        self, request: GatewayRequest
    ) -> AsyncIterator[StreamChunk]: ...
