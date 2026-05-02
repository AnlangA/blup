"""Tests for PromptRenderer — template loading and variable rendering."""
import tempfile
from pathlib import Path

import pytest

from src.renderer import PromptRenderer


@pytest.fixture
def prompts_dir():
    with tempfile.TemporaryDirectory() as tmp:
        d = Path(tmp)
        (d / "feasibility_check.v1.prompt.md").write_text(
            "Evaluate: {{learning_goal}} in {{domain}}"
        )
        (d / "multi_var.v1.prompt.md").write_text(
            "Goal: {{learning_goal}} | Profile: {{user_profile}}"
        )
        yield d


def test_load_existing_prompt(prompts_dir):
    renderer = PromptRenderer(prompts_dir)
    template = renderer.load("feasibility_check", version=1)
    assert "learning_goal" in template
    assert "domain" in template


def test_load_missing_prompt(prompts_dir):
    renderer = PromptRenderer(prompts_dir)
    with pytest.raises(FileNotFoundError, match="nonexistent"):
        renderer.load("nonexistent", version=1)


def test_load_missing_version(prompts_dir):
    renderer = PromptRenderer(prompts_dir)
    with pytest.raises(FileNotFoundError):
        renderer.load("feasibility_check", version=99)


def test_render_single_variable(prompts_dir):
    renderer = PromptRenderer(prompts_dir)
    template = renderer.load("feasibility_check", version=1)
    result = renderer.render(template, {"learning_goal": "Learn Rust", "domain": "programming"})
    assert "Learn Rust" in result
    assert "programming" in result


def test_render_multiple_variables(prompts_dir):
    renderer = PromptRenderer(prompts_dir)
    template = renderer.load("multi_var", version=1)
    result = renderer.render(
        template,
        {"learning_goal": "Learn Python", "user_profile": "beginner"},
    )
    assert "Learn Python" in result
    assert "beginner" in result


def test_render_missing_variable_leaves_placeholder(prompts_dir):
    renderer = PromptRenderer(prompts_dir)
    template = renderer.load("feasibility_check", version=1)
    result = renderer.render(template, {})  # no variables provided
    # Placeholder should remain in the output
    assert "{{learning_goal}}" in result


def test_render_empty_variables(prompts_dir):
    renderer = PromptRenderer(prompts_dir)
    template = renderer.load("feasibility_check", version=1)
    result = renderer.render(template, {"learning_goal": "", "domain": ""})
    assert result is not None


def test_extract_variables(prompts_dir):
    renderer = PromptRenderer(prompts_dir)
    template = renderer.load("feasibility_check", version=1)
    vars_list = renderer.extract_variables(template)
    assert "learning_goal" in vars_list
    assert "domain" in vars_list


def test_extract_variables_no_duplicates(prompts_dir):
    renderer = PromptRenderer(prompts_dir)
    template = "{{x}} and {{x}} again"
    vars_list = renderer.extract_variables(template)
    assert vars_list == ["x"]
