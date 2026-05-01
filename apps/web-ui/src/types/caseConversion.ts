/**
 * Deep-convert all object keys from snake_case to camelCase.
 * Preserves arrays, nulls, and primitives unchanged.
 */
export function deepSnakeToCamel(obj: unknown): unknown {
  if (obj === null || obj === undefined) return obj;
  if (Array.isArray(obj)) return obj.map(deepSnakeToCamel);
  if (typeof obj !== "object") return obj;

  const result: Record<string, unknown> = {};
  for (const [key, value] of Object.entries(obj as Record<string, unknown>)) {
    result[snakeToCamel(key)] = deepSnakeToCamel(value);
  }
  return result;
}

function snakeToCamel(str: string): string {
  return str.replace(/_([a-z0-9])/g, (_, c: string) => c.toUpperCase());
}
