import json
from pathlib import Path
from typing import Dict, List, Any


class FixtureManager:
    def __init__(self, prompts_dir: Path):
        self.prompts_dir = prompts_dir
        self.fixtures_dir = prompts_dir / "fixtures"

    def list_prompts(self) -> List[str]:
        """List all prompt names that have fixtures."""
        if not self.fixtures_dir.exists():
            return []

        prompts = []
        for fixture_dir in self.fixtures_dir.iterdir():
            if fixture_dir.is_dir():
                prompts.append(fixture_dir.name)
        return sorted(prompts)

    def list_prompts_with_counts(self) -> Dict[str, int]:
        """List all prompts with their fixture counts."""
        result = {}
        for prompt_name in self.list_prompts():
            fixtures = self.load_all(prompt_name)
            result[prompt_name] = len(fixtures)
        return result

    def load_all(self, prompt_name: str) -> List[Dict[str, Any]]:
        """Load all fixtures for a prompt."""
        fixture_dir = self.fixtures_dir / prompt_name
        if not fixture_dir.exists():
            return []

        fixtures = []
        for fixture_file in fixture_dir.glob("*.json"):
            try:
                with open(fixture_file) as f:
                    fixture = json.load(f)
                    fixtures.append(fixture)
            except json.JSONDecodeError:
                continue

        return fixtures

    def save(self, prompt_name: str, fixture: Dict[str, Any]):
        """Save a fixture for a prompt."""
        fixture_dir = self.fixtures_dir / prompt_name
        fixture_dir.mkdir(parents=True, exist_ok=True)

        fixture_id = fixture.get("fixture_id", "unknown")
        fixture_file = fixture_dir / f"{fixture_id}.json"

        with open(fixture_file, "w") as f:
            json.dump(fixture, f, indent=2)

    def generate_scaffolding(self, prompt_name: str):
        """Generate fixture scaffolding for a prompt."""
        fixture_dir = self.fixtures_dir / prompt_name
        fixture_dir.mkdir(parents=True, exist_ok=True)

        # Create a template fixture
        template = {
            "fixture_id": f"{prompt_name}_example",
            "description": f"Example fixture for {prompt_name}",
            "prompt_name": prompt_name,
            "prompt_version": 1,
            "target_schema": None,
            "input": {},
            "expected_output": {},
            "semantic_checks": [],
            "captured_from": None,
            "captured_at": None,
            "last_passed": None,
        }

        fixture_file = fixture_dir / "example.json"
        with open(fixture_file, "w") as f:
            json.dump(template, f, indent=2)
