import json
from pathlib import Path
from typing import Any, Dict
import jsonschema


class SchemaValidator:
    def __init__(self, schemas_dir: Path):
        self.schemas_dir = schemas_dir
        self._schemas_cache: Dict[str, dict] = {}

    def load_schema(self, schema_name: str) -> dict:
        if schema_name in self._schemas_cache:
            return self._schemas_cache[schema_name]

        schema_file = self.schemas_dir / schema_name
        if not schema_file.exists():
            raise FileNotFoundError(f"Schema not found: {schema_name}")

        with open(schema_file) as f:
            schema = json.load(f)

        self._schemas_cache[schema_name] = schema
        return schema

    def validate(self, data: Any, schema_name: str) -> bool:
        schema = self.load_schema(schema_name)
        try:
            jsonschema.validate(instance=data, schema=schema)
            return True
        except jsonschema.ValidationError as e:
            raise ValidationError(f"Schema validation failed: {e.message}") from e

    def validate_or_error(self, data: Any, schema_name: str) -> tuple:
        try:
            self.validate(data, schema_name)
            return True, None
        except ValidationError as e:
            return False, str(e)


class ValidationError(Exception):
    pass
