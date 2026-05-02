"""Anthropic prompt caching: inject cache_control breakpoints."""

import structlog

logger = structlog.get_logger()


class PromptCacheManager:
    """Manages Anthropic prompt caching via cache_control breakpoints.

    Injects ephemeral cache breakpoints on the last N messages to maximize
    cache hits for repeated system prompts and conversation history.
    """

    def __init__(self, max_cache_points: int = 4):
        self._max_cache_points = max_cache_points

    def prepare_messages(self, messages: list[dict]) -> list[dict]:
        """Inject cache_control breakpoints into messages for Anthropic."""
        result = []
        for i, msg in enumerate(messages):
            # Only cache the system prompt and first few messages
            if i < self._max_cache_points:
                result.append(
                    {
                        "role": msg.get("role", "user"),
                        "content": [
                            {
                                "type": "text",
                                "text": msg.get("content", ""),
                                "cache_control": {"type": "ephemeral"},
                            }
                        ],
                    }
                )
            else:
                result.append(msg)
        return result

    @staticmethod
    def extract_cache_usage(response) -> dict:
        """Extract cache token counts from an Anthropic response."""
        usage = getattr(response, "usage", None)
        if usage is None:
            return {}

        return {
            "cache_read_tokens": getattr(usage, "cache_read_input_tokens", 0),
            "cache_write_tokens": getattr(usage, "cache_creation_input_tokens", 0),
        }
