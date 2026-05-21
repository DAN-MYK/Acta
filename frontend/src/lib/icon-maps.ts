import type { AppIconName } from "./icons";
import type {
  DocumentKind,
  PaletteItemDto,
  ScreenId,
  SettingsSection,
  TaskPriority,
  TaskStatus
} from "./types";

export const documentKindLabels: Record<DocumentKind, string> = {
  invoice: "Рахунок",
  act: "Акт",
  waybill: "Накладна",
  adjustment_act: "Акт коригування"
};

export const documentKindIcons: Record<DocumentKind, AppIconName> = {
  invoice: "invoice",
  act: "act",
  waybill: "waybill",
  adjustment_act: "act"
};

export const screenIcons: Record<ScreenId, AppIconName> = {
  dashboard: "dashboard",
  documents: "documents",
  counterparties: "counterparties",
  payments: "payments",
  reports: "reports",
  tasks: "tasks",
  settings: "settings"
};

function getDocumentKindIconOrNull(kind: string): AppIconName | null {
  const icon = getDocumentKindIcon(kind);
  return icon === "documents" ? null : icon;
}

export function getScreenIcon(screen: string): AppIconName {
  const normalized = screen.toLowerCase() as ScreenId;
  return screenIcons[normalized] ?? "palette";
}

export function getPaletteItemIcon(item: PaletteItemDto): AppIconName {
  const kind = item.kind.toLowerCase();
  const payload = item.payload.toLowerCase();
  const content = `${item.title} ${item.subtitle} ${item.payload}`.toLowerCase();
  const documentIcon = getDocumentKindIconOrNull(content);

  if (kind === "navigate" || payload.startsWith("nav:")) {
    const [, screen = ""] = payload.split(":");
    return getScreenIcon(screen);
  }

  if (kind === "open_document") {
    return documentIcon ?? "documents";
  }

  if (kind === "create_document_draft") {
    return documentIcon ?? "add";
  }

  if (kind === "open_counterparty" || kind === "open_counterparty_create") {
    return content.includes("компан") ? "company" : "counterparties";
  }

  if (kind === "navigate_for_counterparty_selection") {
    return "counterparties";
  }

  return documentIcon ?? "palette";
}

export function getDocumentKindIcon(kind: string): AppIconName {
  const normalized = kind.toLowerCase();
  if (normalized === "invoice" || normalized.includes("рах")) return "invoice";
  if (normalized === "act" || normalized.includes("акт")) return "act";
  if (normalized === "waybill" || normalized.includes("наклад")) return "waybill";
  if (normalized === "adjustment_act" || normalized.includes("кориг")) return "act";
  if (normalized.includes("догов")) return "contract";
  if (normalized.includes("pdf")) return "pdf";
  if (normalized.includes("excel") || normalized.includes("xls")) return "excel";
  return "documents";
}

export function getDocumentKindLabel(kind: string): string {
  const normalized = kind.toLowerCase();
  if (normalized === "invoice" || normalized.includes("рах")) return "Рахунок";
  if (normalized === "act" || normalized.includes("акт")) return "Акт";
  if (normalized === "waybill" || normalized.includes("наклад")) return "Накладна";
  if (normalized === "adjustment_act" || normalized.includes("кориг")) return "Акт коригування";
  if (normalized.includes("догов")) return "Договір";
  if (normalized.includes("pdf")) return "PDF";
  if (normalized.includes("excel") || normalized.includes("xls")) return "Excel";
  return kind;
}

export const settingsSectionIcons: Record<SettingsSection, AppIconName> = {
  appearance: "appearance",
  company: "company",
  numbering: "numbering",
  integrations: "integrations",
  team: "team",
  backup: "backup"
};

export const taskStatusIcons: Record<TaskStatus, AppIconName> = {
  open: "openStatus",
  in_progress: "progressStatus",
  done: "doneStatus",
  cancelled: "close"
};

export const taskPriorityIcons: Record<TaskPriority, AppIconName> = {
  low: "priorityLow",
  normal: "priorityNormal",
  high: "priorityHigh",
  critical: "priorityCritical"
};
