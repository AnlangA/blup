from typing import Any, Dict


class MockLlm:
    def respond(self, expected_output: Dict[str, Any]) -> Dict[str, Any]:
        """Return the expected output as the mock response."""
        return expected_output
