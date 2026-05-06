const MINOR_SCALE = 2;
const MINOR_UNIT = 100n;

function pow10(exponent: number): bigint {
  let result = 1n;
  for (let index = 0; index < exponent; index += 1) {
    result *= 10n;
  }
  return result;
}

function normalizeMoneyInput(raw: string): string | null {
  let normalized = "";
  for (const ch of raw) {
    const code = ch.charCodeAt(0);
    if (code === 0x00a0 || code === 0x202f || code === 0x2009 || code === 0x200b) {
      continue;
    }
    if (ch === " " || ch === "\t" || ch === "\n" || ch === "\r") {
      continue;
    }
    normalized += ch;
  }

  normalized = normalized.replace(/(?:грн|uah|₴)/gi, "");
  if (!normalized) {
    return null;
  }

  return normalized.replace(",", ".");
}

export function parseMoneyToMinor(value: string): bigint | null {
  if (typeof value !== "string") {
    return null;
  }

  const normalized = normalizeMoneyInput(value);
  if (normalized === null) {
    return null;
  }

  const match = normalized.match(/^(-?)(\d+)(?:\.(\d+))?$/);
  if (!match) {
    return null;
  }

  const [, sign, integerPart, fractionalPart = ""] = match;
  const negative = sign === "-";

  const minorIntegerPart = BigInt(integerPart) * MINOR_UNIT;

  let minorFractional = 0n;
  if (fractionalPart.length === 0) {
    minorFractional = 0n;
  } else if (fractionalPart.length <= MINOR_SCALE) {
    const digits = fractionalPart.padEnd(MINOR_SCALE, "0");
    minorFractional = BigInt(digits);
  } else {
    const keep = fractionalPart.slice(0, MINOR_SCALE);
    const tail = fractionalPart.slice(MINOR_SCALE);
    const tailValue = BigInt(tail);
    const halfMark = pow10(tail.length - 1) * 5n;
    let rounded = BigInt(keep);
    if (tailValue >= halfMark) {
      rounded += 1n;
    }
    minorFractional = rounded;
  }

  const total = minorIntegerPart + minorFractional;
  return negative ? -total : total;
}

export function formatMinorMoney(minor: bigint): string {
  const negative = minor < 0n;
  const absolute = negative ? -minor : minor;
  const integerPart = absolute / MINOR_UNIT;
  const fractionalPart = (absolute % MINOR_UNIT).toString().padStart(2, "0");
  const grouped = integerPart.toString().replace(/\B(?=(\d{3})+(?!\d))/g, " ");
  return `${negative ? "-" : ""}${grouped},${fractionalPart}`;
}

export function addMinor(...values: bigint[]): bigint {
  let total = 0n;
  for (const value of values) {
    total += value;
  }
  return total;
}

export function subMinor(a: bigint, b: bigint): bigint {
  return a - b;
}

export function compareMinor(a: bigint, b: bigint): number {
  if (a < b) return -1;
  if (a > b) return 1;
  return 0;
}

export function isFormattedMoneyNegative(value: string): boolean {
  return /^[-(]/.test(value.trim());
}
