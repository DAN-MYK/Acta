import type {
  BulkMutationResultDto,
  CounterpartyDetailScreenDto,
  CounterpartyEditorDto,
  CounterpartyItemDto,
  CounterpartiesScreenDto,
  CounterpartySaveResultDto,
  CreateDocumentContextDto,
  DashboardScreenDto,
  DocumentChainDto,
  DocumentEditorDto,
  DocumentItemDto,
  DocumentsListDto,
  ImportPlanDto,
  ImportResultDto,
  MutationResultDto,
  OpenTemplateResultDto,
  PaletteActivationResultDto,
  PaletteItemDto,
  PaletteSearchResultDto,
  PaymentItemDto,
  ReportsExportResultDto,
  ReportsFilterDto,
  ReportsScreenDto,
  SaveDocumentResponse,
  SettingsActionResultDto,
  SettingsScreenDto,
  SettingsScreenMutationResultDto,
  ShellStateDto,
  TaskEditorDto,
  TaskMutationResultDto,
  TaskSaveResultDto,
  TasksScreenDto,
  PaymentsScreenDto
} from "./types";

const state = {
  activeCompanyId: "company-act",
  darkMode: false,
  density: 1
};

const companies = [
  {
    id: "company-act",
    name: "ТОВ Акт",
    subtitle: "Консалтинг та документи",
    badge: "Активна",
    initials: "А"
  },
  {
    id: "company-flow",
    name: "ТОВ Потік",
    subtitle: "Казначейство та звірка",
    badge: "У фокусі",
    initials: "П"
  }
] as const;

const counterparties: CounterpartyItemDto[] = [
  {
    id: "cp-1",
    name: "ТОВ Ромашка",
    edrpou: "12345678",
    kind: "Клієнт",
    balanceStr: "48 200,00 грн",
    docCount: 6,
    overdueCount: 0
  },
  {
    id: "cp-2",
    name: "ФОП Петренко",
    edrpou: "87654321",
    kind: "Постачальник",
    balanceStr: "-19 000,00 грн",
    docCount: 4,
    overdueCount: 1
  }
];

const documents: DocumentItemDto[] = [
  {
    id: "doc-1",
    kind: "invoice",
    number: "INV-2026-0042",
    date: "2026-05-01",
    counterparty: "ТОВ Ромашка",
    amountStr: "48 200,00 грн",
    status: "issued",
    statusLabel: "Виставлено",
    linkedId: "chain-1"
  },
  {
    id: "doc-2",
    kind: "act",
    number: "ACT-2026-0018",
    date: "2026-04-29",
    counterparty: "ФОП Петренко",
    amountStr: "19 400,00 грн",
    status: "draft",
    statusLabel: "Чернетка",
    linkedId: "chain-2"
  }
];

const payments: PaymentItemDto[] = [
  {
    id: "pay-1",
    date: "2026-05-01",
    counterpartyId: "cp-1",
    counterparty: "ТОВ Ромашка",
    amountStr: "48 200,00 грн",
    direction: "in",
    matchedDoc: "INV-2026-0042",
    account: "ПриватБанк"
  },
  {
    id: "pay-2",
    date: "2026-04-30",
    counterpartyId: "cp-2",
    counterparty: "ФОП Петренко",
    amountStr: "19 000,00 грн",
    direction: "out",
    matchedDoc: "",
    account: "mono"
  }
];

function clone<T>(value: T): T {
  return JSON.parse(JSON.stringify(value)) as T;
}

function shellState(): ShellStateDto {
  const active = companies.find((company) => company.id === state.activeCompanyId) ?? companies[0];

  return {
    chrome: {
      companyName: active.name,
      userName: "Михайло Данилюк",
      userInitials: "МД",
      userRole: active.subtitle,
      documentsBadge: 4,
      tasksBadge: 2
    },
    companyItems: companies.map((company) => ({
      id: company.id,
      name: company.name,
      subtitle: company.subtitle,
      initials: company.initials,
      badge: company.badge,
      active: company.id === active.id
    })),
    activeCompanyId: active.id,
    isDark: state.darkMode
  };
}

