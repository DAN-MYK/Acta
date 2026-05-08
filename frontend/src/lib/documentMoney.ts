import type { DocumentDraftItemDto } from "./types";

interface DecimalValue {
  value: bigint;
  scale: number;
}

function pow10(exponent: number): bigint {
  let result = 1n;

  for (let index = 0; index < exponent; index += 1) {
    result *= 10n;
  }

  return result;
}

function parseDecimal(value: string): DecimalValue | null {
  const normalized = value.replace(/\s+/g, "").replace(",", ".").trim();
  if (!normalized) {
    return null;
  }

  const match = normalized.match(/^(-?)(\d+)(?:\.(\d+))?$/);
  if (!match) {
    return null;
  }

  const [, sign, integerPart, fractionalPart = ""] = match;
  const digits = `${integerPart}${fractionalPart}`.replace(/^0+(?=\d)/, "") || "0";

  return {
    value: sign === "-" ? -BigInt(digits) : BigInt(digits),
    scale: fractionalPart.length
  };
}

function multiplyDecimals(left: string, right: string): DecimalValue | null {
  const leftDecimal = parseDecimal(left);
  const rightDecimal = parseDecimal(right);

  if (!leftDecimal || !rightDecimal) {
    return null;
  }

  return {
    value: leftDecimal.value * rightDecimal.value,
    scale: leftDecimal.scale + rightDecimal.scale
  };
}

function addDecimalValues(current: DecimalValue, next: DecimalValue): DecimalValue {
  if (current.scale === next.scale) {
    return {
      value: current.value + next.value,
      scale: current.scale
    };
  }

  if (current.scale > next.scale) {
    return {
      value: current.value + next.value * pow10(current.scale - next.scale),
      scale: current.scale
    };
  }

  return {
    value: current.value * pow10(next.scale - current.scale) + next.value,
    scale: next.scale
  };
}

function formatScaledMoney(decimal: DecimalValue): string {
  const negative = decimal.value < 0n;
  const absoluteValue = negative ? -decimal.value : decimal.value;
  let roundedMinorUnits: bigint;

  if (decimal.scale > 2) {
    const divisor = pow10(decimal.scale - 2);
    roundedMinorUnits = (absoluteValue + divisor / 2n) / divisor;
  } else if (decimal.scale < 2) {
    roundedMinorUnits = absoluteValue * pow10(2 - decimal.scale);
  } else {
    roundedMinorUnits = absoluteValue;
  }

  const integerPart = roundedMinorUnits / 100n;
  const fractionalPart = (roundedMinorUnits % 100n).toString().padStart(2, "0");
  const groupedIntegerPart = integerPart.toString().replace(/\B(?=(\d{3})+(?!\d))/g, " ");

  return `${negative ? "-" : ""}${groupedIntegerPart},${fractionalPart} грн`;
}

export function formatDocumentItemTotal(quantity: string, price: string): string {
  const total = multiplyDecimals(quantity, price);
  return total ? formatScaledMoney(total) : "—";
}

export function formatDocumentDraftTotal(items: DocumentDraftItemDto[]): string {
  let total: DecimalValue = { value: 0n, scale: 0 };

  for (const item of items) {
    const itemTotal = multiplyDecimals(item.quantity, item.price);
    if (!itemTotal) {
      continue;
    }

    total = addDecimalValues(total, itemTotal);
  }

  return formatScaledMoney(total);
}
