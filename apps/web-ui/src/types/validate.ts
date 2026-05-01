/**
 * Lightweight runtime shape checker for API responses.
 *
 * Does NOT perform full schema validation — just verifies that expected
 * top-level fields exist with the right JavaScript types. Mismatches are
 * logged as console.warn for debugging; they never throw.
 *
 * This serves as a living contract: when the backend changes its response
 * shape, the developer gets warnings in the console instead of silent
 * runtime errors deep in component render trees.
 */

export interface FieldRule {
  type: "string" | "number" | "boolean" | "object" | "array" | "null";
  optional?: boolean;
  fields?: Record<string, FieldRule>;
  itemRule?: FieldRule;
}

export function checkShape(
  name: string,
  data: unknown,
  rules: Record<string, FieldRule>,
): string[] {
  const warnings: string[] = [];

  if (typeof data !== "object" || data === null) {
    warnings.push(`${name}: expected object, got ${typeof data}`);
    return warnings;
  }

  const obj = data as Record<string, unknown>;

  for (const [field, rule] of Object.entries(rules)) {
    const value = obj[field];

    if (value === undefined) {
      if (!rule.optional) {
        warnings.push(`${name}.${field}: missing required field`);
      }
      continue;
    }

    if (value === null) {
      if (!rule.optional && rule.type !== "null") {
        warnings.push(`${name}.${field}: expected ${rule.type}, got null`);
      }
      continue;
    }

    const actualType = Array.isArray(value) ? "array" : typeof value;
    if (actualType !== rule.type) {
      warnings.push(
        `${name}.${field}: expected ${rule.type}, got ${actualType}`,
      );
      continue;
    }

    // Recurse into nested objects
    if (rule.type === "object" && rule.fields) {
      warnings.push(
        ...checkShape(`${name}.${field}`, value, rule.fields),
      );
    }

    // Check array item types if specified
    if (rule.type === "array" && rule.itemRule && Array.isArray(value)) {
      for (let i = 0; i < value.length; i++) {
        const itemType = typeof value[i];
        if (itemType !== rule.itemRule.type) {
          warnings.push(
            `${name}.${field}[${i}]: expected ${rule.itemRule.type}, got ${itemType}`,
          );
        }
      }
    }
  }

  return warnings;
}