function documentsList(): DocumentsListDto {
  return {
    items: clone(documents),
    invoiceItems: clone(documents.filter((item) => item.kind === "invoice")),
    actItems: clone(documents.filter((item) => item.kind === "act")),
    waybillItems: [],
    totalCount: documents.length,
    pageCount: 1
  };
}

function documentEditor(kind = "invoice", counterpartyId = "cp-1"): DocumentEditorDto {
  const counterparty = counterparties.find((item) => item.id === counterpartyId) ?? counterparties[0];

  return {
    form: {
      id: "doc-1",
      kind,
      counterpartyId: counterparty.id,
      counterpartyName: counterparty.name,
      title: kind === "invoice" ? "Рахунок INV-2026-0042" : "Документ у роботі",
      number: kind === "invoice" ? "INV-2026-0042" : "ACT-2026-0018",
      date: "2026-05-01",
      notes: "Погодити з контрагентом до кінця дня."
    },
    items: [
      {
        description: "Консультаційний супровід",
        unit: "год",
        quantity: "8,0000",
        price: "6025,00"
      }
    ],
    showTypePicker: false,
    showEditor: true
  };
}

function documentChain(): DocumentChainDto {
  return {
    sourceId: "doc-1",
    steps: [
      {
        docType: "invoice",
        docNumber: "INV-2026-0042",
        amountStr: "48 200,00 грн",
        status: "Виставлено",
        exists: true
      },
      {
        docType: "act",
        docNumber: "ACT-2026-0018",
        amountStr: "19 400,00 грн",
        status: "Чернетка",
        exists: true
      },
      {
        docType: "waybill",
        docNumber: "Ще не створено",
        amountStr: "—",
        status: "Очікує",
        exists: false
      }
    ]
  };
}

function reportsScreen(filter?: ReportsFilterDto): ReportsScreenDto {
  const resolvedFilter: ReportsFilterDto = {
    tab: filter?.tab ?? "bank",
    scope: filter?.scope ?? "active",
    dateFrom: filter?.dateFrom ?? "2026-02-01",
    dateTo: filter?.dateTo ?? "2026-05-01",
    query: filter?.query ?? ""
  };

  return {
    filter: resolvedFilter,
    summary: {
      openingBalanceStr: "125 000,00 грн",
      incomeStr: "48 200,00 грн",
      expenseStr: "19 000,00 грн",
      closingBalanceStr: "154 200,00 грн",
      receivablesTotalStr: "23 000,00 грн",
      payablesTotalStr: "14 500,00 грн"
    },
    bankRows: [
      {
        key: "ops",
        label: "Операційна діяльність",
        incomeStr: "48 200,00 грн",
        expenseStr: "19 000,00 грн",
        netStr: "29 200,00 грн"
      }
    ],
    receivablesRows: [
      {
        docId: "doc-1",
        docType: "invoice",
        docNumber: "INV-2026-0042",
        docDate: "2026-05-01",
        companyName: "ТОВ Акт",
        counterparty: "ТОВ Ромашка",
        amountStr: "48 200,00 грн",
        expectedDate: "2026-05-05",
        overdueDays: 0,
        status: "Очікується"
      }
    ],
    payablesRows: [
      {
        id: "payable-1",
        title: "Оплата послуг банку",
        companyName: "ТОВ Потік",
        counterparty: "mono",
        amountStr: "14 500,00 грн",
        dueDate: "2026-05-03",
        overdueDays: 0,
        recurrence: "Щомісяця"
      }
    ]
  };
}

function settingsScreen(): SettingsScreenDto {
  return {
    company: {
      fullName: "ТОВ Акт",
      shortName: "Акт",
      edrpou: "12345678",
      ipn: "3087654321",
      address: "м. Київ, вул. Хрещатик, 1",
      director: "Іваненко І.І.",
      iban: "UA123456789012345678901234567",
      bank: "ПриватБанк",
      vatRegistered: true,
      vatCert: "ПДВ № 100200300"
    },
    integrations: [
      {
        label: "BAS",
        description: "Обмін довідниками та документами з BAS",
        tag: "bas",
        enabled: true
      }
    ],
    team: [
      {
        name: "Михайло Данилюк",
        email: "mykhailo@example.com",
        role: "Адміністратор",
        lastActive: "Сьогодні"
      }
    ],
    numbering: [
      {
        docType: "Акт",
        template: "ACT-{yyyy}-{nnnn}",
        example: "ACT-2026-0042",
        nextNumber: "0043"
      }
    ],
    preferences: {
      darkMode: state.darkMode,
      density: state.density
    },
    backup: {
      label: "Остання резервна копія",
      file: "storage/backups/acta-2026-05-01.zip",
      kind: "Автоматична",
      note: "Створено 12 хвилин тому",
      tone: "success"
    }
  };
}

