import json
from enum import Enum
from typing import Dict, List


class ReportFormat(Enum):
    TERMINAL = "terminal"
    JSON = "json"


class Reporter:
    def __init__(self, format: ReportFormat = ReportFormat.TERMINAL):
        self.format = format

    def print(self, results):
        if self.format == ReportFormat.TERMINAL:
            self._print_terminal(results)
        elif self.format == ReportFormat.JSON:
            self._print_json(results)

    def print_prompt_results(self, prompt_name: str, results: list):
        if self.format == ReportFormat.TERMINAL:
            self._print_prompt_terminal(prompt_name, results)
        elif self.format == ReportFormat.JSON:
            self._print_prompt_json(prompt_name, results)

    def _print_terminal(self, results):
        print("=== Prompt Test Results ===\n")
        for prompt_name, fixture_results in results.by_prompt.items():
            passed = sum(1 for r in fixture_results if r.passed)
            total = len(fixture_results)
            status = "✓" if passed == total else "✗"
            print(f"{status} {prompt_name}: {passed}/{total} passed")

            for r in fixture_results:
                if r.passed:
                    print(f"  ✓ {r.fixture_id}")
                else:
                    print(f"  ✗ {r.fixture_id}")
                    if r.schema_errors:
                        for err in r.schema_errors:
                            print(f"    Schema error: {err}")
                    if r.semantic_errors:
                        for err in r.semantic_errors:
                            print(f"    Semantic error: {err}")
                    if r.error:
                        print(f"    Error: {r.error}")
            print()

        total_passed = sum(
            1 for results in results.by_prompt.values() for r in results if r.passed
        )
        total_all = sum(len(r) for r in results.by_prompt.values())
        print(f"Total: {total_passed}/{total_all} passed")

    def _print_json(self, results):
        output = {
            "all_passed": results.all_passed,
            "prompts": {},
        }
        for prompt_name, fixture_results in results.by_prompt.items():
            output["prompts"][prompt_name] = [
                {
                    "fixture_id": r.fixture_id,
                    "passed": r.passed,
                    "schema_valid": r.schema_valid,
                    "semantic_valid": r.semantic_valid,
                    "schema_errors": r.schema_errors,
                    "semantic_errors": r.semantic_errors,
                    "diff": r.diff,
                    "error": r.error,
                }
                for r in fixture_results
            ]
        print(json.dumps(output, indent=2))

    def _print_prompt_terminal(self, prompt_name: str, results: list):
        passed = sum(1 for r in results if r.passed)
        total = len(results)
        status = "✓" if passed == total else "✗"
        print(f"{status} {prompt_name}: {passed}/{total} passed")

        for r in results:
            if r.passed:
                print(f"  ✓ {r.fixture_id}")
            else:
                print(f"  ✗ {r.fixture_id}")
                if r.schema_errors:
                    for err in r.schema_errors:
                        print(f"    Schema error: {err}")
                if r.semantic_errors:
                    for err in r.semantic_errors:
                        print(f"    Semantic error: {err}")
                if r.error:
                    print(f"    Error: {r.error}")

    def _print_prompt_json(self, prompt_name: str, results: list):
        output = {
            "prompt_name": prompt_name,
            "results": [
                {
                    "fixture_id": r.fixture_id,
                    "passed": r.passed,
                    "schema_valid": r.schema_valid,
                    "semantic_valid": r.semantic_valid,
                    "schema_errors": r.schema_errors,
                    "semantic_errors": r.semantic_errors,
                    "diff": r.diff,
                    "error": r.error,
                }
                for r in results
            ],
        }
        print(json.dumps(output, indent=2))
