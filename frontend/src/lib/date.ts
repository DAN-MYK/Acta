const DAY_IN_MS = 24 * 60 * 60 * 1000;

export function formatDate(value: string | null | undefined): string {
  if (!value) return "—";
  const parts = value.match(/^(\d{4})-(\d{2})-(\d{2})$/);
  if (!parts) return "—";
  const [, y, m, d] = parts;
  const month = Number(m);
  const day = Number(d);
  if (month < 1 || month > 12 || day < 1 || day > 31) return "—";
  return `${d}.${m}.${y}`;
}

export function daysUntil(dateValue: string, now = Date.now()): number | null {
  if (!dateValue) {
    return null;
  }

  const parsed = Date.parse(dateValue);
  if (Number.isNaN(parsed)) {
    return null;
  }

  return Math.ceil((parsed - now) / DAY_IN_MS);
}