function dashboardScreen(): DashboardScreenDto {
  return {
    kpis: [
      { label: "Чистий рух", value: "29 200,00 грн", detail: "Поточний місяць", tone: "positive" },
      { label: "Дебіторка", value: "23 000,00 грн", detail: "Потребує контролю", tone: "warning" },
      { label: "Кредиторка", value: "14 500,00 грн", detail: "Планові виплати", tone: "accent" }
    ],
    cashflowRows: reportsScreen().bankRows,
    recentDocuments: clone(documents),
    upcomingPayments: [
      {
        id: "upcoming-1",
        dateLabel: "03 травня",
        contractor: "mono",
        amountStr: "14 500,00 грн",
        isOverdue: false
      }
    ],
    urgentTasks: [
      {
        id: "task-1",
        title: "Погодити оплату по INV-2026-0042",
        description: "Контроль підписання та оплати від ТОВ Ромашка",
        status: "open",
        statusLabel: "Відкрита",
        priority: "high",
        priorityLabel: "Високий",
        dueDate: "2026-05-02",
        reminderAt: "2026-05-01 15:00",
        linkKind: "document",
        linkLabel: "INV-2026-0042"
      }
    ]
  };
}

function tasksScreen(): TasksScreenDto {
  return {
    items: dashboardScreen().urgentTasks,
    openCount: 1,
    doneCount: 3,
    highCount: 1,
    todayCount: 1
  };
}

function paymentsScreen(): PaymentsScreenDto {
  return {
    items: clone(payments),
    counterparties: clone(counterparties.map((item) => ({ id: item.id, name: item.name }))),
    kpi: {
      incomingStr: "48 200,00 грн",
      outgoingStr: "19 000,00 грн",
      netStr: "29 200,00 грн",
      unmatchedStr: "19 000,00 грн",
      incomingSub: "Надходження",
      outgoingSub: "Витрати",
      unmatchedCount: 1
    }
  };
}

function paletteItems(): PaletteItemDto[] {
  return [
    {
      kind: "navigate",
      title: "Звіти",
      subtitle: "Гроші, дебіторка та кредиторка",
      shortcut: "Ctrl+5",
      payload: "screen:reports"
    },
    {
      kind: "navigate",
      title: "Платежі",
      subtitle: "Звірка та ручні платежі",
      shortcut: "Ctrl+4",
      payload: "screen:payments"
    },
    {
      kind: "open_document",
      title: "INV-2026-0042",
      subtitle: "ТОВ Ромашка · 48 200,00 грн",
      shortcut: "Документ",
      payload: "document:doc-1"
    }
  ];
}

