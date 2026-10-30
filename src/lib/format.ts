import type { Timestamp } from "@/types/config";

export function toDate(ts: Timestamp | undefined): Date | null {
  if (ts === null || ts === undefined || ts === "") return null;
  let d: Date;
  if (typeof ts === "number") {
    // Heuristic: seconds vs milliseconds.
    d = new Date(ts < 1e12 ? ts * 1000 : ts);
  } else {
    const n = Number(ts);
    d = Number.isFinite(n) && /^\d+$/.test(ts) ? toDate(n)! : new Date(ts);
  }
  return Number.isNaN(d.getTime()) ? null : d;
}

export function formatTime(ts: Timestamp | undefined): string {
  const d = toDate(ts);
  if (!d) return "—";
  return d.toLocaleString("ru", {
    year: "numeric",
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });
}

export function relativeTime(ts: Timestamp | undefined, now = Date.now()): string {
  const d = toDate(ts);
  if (!d) return "никогда";
  const s = Math.round((now - d.getTime()) / 1000);
  if (s < 5) return "только что";
  if (s < 60) return `${s} с назад`;
  const m = Math.round(s / 60);
  if (m < 60) return `${m} мин назад`;
  const h = Math.round(m / 60);
  if (h < 24) return `${h} ч назад`;
  const days = Math.round(h / 24);
  if (days < 30) return `${days} дн. назад`;
  return d.toLocaleDateString("ru");
}

export function basename(path: string): string {
  const parts = path.split(/[\\/]/).filter(Boolean);
  return parts[parts.length - 1] ?? path;
}

/** Collapse a home-dir prefix to `~` for display. */
export function shortenPath(path: string): string {
  return path.replace(/^\/Users\/[^/]+/, "~").replace(/^\/home\/[^/]+/, "~");
}

export function formatBytes(n: number | null | undefined): string {
  if (n === null || n === undefined) return "—";
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
  return `${(n / 1024 / 1024).toFixed(1)} MB`;
}

export function formatValue(v: unknown): string {
  if (v === undefined) return "—";
  if (typeof v === "string") return v;
  try {
    return JSON.stringify(v);
  } catch {
    return String(v);
  }
}

export function extname(path: string): string {
  const base = basename(path);
  const i = base.lastIndexOf(".");
  return i > 0 ? base.slice(i + 1).toLowerCase() : "";
}
