const DAY_IN_MS = 24 * 60 * 60 * 1000;

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
