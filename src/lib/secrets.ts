const SECRET_KEY = /(token|secret|password|passwd|api[_-]?key|auth|credential|private[_-]?key)/i;
const ALREADY_MASKED = /(\*{3,}|•{3,}|<redacted>|\[redacted\])/i;

/**
 * Defensive client-side masking. The backend already masks secrets; this only
 * catches values that look sensitive by key name and weren't masked.
 */
export function maskIfSecret(key: string, value: unknown): unknown {
  if (typeof value !== "string" || !value) return value;
  if (!SECRET_KEY.test(key)) return value;
  if (ALREADY_MASKED.test(value)) return value;
  if (/^\$\{?[A-Z0-9_]+\}?(:-.*)?$/.test(value)) return value; // env reference
  return "••••••••";
}

export function maskRecord(
  rec: Record<string, unknown> | undefined | null,
): Record<string, unknown> {
  const out: Record<string, unknown> = {};
  if (!rec) return out;
  for (const [k, v] of Object.entries(rec)) out[k] = maskIfSecret(k, v);
  return out;
}

/** Deep-mask objects such as `env` blocks inside a settings document. */
export function maskDeep(value: unknown, key = ""): unknown {
  if (Array.isArray(value)) return value.map((v) => maskDeep(v, key));
  if (value && typeof value === "object") {
    const out: Record<string, unknown> = {};
    for (const [k, v] of Object.entries(value as Record<string, unknown>)) {
      out[k] = maskDeep(v, k);
    }
    return out;
  }
  return maskIfSecret(key, value);
}
