"""Tests for PromptTester — integration tests using mock mode."""
import json
import tempfile
from pathlib import Path

import pytest

from src.config import Config
from src.tester import PromptTester


@pytest.fixture
def test_env():
    """Create a minimal test environment with one prompt and one fixture."""
    with tempfile.TemporaryDirectory() as tmp:
        root = Path(tmp)

        # Create prompts directory
        prompts = root / "prompts"
        prompts.mkdir()
        (prompts / "feasibility_check.v1.prompt.md").write_text(
            "Evaluate: {{learning_goal}} in {{domain}}"
        )

        # Create schemas directory
        schemas = root / "schemas"
        schemas.mkdir()
        (schemas / "feasibility_result.v1.schema.json").write_text(json.dumps({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "type": "object",
            "required": ["feasible", "reason"],
            "properties": {
                "feasible": {"type": "boolean"},
                "reason": {"type": "string"},
                "suggestions": {"type": "array", "items": {"type": "string"}},
                "estimated_duration": {"type": "string"},
                "prerequisites": {"type": "array", "items": {"type": "string"}},
            },
        }))

        # Create fixture for feasibility_check
        fixtures = prompts / "fixtures" / "feasibility_check"
        fixtures.mkdir(parents=True)
        (fixtures / "test_fixture.json").write_text(json.dumps({
            "fixture_id": "feasibility_check-test",
            "description": "Test fixture",
            "prompt_name": "feasibility_check",
            "prompt_version": 1,
            "target_schema": "feasibility_result.v1.schema.json",
            "input": {
                "learning_goal": "Learn Python",
                "domain": "programming",
            },
            "expected_output": {
                "feasible": True,
                "reason": "This is a well-defined programming goal",
                "suggestions": [],
                "estimated_duration": "4 weeks",
                "prerequisites": [],
            },
            "semantic_checks": [],
        }))

        yield {
            "prompts_dir": str(prompts),
            "schemas_dir": str(schemas),
        }


def test_test_all_mock_mode(test_env):
    config = Config(
        prompts_dir=test_env["prompts_dir"],
        schemas_dir=test_env["schemas_dir"],
    )
    tester = PromptTester(config)
    results = tester.test_all()
    assert results.all_passed


def test_test_single_prompt(test_env):
    config = Config(
        prompts_dir=test_env["prompts_dir"],
        schemas_dir=test_env["schemas_dir"],
    )
    tester = PromptTester(config)
    results = tester.test_prompt("feasibility_check")
    assert len(results) == 1
    assert results[0].passed


def test_schema_failure_detected(test_env):
    """Fixture with output that doesn't match schema should fail."""
    config = Config(
        prompts_dir=test_env["prompts_dir"],
        schemas_dir=test_env["schemas_dir"],
    )
    tester = PromptTester(config)

    # Override the fixture with bad expected output (missing required fields)
    fixtures_dir = Path(test_env["prompts_dir"]) / "fixtures" / "feasibility_check"
    (fixtures_dir / "bad_fixture.json").write_text(json.dumps({
        "fixture_id": "feasibility_check-bad",
        "description": "Bad fixture with missing fields",
        "prompt_name": "feasibility_check",
        "prompt_version": 1,
        "target_schema": "feasibility_result.v1.schema.json",
        "input": {"learning_goal": "X", "domain": "Y"},
        "expected_output": {"feasible": True},  # missing reason
        "semantic_checks": [],
    }))

    results = tester.test_prompt("feasibility_check")
    # The bad fixture should fail schema validation
    bad_result = [r for r in results if r.fixture_id == "feasibility_check-bad"]
    assert len(bad_result) > 0
    assert not bad_result[0].passed


def test_mock_mode_compares_output(test_env):
    """Mock mode should compare expected vs actual output."""
    config = Config(
        prompts_dir=test_env["prompts_dir"],
        schemas_dir=test_env["schemas_dir"],
    )
    tester = PromptTester(config)
    results = tester.test_prompt("feasibility_check")
    result = results[0]
    # Mock mode returns the expected_output directly, so diff should be empty
    assert result.diff == {} or result.diff is None
    assert result.schema_valid
    assert result.passed


def test_list_prompts(test_env):
    config = Config(
        prompts_dir=test_env["prompts_dir"],
        schemas_dir=test_env["schemas_dir"],
    )
    tester = PromptTester(config)
    prompts = tester.list_prompts()
    assert "feasibility_check" in prompts
    assert prompts["feasibility_check"] == 1
