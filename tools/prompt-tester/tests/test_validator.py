"""Tests for SchemaValidator — schema loading and JSON validation."""
import tempfile
from pathlib import Path
import json

import pytest

from src.validator import SchemaValidator


@pytest.fixture
def schemas_dir():
    with tempfile.TemporaryDirectory() as tmp:
        d = Path(tmp)
        (d / "test_schema.v1.schema.json").write_text(json.dumps({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "type": "object",
            "required": ["name", "age"],
            "properties": {
                "name": {"type": "string"},
                "age": {"type": "integer", "minimum": 0},
            },
        }))
        yield d


SCHEMA = "test_schema.v1.schema.json"


def test_validate_valid_data(schemas_dir):
    validator = SchemaValidator(schemas_dir)
    # Should not raise
    validator.validate({"name": "Alice", "age": 30}, SCHEMA)


def test_validate_invalid_missing_field(schemas_dir):
    validator = SchemaValidator(schemas_dir)
    with pytest.raises(Exception):
        validator.validate({"name": "Bob"}, SCHEMA)  # missing age


def test_validate_invalid_wrong_type(schemas_dir):
    validator = SchemaValidator(schemas_dir)
    with pytest.raises(Exception):
        validator.validate({"name": "Bob", "age": "thirty"}, SCHEMA)


def test_validate_schema_not_found(schemas_dir):
    validator = SchemaValidator(schemas_dir)
    with pytest.raises(FileNotFoundError):
        validator.validate({}, "nonexistent")


def test_validate_empty_object(schemas_dir):
    validator = SchemaValidator(schemas_dir)
    with pytest.raises(Exception):
        validator.validate({}, SCHEMA)
