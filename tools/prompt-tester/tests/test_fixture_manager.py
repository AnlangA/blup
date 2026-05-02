"""Tests for FixtureManager — fixture loading, saving, and listing."""
import json
import tempfile
from pathlib import Path

import pytest

from src.fixture_manager import FixtureManager


@pytest.fixture
def fixture_dir():
    with tempfile.TemporaryDirectory() as tmp:
        d = Path(tmp)
        fixtures = d / "fixtures"
        fixtures.mkdir()
        # Create feasibility_check fixtures
        fc = fixtures / "feasibility_check"
        fc.mkdir()
        (fc / "fixture1.json").write_text(json.dumps({
            "fixture_id": "feasibility_check-fixture1",
            "description": "Test fixture 1",
            "prompt_name": "feasibility_check",
            "input": {"learning_goal": "Learn Python", "domain": "programming"},
        }))
        (fc / "fixture2.json").write_text(json.dumps({
            "fixture_id": "feasibility_check-fixture2",
            "description": "Test fixture 2",
            "prompt_name": "feasibility_check",
            "input": {"learning_goal": "Learn Rust", "domain": "systems"},
        }))
        # Create empty profile_collection
        (fixtures / "profile_collection").mkdir()
        yield d


def test_list_prompts(fixture_dir):
    mgr = FixtureManager(fixture_dir)
    prompts = mgr.list_prompts()
    assert "feasibility_check" in prompts
    assert "profile_collection" in prompts


def test_list_prompts_with_counts(fixture_dir):
    mgr = FixtureManager(fixture_dir)
    counts = mgr.list_prompts_with_counts()
    assert counts["feasibility_check"] == 2
    assert counts["profile_collection"] == 0


def test_load_all(fixture_dir):
    mgr = FixtureManager(fixture_dir)
    fixtures = mgr.load_all("feasibility_check")
    assert len(fixtures) == 2


def test_load_all_empty_directory(fixture_dir):
    mgr = FixtureManager(fixture_dir)
    fixtures = mgr.load_all("profile_collection")
    assert fixtures == []


def test_load_all_nonexistent_prompt(fixture_dir):
    mgr = FixtureManager(fixture_dir)
    fixtures = mgr.load_all("nonexistent")
    assert fixtures == []


def test_list_prompts_empty_dir():
    with tempfile.TemporaryDirectory() as tmp:
        mgr = FixtureManager(Path(tmp))
        assert mgr.list_prompts() == []


def test_save_fixture(fixture_dir):
    mgr = FixtureManager(fixture_dir)
    mgr.save("feasibility_check", {
        "fixture_id": "feasibility_check-new",
        "description": "New fixture",
    })
    fixtures = mgr.load_all("feasibility_check")
    assert len(fixtures) == 3


def test_save_creates_directory(fixture_dir):
    mgr = FixtureManager(fixture_dir)
    mgr.save("new_prompt", {
        "fixture_id": "new_prompt-test",
        "description": "Test",
    })
    assert "new_prompt" in mgr.list_prompts()


def test_corrupt_json_skipped(fixture_dir):
    mgr = FixtureManager(fixture_dir)
    fc = fixture_dir / "fixtures" / "feasibility_check"
    (fc / "corrupt.json").write_text("not valid json {{{")
    fixtures = mgr.load_all("feasibility_check")
    # Should still load the 2 valid ones, skip corrupt one
    assert len(fixtures) == 2
