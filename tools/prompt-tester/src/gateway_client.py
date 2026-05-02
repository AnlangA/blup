import httpx
from typing import Dict, Any, Optional


class GatewayClient:
    def __init__(self, gateway_url: str, secret: str = ""):
        self.gateway_url = gateway_url
        self.secret = secret

    def complete(self, model: str, messages: list, **kwargs) -> Dict[str, Any]:
        """Send a completion request to the Python LLM Gateway."""
        try:
            with httpx.Client(timeout=60.0) as client:
                response = client.post(
                    f"{self.gateway_url}/v1/gateway/complete",
                    headers={"X-Gateway-Secret": self.secret},
                    json={
                        "model": model,
                        "messages": messages,
                        "stream": False,
                        **kwargs,
                    },
                )
                response.raise_for_status()
                return response.json()
        except httpx.HTTPStatusError as e:
            raise RuntimeError(f"Gateway request failed: {e}") from e
        except httpx.RequestError as e:
            raise RuntimeError(f"Gateway connection failed: {e}") from e

    def health_check(self) -> bool:
        try:
            with httpx.Client(timeout=5.0) as client:
                r = client.get(f"{self.gateway_url}/health")
                return r.status_code == 200
        except Exception:
            return False
