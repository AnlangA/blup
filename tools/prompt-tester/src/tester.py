import json
from pathlib import Path
from typing import Dict, List, Optional, Any
from dataclasses import dataclass
from datetime import datetime

from .config import Config
from .renderer import PromptRenderer
from .validator import SchemaValidator
from .mock_llm import MockLlm
from .gateway_client import GatewayClient
from .fixture_manager import FixtureManager


@dataclass
class FixtureResult:
    fixture_id: str
    passed: bool = False
    schema_valid: bool = False
    semantic_valid: bool = False
    schema_errors: List[str] = None
    semantic_errors: List[str] = None
    diff: Optional[Dict] = None
    error: Optional[str] = None

    def __post_init__(self):
        if self.schema_errors is None:
            self.schema_errors = []
        if self.semantic_errors is None:
            self.semantic_errors = []


@dataclass
class TestResults:
    by_prompt: Dict[str, List[FixtureResult]] = None

    def __post_init__(self):
        if self.by_prompt is None:
            self.by_prompt = {}

    @property
    def all_passed(self) -> bool:
        return all(r.passed for results in self.by_prompt.values() for r in results)

    def add(self, prompt_name: str, results: List[FixtureResult]):
        self.by_prompt[prompt_name] = results


class PromptTester:
    def __init__(self, config: Config):
        self.config = config
        self.renderer = PromptRenderer(config.prompts_path)
        self.fixtures = FixtureManager(config.prompts_path)
        self.validator = SchemaValidator(config.schemas_path)
        self.mock_llm = MockLlm()
        self.gateway = GatewayClient(config.gateway_url) if config.use_gateway else None

    def test_all(self) -> TestResults:
        results = TestResults()
        for prompt_name in self.fixtures.list_prompts():
            prompt_results = self.test_prompt(prompt_name)
            results.add(prompt_name, prompt_results)
        return results

    def test_prompt(self, prompt_name: str) -> List[FixtureResult]:
        results = []
        fixtures = self.fixtures.load_all(prompt_name)
        prompt_template = self.renderer.load(prompt_name, version=1)

        for fixture in fixtures:
            result = FixtureResult(fixture_id=fixture.get("fixture_id", "unknown"))

            try:
                # 1. Render prompt with fixture input variables
                rendered = self.renderer.render(
                    prompt_template, fixture.get("input", {})
                )

                # 2. Get LLM response (mock or real gateway)
                if self.gateway:
                    response = self.gateway.complete(
                        model="gpt-4o",
                        messages=[
                            {"role": "system", "content": rendered},
                            {
                                "role": "user",
                                "content": json.dumps(fixture.get("input", {})),
                            },
                        ],
                    )
                else:
                    response = self.mock_llm.respond(fixture.get("expected_output", {}))

                # 3. Validate response against target schema
                target_schema = fixture.get("target_schema")
                if target_schema:
                    self.validator.validate(response, target_schema)
                    result.schema_valid = True

                # 4. Run semantic checks
                semantic_checks = fixture.get("semantic_checks", [])
                if semantic_checks:
                    semantic_errors = self.run_semantic_checks(
                        response, semantic_checks
                    )
                    result.semantic_errors = semantic_errors
                    result.semantic_valid = len(semantic_errors) == 0
                else:
                    result.semantic_valid = True

                # 5. Compare with expected output (mock mode only)
                if not self.gateway:
                    expected = fixture.get("expected_output", {})
                    diff = self.compare_outputs(response, expected)
                    result.diff = diff

                result.passed = result.schema_valid and result.semantic_valid

            except Exception as e:
                result.error = str(e)
                result.passed = False

            results.append(result)

        return results

    def run_semantic_checks(self, output: dict, checks: list) -> List[str]:
        errors = []
        for check in checks:
            rule = check.get("rule")
            if rule == "feasible_true_implies_empty_suggestions":
                if output.get("feasible") and len(output.get("suggestions", [])) > 0:
                    errors.append(
                        f"feasible=true but suggestions is non-empty: {output['suggestions']}"
                    )

            elif rule == "estimated_duration_format":
                import re

                duration = output.get("estimated_duration", "")
                if not re.match(r"\d+[-–]\d+\s*(weeks?|months?|hours?)", duration):
                    errors.append(
                        f"Duration '{duration}' does not match expected format"
                    )

            elif rule == "reason_is_specific":
                reason = output.get("reason", "")
                if len(reason) < 20:
                    errors.append(f"Reason is too short: '{reason}'")

        return errors

    def compare_outputs(self, actual: dict, expected: dict) -> dict:
        diff = {}
        for key in set(list(actual.keys()) + list(expected.keys())):
            if key not in actual:
                diff[key] = {"expected": expected[key], "actual": None}
            elif key not in expected:
                diff[key] = {"expected": None, "actual": actual[key]}
            elif actual[key] != expected[key]:
                diff[key] = {"expected": expected[key], "actual": actual[key]}
        return diff

    def capture(self, prompt_name: str) -> dict:
        if not self.gateway:
            raise RuntimeError("Gateway mode required for capture")

        fixtures = self.fixtures.load_all(prompt_name)
        if not fixtures:
            raise ValueError(f"No fixtures found for {prompt_name}")

        fixture = fixtures[0]
        rendered = self.renderer.render(
            self.renderer.load(prompt_name, version=1),
            fixture.get("input", {}),
        )

        response = self.gateway.complete(
            model="gpt-4o",
            messages=[
                {"role": "system", "content": rendered},
                {"role": "user", "content": json.dumps(fixture.get("input", {}))},
            ],
        )

        # Validate the captured response
        target_schema = fixture.get("target_schema")
        if target_schema:
            self.validator.validate(response, target_schema)

        return {
            **fixture,
            "expected_output": response,
            "captured_from": "gpt-4o",
            "captured_at": datetime.utcnow().isoformat(),
        }

    def save_fixture(self, prompt_name: str, fixture: dict):
        self.fixtures.save(prompt_name, fixture)

    def list_prompts(self) -> Dict[str, int]:
        return self.fixtures.list_prompts_with_counts()

    def generate_fixture_scaffolding(self, prompt_name: str):
        self.fixtures.generate_scaffolding(prompt_name)
