import act from "./act.svg?raw";
import add from "./add.svg?raw";
import address from "./address.svg?raw";
import allStatus from "./all-status.svg?raw";
import appearance from "./appearance.svg?raw";
import archive from "./archive.svg?raw";
import backup from "./backup.svg?raw";
import bank from "./bank.svg?raw";
import calendar from "./calendar.svg?raw";
import close from "./close.svg?raw";
import contract from "./contract.svg?raw";
import company from "./company.svg?raw";
import counterparties from "./counterparties.svg?raw";
import dashboard from "./dashboard.svg?raw";
import deleteIcon from "./delete.svg?raw";
import doneStatus from "./done-status.svg?raw";
import documents from "./documents.svg?raw";
import edit from "./edit.svg?raw";
import email from "./email.svg?raw";
import excel from "./excel.svg?raw";
import expense from "./expense.svg?raw";
import exportIcon from "./export.svg?raw";
import filter from "./filter.svg?raw";
import iban from "./iban.svg?raw";
import importIcon from "./import.svg?raw";
import income from "./income.svg?raw";
import integrations from "./integrations.svg?raw";
import invoice from "./invoice.svg?raw";
import net from "./net.svg?raw";
import numbering from "./numbering.svg?raw";
import openLink from "./open-link.svg?raw";
import openStatus from "./open-status.svg?raw";
import palette from "./palette.svg?raw";
import payments from "./payments.svg?raw";
import pdf from "./pdf.svg?raw";
import phone from "./phone.svg?raw";
import priorityCritical from "./priority-critical.svg?raw";
import priorityHigh from "./priority-high.svg?raw";
import priorityLow from "./priority-low.svg?raw";
import priorityNormal from "./priority-normal.svg?raw";
import progressStatus from "./progress-status.svg?raw";
import reconcile from "./reconcile.svg?raw";
import refresh from "./refresh.svg?raw";
import reports from "./reports.svg?raw";
import save from "./save.svg?raw";
import search from "./search.svg?raw";
import settings from "./settings.svg?raw";
import sync from "./sync.svg?raw";
import tasks from "./tasks.svg?raw";
import team from "./team.svg?raw";
import template from "./template.svg?raw";
import theme from "./theme.svg?raw";
import today from "./today.svg?raw";
import unmatched from "./unmatched.svg?raw";
import unreconcile from "./unreconcile.svg?raw";
import waybill from "./waybill.svg?raw";

export const appIcons = {
  act,
  add,
  address,
  allStatus,
  appearance,
  archive,
  backup,
  bank,
  calendar,
  close,
  company,
  contract,
  counterparties,
  dashboard,
  delete: deleteIcon,
  doneStatus,
  documents,
  edit,
  email,
  excel,
  expense,
  export: exportIcon,
  filter,
  iban,
  import: importIcon,
  income,
  integrations,
  invoice,
  net,
  numbering,
  openLink,
  openStatus,
  palette,
  payments,
  pdf,
  phone,
  priorityCritical,
  priorityHigh,
  priorityLow,
  priorityNormal,
  progressStatus,
  reconcile,
  refresh,
  reports,
  save,
  search,
  settings,
  sync,
  tasks,
  team,
  template,
  theme,
  today,
  unmatched,
  unreconcile,
  waybill
} as const;

export type AppIconName = keyof typeof appIcons;
