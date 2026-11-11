export function formatBytes(n: number | null | undefined): string {
  if (n == null || !isFinite(n)) return "—";
  const units = ["B", "KB", "MB", "GB", "TB"];
  let i = 0;
  let v = n;
  while (v >= 1024 && i < units.length - 1) {
    v /= 1024;
    i++;
  }
  return `${v.toFixed(v >= 100 || i === 0 ? 0 : 1)} ${units[i]}`;
}

export function formatDuration(seconds: number | null | undefined): string {
  if (seconds == null || !isFinite(seconds) || seconds < 0) return "—";
  const s = Math.round(seconds);
  if (s < 60) return `${s}s`;
  const m = Math.floor(s / 60);
  if (m < 60) return `${m}m ${s % 60}s`;
  return `${Math.floor(m / 60)}h ${m % 60}m`;
}

export function formatDate(iso: string | null | undefined): string {
  if (!iso) return "—";
  const d = new Date(iso);
  if (isNaN(d.getTime())) return "—";
  return d.toLocaleString(undefined, { dateStyle: "medium", timeStyle: "short" });
}

export function relativeTime(iso: string | null | undefined, now: Date = new Date()): string {
  if (!iso) return "never";
  const d = new Date(iso);
  if (isNaN(d.getTime())) return "—";
  const diff = (now.getTime() - d.getTime()) / 1000;
  if (diff < 45) return "just now";
  if (diff < 3600) return `${Math.round(diff / 60)} min ago`;
  if (diff < 86400) return `${Math.round(diff / 3600)} h ago`;
  if (diff < 86400 * 7) return `${Math.round(diff / 86400)} d ago`;
  return d.toLocaleDateString();
}

export function isHexColor(s: string): boolean {
  return /^#[0-9a-fA-F]{6}$/.test(s);
}

/** Mix a hex color toward white by `amount` (0..1). */
export function lighten(hex: string, amount: number): string {
  if (!isHexColor(hex)) return hex;
  const n = parseInt(hex.slice(1), 16);
  const ch = (shift: number) => {
    const c = (n >> shift) & 255;
    return Math.round(c + (255 - c) * amount);
  };
  return "#" + [16, 8, 0].map((s) => ch(s).toString(16).padStart(2, "0")).join("");
}

/** Parse what the user typed into the flag value field into a JSON value. */
export function parseFlagInput(raw: string, type: string): unknown {
  const t = raw.trim();
  if (type === "boolean") {
    if (/^true$/i.test(t)) return true;
    if (/^false$/i.test(t)) return false;
    return t;
  }
  if (type === "integer" || type === "log") {
    return /^-?\d+$/.test(t) ? Number(t) : t;
  }
  if (type === "string") return raw;
  if (/^(true|false)$/i.test(t)) return /^true$/i.test(t);
  if (/^-?\d+(\.\d+)?$/.test(t)) return Number(t);
  return raw;
}

export function flagTypeOf(name: string): "boolean" | "integer" | "string" | "log" | "unknown" {
  const prefixes: [string, "boolean" | "integer" | "string" | "log"][] = [
    ["DFFlag", "boolean"], ["SFFlag", "boolean"], ["FFlag", "boolean"],
    ["DFInt", "integer"], ["SFInt", "integer"], ["FInt", "integer"],
    ["DFString", "string"], ["SFString", "string"], ["FString", "string"],
    ["DFLog", "log"], ["FLog", "log"],
  ];
  for (const [p, t] of prefixes) if (name.startsWith(p) && name.length > p.length) return t;
  return "unknown";
}

export function displayValue(v: unknown): string {
  if (typeof v === "string") return v;
  return JSON.stringify(v);
}
