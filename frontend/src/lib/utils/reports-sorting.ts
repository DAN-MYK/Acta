import type { PayableRowDto, ReceivableRowDto } from "../types";

const sortCollator = new Intl.Collator("uk", {
  numeric: true,
  sensitivity: "base"
});

export function compareStrings(left: string, right: string): number {
  return sortCollator.compare(left || "", right || "");
}

export function compareDates(left: string, right: string): number {
  return compareStrings(left || "9999-12-31", right || "9999-12-31");
}

export function parseMoneyValue(value: string): number {
  const normalized = value.replace(/\s+/g, "").replace("грн", "").replace(",", ".").trim();
  const parsed = Number.parseFloat(normalized);
  return Number.isFinite(parsed) ? parsed : 0;
}

export function stableSortRows<T>(rows: T[], compare: (left: T, right: T) => number): T[] {
  return rows
    .map((row, index) => ({ row, index }))
    .sort((left, right) => {
      const result = compare(left.row, right.row);
      return result !== 0 ? result : left.index - right.index;
    })
    .map(({ row }) => row);
}

export function compareReceivables(left: ReceivableRowDto, right: ReceivableRowDto): number {
  if (left.overdueDays !== right.overdueDays) {
    return right.overdueDays - left.overdueDays;
  }

  const dueDateOrder = compareDates(left.expectedDate, right.expectedDate);
  if (dueDateOrder !== 0) {
    return dueDateOrder;
  }

  const amountOrder = parseMoneyValue(right.amountStr) - parseMoneyValue(left.amountStr);
  if (amountOrder !== 0) {
    return amountOrder;
  }

  const counterpartyOrder = compareStrings(left.counterparty, right.counterparty);
  if (counterpartyOrder !== 0) {
    return counterpartyOrder;
  }

  return compareStrings(left.docNumber, right.docNumber);
}

export function comparePayables(left: PayableRowDto, right: PayableRowDto): number {
  if (left.overdueDays !== right.overdueDays) {
    return right.overdueDays - left.overdueDays;
  }

  const dueDateOrder = compareDates(left.dueDate, right.dueDate);
  if (dueDateOrder !== 0) {
    return dueDateOrder;
  }

  const amountOrder = parseMoneyValue(right.amountStr) - parseMoneyValue(left.amountStr);
  if (amountOrder !== 0) {
    return amountOrder;
  }

  const counterpartyOrder = compareStrings(left.counterparty, right.counterparty);
  if (counterpartyOrder !== 0) {
    return counterpartyOrder;
  }

  return compareStrings(left.title, right.title);
}

export function sortReceivables(rows: ReceivableRowDto[]): ReceivableRowDto[] {
  return stableSortRows(rows, compareReceivables);
}

export function sortPayables(rows: PayableRowDto[]): PayableRowDto[] {
  return stableSortRows(rows, comparePayables);
}