export async function browserFixtureInvoke<T>(command: string, payload?: Record<string, unknown>): Promise<T> {
  switch (command) {
    case "shell_load":
      return clone(shellState()) as T;
    case "shell_set_active_company":
      state.activeCompanyId = String(payload?.companyId ?? state.activeCompanyId);
      return clone(shellState()) as T;
    case "shell_palette_search":
      return clone({
        items: paletteItems().filter((item) => {
          const query = String((payload?.request as { query?: string } | undefined)?.query ?? "").trim().toLowerCase();
          if (!query) {
            return true;
          }
          return `${item.title} ${item.subtitle}`.toLowerCase().includes(query);
        })
      } satisfies PaletteSearchResultDto) as T;
    case "shell_palette_activate":
      if (payload?.payload === "screen:reports") {
        return clone({
          kind: "navigate",
          screen: "reports",
          documentId: null,
          counterpartyId: null,
          documentEditor: null,
          message: "Відкрито звіти"
        } satisfies PaletteActivationResultDto) as T;
      }

      return clone({
        kind: "open_document",
        screen: "documents",
        documentId: "doc-1",
        counterpartyId: null,
        documentEditor: documentEditor(),
        message: "Відкрито документ"
      } satisfies PaletteActivationResultDto) as T;
    case "dashboard_load":
      return clone(dashboardScreen()) as T;
    case "documents_list":
      return clone(documentsList()) as T;
    case "document_open":
      return clone(documentEditor()) as T;
    case "document_create_draft":
      return clone(
        documentEditor(
          String((payload?.request as { kind?: string } | undefined)?.kind ?? "invoice"),
          String((payload?.request as { counterpartyId?: string } | undefined)?.counterpartyId ?? "cp-1")
        )
      ) as T;
    case "document_save":
      return clone({ documentId: "doc-1", kind: "invoice", message: "Документ збережено" } satisfies SaveDocumentResponse) as T;
    case "document_advance_status":
      return clone({ ok: true, documentId: "doc-1", message: "Статус оновлено" } satisfies MutationResultDto) as T;
    case "document_delete":
      return clone({ ok: true, documentId: "doc-1", message: "Документ видалено" } satisfies MutationResultDto) as T;
    case "document_chain_get":
      return clone(documentChain()) as T;
    case "document_chain_create_draft":
      return clone(documentEditor(String((payload?.request as { targetKind?: string } | undefined)?.targetKind ?? "act"))) as T;
    case "documents_bulk_delete":
      return clone({ total: 2, succeeded: 2, failed: 0, errors: [], message: "Вибрані документи видалено" } satisfies BulkMutationResultDto) as T;
    case "documents_bulk_advance_status":
      return clone({ total: 2, succeeded: 2, failed: 0, errors: [], message: "Статуси вибраних документів оновлено" } satisfies BulkMutationResultDto) as T;
    case "counterparties_list":
      return clone({ items: clone(counterparties) } satisfies CounterpartiesScreenDto) as T;
    case "counterparty_get":
      return clone({
        info: {
          id: "cp-1",
          name: "ТОВ Ромашка",
          kind: "Клієнт",
          edrpou: "12345678",
          ipn: "3087654321",
          vat: "ПДВ активний",
          iban: "UA123456789012345678901234567",
          bank: "ПриватБанк",
          address: "м. Київ, вул. Хрещатик, 1",
          director: "Іваненко І.І.",
          phone: "+380671112233",
          email: "office@example.com",
          clientSince: "2024-02-10",
          balanceStr: "48 200,00 грн",
          balanceIsNegative: false,
          docCount: 6,
          overdueCount: 0,
          overdueAmountStr: "0,00 грн",
          lastContactDays: 2,
          lastContactDate: "2026-04-29"
        },
        documents: clone(documents),
        payments: clone(payments)
      } satisfies CounterpartyDetailScreenDto) as T;
    case "counterparty_open_editor":
      return clone({
        form: {
          id: "",
          title: "Новий контрагент",
          name: "",
          edrpou: "",
          ipn: "",
          iban: "UA123456789012345678901234567",
          address: "м. Київ, вул. Хрещатик, 1",
          phone: "+380671112233",
          email: "office@example.com",
          notes: "Потребує уваги щодо термінів оплат."
        },
        showEditor: true
      } satisfies CounterpartyEditorDto) as T;
    case "counterparty_save":
      return clone({
        ok: true,
        savedId: "cp-1",
        message: "Контрагента збережено",
        updatedList: clone(counterparties),
        updatedDetail: null
      } satisfies CounterpartySaveResultDto) as T;
    case "counterparty_archive":
      return clone({ ok: true, message: "Контрагента архівовано" } satisfies MutationResultDto) as T;
    case "counterparty_create_document_context":
      return clone({
        counterpartyId: String(payload?.counterpartyId ?? "cp-1"),
        counterpartyName: "ТОВ Ромашка"
      } satisfies CreateDocumentContextDto) as T;
    case "tasks_list":
      return clone(tasksScreen()) as T;
    case "task_open_editor":
      return clone({
        title: "Нова задача",
        form: {
          id: "",
          title: "",
          description: "",
          priority: "normal",
          dueDate: "2026-05-02",
          reminderAt: "2026-05-01 15:00",
          status: "open",
          counterpartyId: "cp-1",
          actId: "",
          linkKind: "document",
          linkLabel: "INV-2026-0042"
        },
        showEditor: true
      } satisfies TaskEditorDto) as T;
    case "task_save":
      return clone({
        ok: true,
        savedId: "task-1",
        message: "Задачу збережено",
        updatedList: tasksScreen(),
        updatedEditor: null
      } satisfies TaskSaveResultDto) as T;
    case "task_delete":
      return clone({ ok: true, taskId: "task-1", message: "Задачу видалено" } satisfies TaskMutationResultDto) as T;
    case "task_set_status":
      return clone({ ok: true, taskId: "task-1", message: "Статус задачі оновлено" } satisfies TaskMutationResultDto) as T;
    case "reports_load":
      return clone(reportsScreen(payload?.request as ReportsFilterDto | undefined)) as T;
    case "reports_export_csv":
      return clone({ ok: true, path: "storage/reports/report-export.csv", message: "CSV експортовано" } satisfies ReportsExportResultDto) as T;
    case "settings_load":
      return clone(settingsScreen()) as T;
    case "settings_save_preferences":
      state.darkMode = Boolean((payload?.request as { darkMode?: boolean } | undefined)?.darkMode);
      state.density = Number((payload?.request as { density?: number } | undefined)?.density ?? state.density);
      return clone({ ok: true, message: "Налаштування вигляду збережено", screen: settingsScreen() } satisfies SettingsScreenMutationResultDto) as T;
    case "settings_save_company":
      return clone({ ok: true, message: "Дані компанії збережено", screen: settingsScreen() } satisfies SettingsScreenMutationResultDto) as T;
    case "settings_configure_integration":
      return clone({ ok: true, message: "Інтеграцію оновлено", screen: settingsScreen() } satisfies SettingsScreenMutationResultDto) as T;
    case "settings_team_invite":
      return clone({ ok: true, message: "Запрошення створено", screen: settingsScreen() } satisfies SettingsScreenMutationResultDto) as T;
    case "settings_backup_now":
      return clone({ ok: true, message: "Резервну копію створено", screen: settingsScreen() } satisfies SettingsScreenMutationResultDto) as T;
    case "settings_backup_open_latest":
      return clone({ ok: true, message: "Відкрито останню резервну копію", path: "storage/backups/acta-2026-05-01.zip" } satisfies SettingsActionResultDto) as T;
    case "payments_list":
      return clone(paymentsScreen()) as T;
    case "payments_import_latest_csv":
      return clone({ ok: true, message: "CSV імпортовано" } satisfies MutationResultDto) as T;
    case "payments_sync_bank":
      return clone({ ok: true, message: "Банк синхронізовано" } satisfies MutationResultDto) as T;
    case "payments_open_manual_template":
      return clone({ ok: true, path: "storage/import/bank/manual-template.csv", message: "Шаблон CSV відкрито" } satisfies OpenTemplateResultDto) as T;
    case "payment_create_or_update":
      return clone({ ok: true, message: "Платіж збережено" } satisfies MutationResultDto) as T;
    case "payment_reconcile":
      return clone({ ok: true, message: "Платіж зведено" } satisfies MutationResultDto) as T;
    case "payment_unreconcile":
      return clone({ ok: true, message: "Зведення скасовано" } satisfies MutationResultDto) as T;
    case "import_bas_pick_directory":
      return "C:\\tmp\\bas-export" as T;
    case "import_bas_plan":
      return clone({
        entities: [
          {
            entityType: "counterparties",
            fileName: "counterparties.xml",
            parsed: 15,
            willCreate: 10,
            willSkip: 5,
            error: null
          },
          {
            entityType: "payments",
            fileName: "bank_export.csv",
            parsed: 30,
            willCreate: 25,
            willSkip: 5,
            error: null
          }
        ]
      } satisfies ImportPlanDto) as T;
    case "import_bas_execute":
      return clone({
        entities: [
          {
            entityType: "counterparties",
            created: 10,
            updated: 2,
            skipped: 3,
            conflicts: 0,
            error: null
          }
        ]
      } satisfies ImportResultDto) as T;
    default:
      throw new Error(`Browser fixture for command "${command}" is not implemented.`);
  }
}
