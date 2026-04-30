import { beforeEach, describe, expect, it, vi } from "vitest";
import type {
  CounterpartyDetailScreenDto,
  CounterpartyEditorDto,
  CounterpartiesScreenDto,
  PaymentsScreenDto,
  SettingsScreenDto
} from "../../types";
import { invokeMock, loadStores, snapshot } from "./helpers";

function makeCounterparties(ids: string[]): CounterpartiesScreenDto {
  return {
    items: ids.map((id, index) => ({
      id,
      name: `Контрагент ${index + 1}`,
      edrpou: `1234567${index}`,
      kind: "",
      balanceStr: "0,00 грн",
      docCount: 0,
      overdueCount: 0
    }))
  };
}

function makeCounterpartyDetail(id: string): CounterpartyDetailScreenDto {
  return {
    info: {
      id,
      name: `Контрагент ${id}`,
      kind: "",
      edrpou: "12345678",
      ipn: "",
      vat: "",
      iban: "UA123456789012345678901234567",
      bank: "",
      address: "м. Київ",
      director: "",
      phone: "",
      email: "",
      clientSince: "",
      balanceStr: "0,00 грн",
      balanceIsNegative: false,
      docCount: 0,
      overdueCount: 0,
      overdueAmountStr: "0,00 грн",
      lastContactDays: 0,
      lastContactDate: ""
    },
    documents: [],
    payments: []
  };
}

function makeCounterpartyEditor(id = ""): CounterpartyEditorDto {
  return {
    form: {
      id,
      title: id ? "Редагування контрагента" : "Новий контрагент",
      name: id ? `Контрагент ${id}` : "",
      edrpou: "",
      ipn: "",
      iban: "",
      address: "",
      phone: "",
      email: "",
      notes: ""
    },
    showEditor: true
  };
}

function makePayments(ids: string[]): PaymentsScreenDto {
  return {
    items: ids.map((id, index) => ({
      id,
      date: "2026-04-30",
      counterparty: `Контрагент ${index + 1}`,
      amountStr: `${index + 1} 000,00 грн`,
      direction: index % 2 === 0 ? "in" : "out",
      matchedDoc: "",
      account: "ПриватБанк"
    })),
    counterparties: [{ id: "cp-1", name: "Контрагент 1" }],
    kpi: {
      incomingStr: "1 000,00 грн",
      outgoingStr: "0,00 грн",
      netStr: "1 000,00 грн",
      unmatchedStr: "1 000,00 грн",
      incomingSub: "Надходження",
      outgoingSub: "Витрати",
      unmatchedCount: ids.length
    }
  };
}

function makeSettingsScreen(): SettingsScreenDto {
  return {
    company: {
      fullName: "ТОВ Акт",
      shortName: "Акт",
      edrpou: "12345678",
      ipn: "",
      address: "м. Київ",
      director: "Іваненко І.І.",
      iban: "UA123456789012345678901234567",
      bank: "ПриватБанк",
      vatRegistered: false,
      vatCert: ""
    },
    integrations: [
      {
        label: "BAS",
        description: "Експорт BAS",
        tag: "bas",
        enabled: false
      }
    ],
    team: [],
    numbering: [
      {
        docType: "act",
        template: "ACT-{yyyy}-{n}",
        example: "ACT-2026-0001",
        nextNumber: "1"
      }
    ],
    preferences: {
      darkMode: false,
      density: 1
    },
    backup: {
      label: "Остання копія",
      file: "backup.zip",
      kind: "manual",
      note: "OK",
      tone: "info"
    }
  };
}

describe("frontend Tauri store smoke: counterparties + payments + settings", () => {
  beforeEach(() => {
    vi.resetModules();
    invokeMock.mockReset();
  });

  it("covers counterparties load, detail, editor, save, archive and create-document context", async () => {
    const { counterpartiesStore, documentsStore, navigationStore } = await loadStores();

    let screen = makeCounterparties(["cp-1", "cp-2"]);
    let currentDetail = makeCounterpartyDetail("cp-1");

    invokeMock.mockImplementation(async (command, payload) => {
      switch (command) {
        case "counterparties_list":
          return screen;
        case "counterparty_get":
          return makeCounterpartyDetail((payload as { counterpartyId: string }).counterpartyId);
        case "counterparty_open_editor":
          return makeCounterpartyEditor((payload as { counterpartyId: string | null }).counterpartyId ?? "");
        case "counterparty_save":
          currentDetail = makeCounterpartyDetail("cp-3");
          screen = makeCounterparties(["cp-1", "cp-2", "cp-3"]);
          return {
            ok: true,
            savedId: "cp-3",
            message: "Контрагента збережено",
            updatedList: screen.items,
            updatedDetail: currentDetail
          };
        case "counterparty_archive":
          screen = makeCounterparties(["cp-2"]);
          return {
            ok: true,
            message: "Контрагента архівовано"
          };
        case "counterparty_create_document_context":
          expect(payload).toEqual({ counterpartyId: "cp-2" });
          return {
            counterpartyId: "cp-2",
            counterpartyName: "Контрагент cp-2"
          };
        default:
          throw new Error(`unexpected command: ${command}`);
      }
    });

    await counterpartiesStore.load();
    expect(snapshot(counterpartiesStore).selectedId).toBe("cp-1");
    expect(snapshot(counterpartiesStore).detail?.info.id).toBe("cp-1");

    await counterpartiesStore.open("cp-2");
    expect(snapshot(counterpartiesStore).detail?.info.id).toBe("cp-2");

    await counterpartiesStore.openEditor();
    counterpartiesStore.updateFormField("name", "Новий контрагент");
    await counterpartiesStore.save();
    expect(snapshot(counterpartiesStore).selectedId).toBe("cp-3");
    expect(snapshot(counterpartiesStore).message).toBe("Контрагента збережено");

    await counterpartiesStore.archiveCurrent();
    expect(snapshot(counterpartiesStore).selectedId).toBe("cp-2");
    expect(snapshot(counterpartiesStore).message).toBe("Контрагента архівовано");

    await counterpartiesStore.createDocument();
    expect(snapshot(navigationStore)).toBe("documents");
    expect(snapshot(documentsStore).draftContext).toEqual({
      counterpartyId: "cp-2",
      counterpartyName: "Контрагент cp-2"
    });
  });

  it("covers payments load, editor, save, reconcile, import/sync and template flow", async () => {
    const { paymentsStore } = await loadStores();

    let list = makePayments(["pay-1"]);

    invokeMock.mockImplementation(async (command) => {
      switch (command) {
        case "payments_list":
          return list;
        case "payment_create_or_update":
          list = makePayments(["pay-1", "pay-2"]);
          return {
            ok: true,
            message: "Платіж збережено"
          };
        case "payment_reconcile":
          return {
            ok: true,
            message: "Платіж зведено"
          };
        case "payment_unreconcile":
          return {
            ok: true,
            message: "Зведення скасовано"
          };
        case "payments_import_latest_csv":
          list = makePayments(["pay-1", "pay-2", "pay-3"]);
          return {
            ok: true,
            message: "CSV імпортовано"
          };
        case "payments_sync_bank":
          return {
            ok: true,
            message: "Банк синхронізовано"
          };
        case "payments_open_manual_template":
          return {
            ok: true,
            path: "storage/import/bank/manual-template.csv",
            message: "Шаблон відкрито"
          };
        default:
          throw new Error(`unexpected command: ${command}`);
      }
    });

    await paymentsStore.load();
    expect(snapshot(paymentsStore).list?.items).toHaveLength(1);

    const emptySave = await paymentsStore.save();
    expect(emptySave.ok).toBe(false);
    expect(invokeMock.mock.calls.filter(([command]) => command === "payment_create_or_update")).toHaveLength(0);

    paymentsStore.openEditor();
    paymentsStore.updateFormField("amount", "1000.00");
    await paymentsStore.save();
    expect(snapshot(paymentsStore).list?.items).toHaveLength(2);
    expect(snapshot(paymentsStore).message).toBe("Платіж збережено");

    await paymentsStore.reconcile("pay-2");
    expect(snapshot(paymentsStore).message).toBe("Платіж зведено");

    await paymentsStore.unreconcile("pay-2");
    expect(snapshot(paymentsStore).message).toBe("Зведення скасовано");

    const importResult = await paymentsStore.importCsv();
    expect(importResult.ok).toBe(true);
    expect(snapshot(paymentsStore).list?.items).toHaveLength(3);

    const syncResult = await paymentsStore.syncBank();
    expect(syncResult.ok).toBe(true);
    expect(snapshot(paymentsStore).message).toBe("Банк синхронізовано");

    expect(invokeMock.mock.calls.filter(([command]) => command === "payments_list")).toHaveLength(6);

    const templateResult = await paymentsStore.openManualTemplate();
    expect(templateResult.path).toContain("manual-template.csv");
  });

  it("covers settings load, preferences, company, integrations, team and backup flows", async () => {
    const { settingsStore } = await loadStores();

    let screen = makeSettingsScreen();

    invokeMock.mockImplementation(async (command) => {
      switch (command) {
        case "settings_load":
          return screen;
        case "settings_save_preferences":
          screen = {
            ...screen,
            preferences: {
              darkMode: true,
              density: 2
            }
          };
          return {
            ok: true,
            message: "Налаштування вигляду збережено",
            screen
          };
        case "settings_save_company":
          screen = {
            ...screen,
            company: {
              ...screen.company,
              fullName: "ТОВ Акт Плюс"
            }
          };
          return {
            ok: true,
            message: "Компанію збережено",
            screen
          };
        case "settings_configure_integration":
          screen = {
            ...screen,
            integrations: screen.integrations.map((item) =>
              item.tag === "bas" ? { ...item, enabled: true } : item
            )
          };
          return {
            ok: true,
            message: "Інтеграцію оновлено",
            screen
          };
        case "settings_team_invite":
          screen = {
            ...screen,
            team: [
              {
                name: "Олена",
                email: "olena@example.com",
                role: "admin",
                lastActive: "щойно"
              }
            ]
          };
          return {
            ok: true,
            message: "Запрошення створено",
            screen
          };
        case "settings_backup_now":
          return {
            ok: true,
            message: "Резервну копію створено",
            screen
          };
        case "settings_backup_open_latest":
          return {
            ok: true,
            message: "Резервну копію відкрито",
            path: "storage/backups/latest.zip"
          };
        default:
          throw new Error(`unexpected command: ${command}`);
      }
    });

    await settingsStore.load();
    expect(snapshot(settingsStore).screen?.company.fullName).toBe("ТОВ Акт");

    settingsStore.updatePreference("darkMode", true);
    settingsStore.updatePreference("density", 2);
    await settingsStore.savePreferences();
    expect(snapshot(settingsStore).screen?.preferences).toEqual({
      darkMode: true,
      density: 2
    });

    settingsStore.updateCompanyField("fullName", "ТОВ Акт Плюс");
    await settingsStore.saveCompany();
    expect(snapshot(settingsStore).screen?.company.fullName).toBe("ТОВ Акт Плюс");

    await settingsStore.configureIntegration("bas");
    expect(snapshot(settingsStore).screen?.integrations[0]?.enabled).toBe(true);

    await settingsStore.inviteTeam();
    expect(snapshot(settingsStore).screen?.team).toHaveLength(1);

    await settingsStore.backupNow();
    expect(snapshot(settingsStore).message).toBe("Резервну копію створено");

    await settingsStore.openLatestBackup();
    expect(snapshot(settingsStore).message).toContain("storage/backups/latest.zip");
  });
});
