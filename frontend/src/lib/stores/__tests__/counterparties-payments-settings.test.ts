import { beforeEach, describe, expect, it, vi } from "vitest";
import appSource from "../../../App.svelte?raw";
import frontendApiSource from "../../api.ts?raw";
import tauriDocumentsSource from "../../../../../src-tauri/src/commands/documents.rs?raw";
import tauriLibSource from "../../../../../src-tauri/src/lib.rs?raw";
import type {
  CounterpartyDetailScreenDto,
  CounterpartyEditorDto,
  CounterpartiesScreenDto,
  PaymentsScreenDto,
  SettingsScreenDto
} from "../../types";
import { invokeMock, loadStores, snapshot } from "./helpers";

function normalizeMoneyText(value: string | undefined): string {
  return (value ?? "").replace(/\u00a0/g, " ");
}

function makeCounterparties(ids: string[]): CounterpartiesScreenDto {
  return {
    items: ids.map((id, index) => ({
      id,
      name: `РљРѕРЅС‚СЂР°РіРµРЅС‚ ${index + 1}`,
      edrpou: `1234567${index}`,
      kind: "",
      balanceStr: "0,00 РіСЂРЅ",
      docCount: 0,
      overdueCount: 0
    }))
  };
}

function makeCounterpartyDetail(id: string): CounterpartyDetailScreenDto {
  return {
    info: {
      id,
      name: `РљРѕРЅС‚СЂР°РіРµРЅС‚ ${id}`,
      kind: "",
      edrpou: "12345678",
      ipn: "",
      vat: "",
      iban: "UA123456789012345678901234567",
      bank: "",
      address: "Рј. РљРёС—РІ",
      director: "",
      phone: "",
      email: "",
      clientSince: "",
      balanceStr: "0,00 РіСЂРЅ",
      balanceIsNegative: false,
      docCount: 0,
      overdueCount: 0,
      overdueAmountStr: "0,00 РіСЂРЅ",
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
      title: id ? "Р РµРґР°РіСѓРІР°РЅРЅСЏ РєРѕРЅС‚СЂР°РіРµРЅС‚Р°" : "РќРѕРІРёР№ РєРѕРЅС‚СЂР°РіРµРЅС‚",
      name: id ? `РљРѕРЅС‚СЂР°РіРµРЅС‚ ${id}` : "",
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
      counterpartyId: `cp-${index + 1}`,
      counterparty: `РљРѕРЅС‚СЂР°РіРµРЅС‚ ${index + 1}`,
      amountStr: `${index + 1} 000,00 РіСЂРЅ`,
      direction: index % 2 === 0 ? "in" : "out",
      matchedDoc: "",
      account: "РџСЂРёРІР°С‚Р‘Р°РЅРє"
    })),
    counterparties: [{ id: "cp-1", name: "РљРѕРЅС‚СЂР°РіРµРЅС‚ 1" }],
    kpi: {
      incomingStr: "1 000,00 РіСЂРЅ",
      outgoingStr: "0,00 РіСЂРЅ",
      netStr: "1 000,00 РіСЂРЅ",
      unmatchedStr: "1 000,00 РіСЂРЅ",
      incomingSub: "РќР°РґС…РѕРґР¶РµРЅРЅСЏ",
      outgoingSub: "Р’РёС‚СЂР°С‚Рё",
      unmatchedCount: ids.length
    }
  };
}

function makeSettingsScreen(): SettingsScreenDto {
  return {
    company: {
      fullName: "РўРћР’ РђРєС‚",
      shortName: "РђРєС‚",
      edrpou: "12345678",
      ipn: "",
      address: "Рј. РљРёС—РІ",
      director: "Р†РІР°РЅРµРЅРєРѕ Р†.Р†.",
      iban: "UA123456789012345678901234567",
      bank: "РџСЂРёРІР°С‚Р‘Р°РЅРє",
      vatRegistered: false,
      vatCert: ""
    },
    integrations: [
      {
        label: "BAS",
        description: "Р•РєСЃРїРѕСЂС‚ BAS",
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
      darkMode: false
    },
    backup: {
      label: "РћСЃС‚Р°РЅРЅСЏ РєРѕРїС–СЏ",
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
            message: "РљРѕРЅС‚СЂР°РіРµРЅС‚Р° Р·Р±РµСЂРµР¶РµРЅРѕ",
            updatedList: screen.items,
            updatedDetail: currentDetail
          };
        case "counterparty_archive":
          screen = makeCounterparties(["cp-2"]);
          return {
            ok: true,
            message: "РљРѕРЅС‚СЂР°РіРµРЅС‚Р° Р°СЂС…С–РІРѕРІР°РЅРѕ"
          };
        case "counterparty_create_document_context":
          expect(payload).toEqual({ counterpartyId: "cp-2" });
          return {
            counterpartyId: "cp-2",
            counterpartyName: "РљРѕРЅС‚СЂР°РіРµРЅС‚ cp-2"
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
    counterpartiesStore.updateFormField("name", "РќРѕРІРёР№ РєРѕРЅС‚СЂР°РіРµРЅС‚");
    await counterpartiesStore.save();
    expect(snapshot(counterpartiesStore).selectedId).toBe("cp-3");
    expect(snapshot(counterpartiesStore).message).toBe("РљРѕРЅС‚СЂР°РіРµРЅС‚Р° Р·Р±РµСЂРµР¶РµРЅРѕ");

    await counterpartiesStore.archiveCurrent();
    expect(snapshot(counterpartiesStore).selectedId).toBe("cp-2");
    expect(snapshot(counterpartiesStore).message).toBe("РљРѕРЅС‚СЂР°РіРµРЅС‚Р° Р°СЂС…С–РІРѕРІР°РЅРѕ");

    await counterpartiesStore.createDocument();
    expect(snapshot(navigationStore)).toBe("documents");
    expect(snapshot(documentsStore).draftContext).toEqual({
      counterpartyId: "cp-2",
      counterpartyName: "РљРѕРЅС‚СЂР°РіРµРЅС‚ cp-2"
    });
  });

  it("covers payments load, editor, save, preview reconcile, import/sync and template flow", async () => {
    const { paymentsStore } = await loadStores();

    let list = makePayments(["pay-1"]);

    invokeMock.mockImplementation(async (command, payload) => {
      switch (command) {
        case "payments_list":
          return list;
        case "payment_create_or_update":
          list = makePayments(["pay-1", "pay-2"]);
          return {
            ok: true,
            message: "РџР»Р°С‚С–Р¶ Р·Р±РµСЂРµР¶РµРЅРѕ"
          };
        case "payment_match_preview":
          expect(payload).toEqual({ request: { paymentId: "pay-2" } });
          return {
            paymentId: "pay-2",
            isReconciled: false,
            decisionKind: "exact",
            candidates: [
              {
                documentId: "act-1",
                documentKind: "act",
                title: "РђРєС‚ ACT-001",
                openAmountStr: "1 000,00 РіСЂРЅ",
                totalScore: 0.98,
                sameIban: true,
                referenceHit: true,
                textHits: 2,
                daysDistance: 0
              }
            ],
            autoMatch: {
              documentId: "act-1",
              documentKind: "act",
              title: "РђРєС‚ ACT-001",
              amountStr: "1 000,00 РіСЂРЅ"
            }
          };
        case "payment_match_apply_auto":
          return {
            ok: true,
            message: "РђРІС‚РѕР·С–СЃС‚Р°РІР»РµРЅРЅСЏ РїР»Р°С‚РµР¶Сѓ Р·Р°СЃС‚РѕСЃРѕРІР°РЅРѕ"
          };
        case "payment_unreconcile_all":
          return {
            ok: true,
            message: "Р—РІРµРґРµРЅРЅСЏ СЃРєР°СЃРѕРІР°РЅРѕ"
          };
        case "payments_import_latest_csv":
          list = makePayments(["pay-1", "pay-2", "pay-3"]);
          return {
            ok: true,
            message: "CSV С–РјРїРѕСЂС‚РѕРІР°РЅРѕ"
          };
        case "payments_sync_bank":
          return {
            ok: true,
            message: "Р‘Р°РЅРє СЃРёРЅС…СЂРѕРЅС–Р·РѕРІР°РЅРѕ"
          };
        case "payments_open_manual_template":
          return {
            ok: true,
            path: "storage/import/bank/manual-template.csv",
            message: "РЁР°Р±Р»РѕРЅ РІС–РґРєСЂРёС‚Рѕ"
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
    expect(snapshot(paymentsStore).message).toBe("РџР»Р°С‚С–Р¶ Р·Р±РµСЂРµР¶РµРЅРѕ");

    const openedById = await paymentsStore.openById("pay-1");
    expect(openedById).toBe(true);
    expect(snapshot(paymentsStore).editor?.id).toBe("pay-1");
    expect(snapshot(paymentsStore).editor?.counterpartyId).toBe("cp-1");

    await paymentsStore.reconcile("pay-2");
    expect(snapshot(paymentsStore).matchPreview?.decisionKind).toBe("exact");
    expect(snapshot(paymentsStore).matchPreview?.autoMatch?.title).toBe("РђРєС‚ ACT-001");

    await paymentsStore.confirmPreviewAutoMatch();
    expect(snapshot(paymentsStore).message).toBe("РђРІС‚РѕР·С–СЃС‚Р°РІР»РµРЅРЅСЏ РїР»Р°С‚РµР¶Сѓ Р·Р°СЃС‚РѕСЃРѕРІР°РЅРѕ");
    expect(snapshot(paymentsStore).matchPreview).toBeNull();

    await paymentsStore.unreconcile("pay-2");
    expect(snapshot(paymentsStore).message).toBe("Р—РІРµРґРµРЅРЅСЏ СЃРєР°СЃРѕРІР°РЅРѕ");

    const importResult = await paymentsStore.importCsv();
    expect(importResult.ok).toBe(true);
    expect(snapshot(paymentsStore).list?.items).toHaveLength(3);

    const syncResult = await paymentsStore.syncBank();
    expect(syncResult.ok).toBe(true);
    expect(snapshot(paymentsStore).message).toBe("Р‘Р°РЅРє СЃРёРЅС…СЂРѕРЅС–Р·РѕРІР°РЅРѕ");

    expect(invokeMock.mock.calls.filter(([command]) => command === "payments_list")).toHaveLength(6);

    const templateResult = await paymentsStore.openManualTemplate();
    expect(templateResult.path).toContain("manual-template.csv");
  });

  it("opens reconcile preview before apply and persists manual confirm for ambiguous candidates", async () => {
    const { paymentsStore } = await loadStores();

    let list = makePayments(["pay-1", "pay-2", "pay-3"]);

    invokeMock.mockImplementation(async (command, payload) => {
      switch (command) {
        case "payments_list":
          return list;
        case "payment_match_preview": {
          const paymentId = (payload as { request: { paymentId: string } }).request.paymentId;

          if (paymentId === "pay-1") {
            return {
              paymentId,
              isReconciled: false,
              decisionKind: "exact",
              candidates: [
                {
                  documentId: "act-1",
                  documentKind: "act",
                  title: "РђРєС‚ ACT-001",
                  openAmountStr: "1 000,00 РіСЂРЅ",
                  totalScore: 0.99,
                  sameIban: true,
                  referenceHit: true,
                  textHits: 2,
                  daysDistance: 0
                }
              ],
              autoMatch: {
                documentId: "act-1",
                documentKind: "act",
                title: "РђРєС‚ ACT-001",
                amountStr: "1 000,00 РіСЂРЅ"
              }
            };
          }

          if (paymentId === "pay-2") {
            return {
              paymentId,
              isReconciled: false,
              decisionKind: "ambiguous",
              candidates: [
                {
                  documentId: "inv-1",
                  documentKind: "invoice",
                  title: "РќР°РєР»Р°РґРЅР° INV-001",
                  openAmountStr: "2 000,00 РіСЂРЅ",
                  totalScore: 0.83,
                  sameIban: true,
                  referenceHit: false,
                  textHits: 1,
                  daysDistance: 3
                },
                {
                  documentId: "act-2",
                  documentKind: "act",
                  title: "РђРєС‚ ACT-002",
                  openAmountStr: "2 000,00 РіСЂРЅ",
                  totalScore: 0.74,
                  sameIban: false,
                  referenceHit: true,
                  textHits: 2,
                  daysDistance: 5
                }
              ],
              autoMatch: null
            };
          }

          return {
            paymentId,
            isReconciled: false,
            decisionKind: "none",
            candidates: [],
            autoMatch: null
          };
        }
        case "payment_match_apply_auto":
          expect(payload).toEqual({ request: { paymentId: "pay-1" } });
          list = {
            ...list,
            items: list.items.map((item) =>
              item.id === "pay-1" ? { ...item, matchedDoc: "РђРєС‚ ACT-001" } : item
            )
          };
          return {
            ok: true,
            message: "РђРІС‚РѕР·С–СЃС‚Р°РІР»РµРЅРЅСЏ РїР»Р°С‚РµР¶Сѓ Р·Р°СЃС‚РѕСЃРѕРІР°РЅРѕ"
          };
        case "payment_reconcile":
          expect((payload as { request: { paymentId: string; documentKind: string; documentId: string; amount: string } }).request).toMatchObject({
            paymentId: "pay-2",
            documentKind: "act",
            documentId: "act-2"
          });
          expect((payload as { request: { amount: string } }).request.amount).toContain("2 000,00");
          list = {
            ...list,
            items: list.items.map((item) =>
              item.id === "pay-2" ? { ...item, matchedDoc: "Акт ACT-002" } : item
            )
          };
          return {
            ok: true,
            message: "Звірку платежу підтверджено"
          };
        default:
          throw new Error(`unexpected command: ${command}`);
      }
    });

    await paymentsStore.load();

    await paymentsStore.reconcile("pay-1");
    expect(snapshot(paymentsStore).matchPreview?.paymentId).toBe("pay-1");
    expect(snapshot(paymentsStore).matchPreview?.decisionKind).toBe("exact");
    expect(snapshot(paymentsStore).matchPreview?.autoMatch?.title).toBe("РђРєС‚ ACT-001");
    expect(invokeMock.mock.calls.filter(([command]) => command === "payment_match_apply_auto")).toHaveLength(0);

    await paymentsStore.confirmPreviewAutoMatch();
    expect(snapshot(paymentsStore).matchPreview).toBeNull();
    expect(snapshot(paymentsStore).message).toBe("РђРІС‚РѕР·С–СЃС‚Р°РІР»РµРЅРЅСЏ РїР»Р°С‚РµР¶Сѓ Р·Р°СЃС‚РѕСЃРѕРІР°РЅРѕ");

    await paymentsStore.reconcile("pay-2");
    expect(snapshot(paymentsStore).matchPreview?.decisionKind).toBe("ambiguous");
    expect(snapshot(paymentsStore).selectedCandidateId).toBe("inv-1");
    paymentsStore.selectPreviewCandidate("act-2");
    expect(snapshot(paymentsStore).selectedCandidateId).toBe("act-2");
    expect(snapshot(paymentsStore).message).toContain("Ручне підтвердження");
    const confirmResult = await paymentsStore.confirmSelectedPreviewCandidate();
    expect(snapshot(paymentsStore).matchPreview).toBeNull();
    expect(snapshot(paymentsStore).selectedCandidateId).toBeNull();
    expect(confirmResult.ok).toBe(true);
    expect(snapshot(paymentsStore).message).toBe(confirmResult.message);
    expect(snapshot(paymentsStore).list?.items.find((item) => item.id === "pay-2")?.matchedDoc).toContain("ACT-002");
    expect(invokeMock.mock.calls.filter(([command]) => command === "payment_reconcile")).toHaveLength(1);

    await paymentsStore.reconcile("pay-3");
    expect(snapshot(paymentsStore).matchPreview?.decisionKind).toBe("none");
    expect(snapshot(paymentsStore).matchPreview?.candidates).toHaveLength(0);
  });

  it("returns a clear message when ambiguous confirm has no selected candidate", async () => {
    const { paymentsStore } = await loadStores();

    invokeMock.mockImplementation(async (command, payload) => {
      switch (command) {
        case "payments_list":
          return makePayments(["pay-2"]);
        case "payment_match_preview":
          expect(payload).toEqual({ request: { paymentId: "pay-2" } });
          return {
            paymentId: "pay-2",
            isReconciled: false,
            decisionKind: "ambiguous",
            candidates: [
              {
                documentId: "inv-1",
                documentKind: "invoice",
                title: "Р СњР В°Р С”Р В»Р В°Р Т‘Р Р…Р В° INV-001",
                openAmountStr: "2 000,00 Р С–РЎР‚Р Р…",
                totalScore: 0.83,
                sameIban: true,
                referenceHit: false,
                textHits: 1,
                daysDistance: 3
              }
            ],
            autoMatch: null
          };
        default:
          throw new Error(`unexpected command: ${command}`);
      }
    });

    await paymentsStore.load();
    await paymentsStore.reconcile("pay-2");

    paymentsStore.selectPreviewCandidate("");
    const result = await paymentsStore.confirmSelectedPreviewCandidate();

    expect(result.ok).toBe(false);
    expect(result.message).toContain("Виберіть кандидата");
    expect(snapshot(paymentsStore).message).toContain("Виберіть кандидата");
    expect(snapshot(paymentsStore).matchPreview?.paymentId).toBe("pay-2");
    expect(invokeMock.mock.calls.filter(([command]) => command === "payment_reconcile")).toHaveLength(0);
  });

  it("opens manual picker for no-match preview and confirms a searched document", async () => {
    const { paymentsStore } = await loadStores();

    let list = makePayments(["pay-1", "pay-2", "pay-3"]);

    invokeMock.mockImplementation(async (command, payload) => {
      switch (command) {
        case "payments_list":
          return list;
        case "payment_match_preview":
          expect(payload).toEqual({ request: { paymentId: "pay-3" } });
          return {
            paymentId: "pay-3",
            isReconciled: false,
            decisionKind: "none",
            candidates: [],
            autoMatch: null
          };
        case "payment_match_manual_candidates": {
          const request = (payload as { request: { paymentId: string; query: string } }).request;
          expect(request.paymentId).toBe("pay-3");

          if (request.query === "ACT") {
            return {
              paymentId: "pay-3",
              query: "ACT",
              candidates: [
                {
                  documentId: "act-9",
                  documentKind: "act",
                  title: "Акт ACT-009",
                  openAmountStr: "3 000,00 грн",
                  totalScore: 40,
                  sameIban: false,
                  referenceHit: false,
                  textHits: 1,
                  daysDistance: 4
                }
              ]
            };
          }

          return {
            paymentId: "pay-3",
            query: request.query,
            candidates: [
              {
                documentId: "inv-7",
                documentKind: "invoice",
                title: "Накладна INV-007",
                openAmountStr: "1 500,00 грн",
                totalScore: 0,
                sameIban: false,
                referenceHit: false,
                textHits: 0,
                daysDistance: 365
              },
              {
                documentId: "act-9",
                documentKind: "act",
                title: "Акт ACT-009",
                openAmountStr: "3 000,00 грн",
                totalScore: 40,
                sameIban: false,
                referenceHit: false,
                textHits: 1,
                daysDistance: 4
              }
            ]
          };
        }
        case "payment_reconcile":
          expect((payload as { request: { paymentId: string; documentId: string; documentKind: string } }).request).toMatchObject({
            paymentId: "pay-3",
            documentId: "act-9",
            documentKind: "act"
          });
          list = {
            ...list,
            items: list.items.map((item) =>
              item.id === "pay-3" ? { ...item, matchedDoc: "ACT-009" } : item
            )
          };
          return {
            ok: true,
            message: "Ручну звірку підтверджено"
          };
        default:
          throw new Error(`unexpected command: ${command}`);
      }
    });

    await paymentsStore.load();
    await paymentsStore.reconcile("pay-3");
    expect(snapshot(paymentsStore).matchPreview?.decisionKind).toBe("none");

    await paymentsStore.openManualMatchPicker("pay-3");
    expect(snapshot(paymentsStore).manualPicker?.paymentId).toBe("pay-3");
    expect(snapshot(paymentsStore).manualPicker?.candidates).toHaveLength(2);

    paymentsStore.updateManualMatchQuery("ACT");
    await paymentsStore.searchManualMatchCandidates();
    expect(snapshot(paymentsStore).manualPicker?.query).toBe("ACT");
    expect(snapshot(paymentsStore).manualPicker?.candidates).toHaveLength(1);
    expect(snapshot(paymentsStore).manualPicker?.selectedCandidateId).toBe("act-9");

    const result = await paymentsStore.confirmManualPickerCandidate();
    expect(result.ok).toBe(true);
    expect(snapshot(paymentsStore).manualPicker).toBeNull();
    expect(snapshot(paymentsStore).matchPreview).toBeNull();
    expect(snapshot(paymentsStore).message).toBe("Ручну звірку підтверджено");
    expect(snapshot(paymentsStore).list?.items.find((item) => item.id === "pay-3")?.matchedDoc).toContain("ACT-009");
    expect(invokeMock.mock.calls.filter(([command]) => command === "payment_match_manual_candidates")).toHaveLength(2);
  });

  it("builds a split draft from manual picker candidates", async () => {
    const { paymentsStore } = await loadStores();

    const list = makePayments(["pay-1", "pay-2", "pay-3"]);

    invokeMock.mockImplementation(async (command, payload) => {
      switch (command) {
        case "payments_list":
          return list;
        case "payment_match_preview":
          expect(payload).toEqual({ request: { paymentId: "pay-3" } });
          return {
            paymentId: "pay-3",
            isReconciled: false,
            decisionKind: "none",
            candidates: [],
            autoMatch: null
          };
        case "payment_match_manual_candidates":
          return {
            paymentId: "pay-3",
            query: "",
            candidates: [
              {
                documentId: "inv-7",
                documentKind: "invoice",
                title: "Накладна INV-007",
                openAmountStr: "1 500,00 грн",
                totalScore: 35,
                sameIban: false,
                referenceHit: false,
                textHits: 0,
                daysDistance: 6
              },
              {
                documentId: "act-9",
                documentKind: "act",
                title: "Акт ACT-009",
                openAmountStr: "3 000,00 грн",
                totalScore: 40,
                sameIban: false,
                referenceHit: false,
                textHits: 1,
                daysDistance: 4
              }
            ]
          };
        default:
          throw new Error(`unexpected command: ${command}`);
      }
    });

    await paymentsStore.load();
    await paymentsStore.reconcile("pay-3");
    await paymentsStore.openManualMatchPicker("pay-3");

    await paymentsStore.addSelectedManualPickerCandidateToSplit();
    expect(snapshot(paymentsStore).splitDraft?.allocations).toHaveLength(1);
    expect(normalizeMoneyText(snapshot(paymentsStore).splitDraft?.remainingAmountStr)).toContain("1 500,00");

    paymentsStore.selectManualPickerCandidate("act-9");
    await paymentsStore.addSelectedManualPickerCandidateToSplit();

    expect(snapshot(paymentsStore).splitDraft?.allocations).toHaveLength(2);
    expect(normalizeMoneyText(snapshot(paymentsStore).splitDraft?.remainingAmountStr)).toContain("0,00");

    paymentsStore.updateSplitAllocationAmount("act-9", "1 500,00");
    paymentsStore.updateSplitAllocationAmount("inv-7", "1 500,00");

    expect(snapshot(paymentsStore).splitDraft?.allocations).toHaveLength(2);
    expect(snapshot(paymentsStore).splitDraft?.remainingAmountStr).toContain("0,00");
  });

  it("persists a split draft through one batch reconcile call", async () => {
    const { paymentsStore } = await loadStores();

    let list = makePayments(["pay-1", "pay-2", "pay-3"]);
    const splitCalls: Array<{
      paymentId: string;
      allocations: Array<{
        documentId: string;
        documentKind: string;
        amount: string;
      }>;
    }> = [];

    invokeMock.mockImplementation(async (command, payload) => {
      switch (command) {
        case "payments_list":
          return list;
        case "payment_match_preview":
          expect(payload).toEqual({ request: { paymentId: "pay-3" } });
          return {
            paymentId: "pay-3",
            isReconciled: false,
            decisionKind: "none",
            candidates: [],
            autoMatch: null
          };
        case "payment_match_manual_candidates":
          return {
            paymentId: "pay-3",
            query: "",
            candidates: [
              {
                documentId: "inv-7",
                documentKind: "invoice",
                title: "Накладна INV-007",
                openAmountStr: "1 500,00 грн",
                totalScore: 35,
                sameIban: false,
                referenceHit: false,
                textHits: 0,
                daysDistance: 6
              },
              {
                documentId: "act-9",
                documentKind: "act",
                title: "Акт ACT-009",
                openAmountStr: "3 000,00 грн",
                totalScore: 40,
                sameIban: false,
                referenceHit: false,
                textHits: 1,
                daysDistance: 4
              }
            ]
          };
        case "payment_reconcile_split": {
          const request = {
            ...(payload as {
              request: {
                paymentId: string;
                allocations: Array<{
                  documentId: string;
                  documentKind: string;
                  amount: string;
                }>;
              };
            }).request
          };
          splitCalls.push(request);

          expect(request.paymentId).toBe("pay-3");
          expect(request.allocations).toHaveLength(2);
          expect(request.allocations.map((allocation) => allocation.documentId)).toEqual([
            "inv-7",
            "act-9"
          ]);
          expect(request.allocations.map((allocation) => allocation.documentKind)).toEqual([
            "invoice",
            "act"
          ]);
          expect(
            request.allocations.every((allocation) =>
              normalizeMoneyText(allocation.amount).includes("1 500,00")
            )
          ).toBe(true);

          list = {
            ...list,
            items: list.items.map((item) =>
              item.id === "pay-3"
                ? {
                    ...item,
                    matchedDoc: "INV-007 + ACT-009"
                  }
                : item
            )
          };

          return {
            ok: true,
            message: "Розподіл платежу підтверджено",
            paymentId: "pay-3",
            allocationCount: 2,
            totalAllocatedStr: "3 000,00 грн",
            allocations: request.allocations.map((allocation) => ({
              ...allocation,
              amount: allocation.amount
            }))
          };
        }
        default:
          throw new Error(`unexpected command: ${command}`);
      }
    });

    await paymentsStore.load();
    await paymentsStore.reconcile("pay-3");
    await paymentsStore.openManualMatchPicker("pay-3");

    await paymentsStore.addSelectedManualPickerCandidateToSplit();
    paymentsStore.selectManualPickerCandidate("act-9");
    await paymentsStore.addSelectedManualPickerCandidateToSplit();

    const result = await paymentsStore.confirmSplitDraft();

    expect(result.ok).toBe(true);
    expect(result.message).toBe("Розподіл платежу підтверджено");
    expect(snapshot(paymentsStore).splitDraft).toBeNull();
    expect(snapshot(paymentsStore).manualPicker).toBeNull();
    expect(snapshot(paymentsStore).matchPreview).toBeNull();
    expect(snapshot(paymentsStore).message).toBe("Розподіл платежу підтверджено");
    expect(snapshot(paymentsStore).list?.items.find((item) => item.id === "pay-3")?.matchedDoc).toContain("ACT-009");
    expect(splitCalls).toHaveLength(1);
    expect(invokeMock.mock.calls.filter(([command]) => command === "payment_reconcile_split")).toHaveLength(1);
    expect(invokeMock.mock.calls.filter(([command]) => command === "payments_list")).toHaveLength(2);
  });

  it("keeps ambiguous preview state when manual reconcile returns an error result", async () => {
    const { paymentsStore } = await loadStores();

    invokeMock.mockImplementation(async (command, payload) => {
      switch (command) {
        case "payments_list":
          return makePayments(["pay-2"]);
        case "payment_match_preview":
          expect(payload).toEqual({ request: { paymentId: "pay-2" } });
          return {
            paymentId: "pay-2",
            isReconciled: false,
            decisionKind: "ambiguous",
            candidates: [
              {
                documentId: "inv-1",
                documentKind: "invoice",
                title: "Накладна INV-001",
                openAmountStr: "2 000,00 грн",
                totalScore: 0.83,
                sameIban: true,
                referenceHit: false,
                textHits: 1,
                daysDistance: 3
              },
              {
                documentId: "act-2",
                documentKind: "act",
                title: "Акт ACT-002",
                openAmountStr: "2 000,00 грн",
                totalScore: 0.74,
                sameIban: false,
                referenceHit: true,
                textHits: 2,
                daysDistance: 1
              }
            ],
            autoMatch: null
          };
        case "payment_reconcile":
          expect((payload as { request: { paymentId: string; documentKind: string; documentId: string } }).request).toMatchObject({
            paymentId: "pay-2",
            documentKind: "act",
            documentId: "act-2"
          });
          return {
            ok: false,
            message: "Не вдалося підтвердити звірку"
          };
        default:
          throw new Error(`unexpected command: ${command}`);
      }
    });

    await paymentsStore.load();
    await paymentsStore.reconcile("pay-2");
    paymentsStore.selectPreviewCandidate("act-2");

    const result = await paymentsStore.confirmSelectedPreviewCandidate();

    expect(result.ok).toBe(false);
    expect(result.message).toBe("Не вдалося підтвердити звірку");
    expect(snapshot(paymentsStore).message).toBe("Не вдалося підтвердити звірку");
    expect(snapshot(paymentsStore).matchPreview?.paymentId).toBe("pay-2");
    expect(snapshot(paymentsStore).matchPreview?.decisionKind).toBe("ambiguous");
    expect(snapshot(paymentsStore).selectedCandidateId).toBe("act-2");
    expect(snapshot(paymentsStore).list?.items.find((item) => item.id === "pay-2")?.matchedDoc).toBe("");
    expect(invokeMock.mock.calls.filter(([command]) => command === "payment_reconcile")).toHaveLength(1);
    expect(invokeMock.mock.calls.filter(([command]) => command === "payments_list")).toHaveLength(1);
  });

  it("keeps ambiguous preview state when manual reconcile throws", async () => {
    const { paymentsStore } = await loadStores();

    invokeMock.mockImplementation(async (command, payload) => {
      switch (command) {
        case "payments_list":
          return makePayments(["pay-2"]);
        case "payment_match_preview":
          expect(payload).toEqual({ request: { paymentId: "pay-2" } });
          return {
            paymentId: "pay-2",
            isReconciled: false,
            decisionKind: "ambiguous",
            candidates: [
              {
                documentId: "inv-1",
                documentKind: "invoice",
                title: "Накладна INV-001",
                openAmountStr: "2 000,00 грн",
                totalScore: 0.83,
                sameIban: true,
                referenceHit: false,
                textHits: 1,
                daysDistance: 3
              },
              {
                documentId: "act-2",
                documentKind: "act",
                title: "Акт ACT-002",
                openAmountStr: "2 000,00 грн",
                totalScore: 0.74,
                sameIban: false,
                referenceHit: true,
                textHits: 2,
                daysDistance: 1
              }
            ],
            autoMatch: null
          };
        case "payment_reconcile":
          expect((payload as { request: { paymentId: string; documentKind: string; documentId: string } }).request).toMatchObject({
            paymentId: "pay-2",
            documentKind: "act",
            documentId: "act-2"
          });
          throw new Error("Тестова помилка звірки");
        default:
          throw new Error(`unexpected command: ${command}`);
      }
    });

    await paymentsStore.load();
    await paymentsStore.reconcile("pay-2");
    paymentsStore.selectPreviewCandidate("act-2");

    const result = await paymentsStore.confirmSelectedPreviewCandidate();

    expect(result.ok).toBe(false);
    expect(result.message).toContain("Тестова помилка звірки");
    expect(snapshot(paymentsStore).message).toContain("Тестова помилка звірки");
    expect(snapshot(paymentsStore).matchPreview?.paymentId).toBe("pay-2");
    expect(snapshot(paymentsStore).matchPreview?.decisionKind).toBe("ambiguous");
    expect(snapshot(paymentsStore).selectedCandidateId).toBe("act-2");
    expect(snapshot(paymentsStore).list?.items.find((item) => item.id === "pay-2")?.matchedDoc).toBe("");
    expect(invokeMock.mock.calls.filter(([command]) => command === "payment_reconcile")).toHaveLength(1);
    expect(invokeMock.mock.calls.filter(([command]) => command === "payments_list")).toHaveLength(1);
  });

  it("clears stale preview when the next reconcile preview request fails", async () => {
    const { paymentsStore } = await loadStores();

    invokeMock.mockImplementation(async (command, payload) => {
      switch (command) {
        case "payments_list":
          return makePayments(["pay-1", "pay-2"]);
        case "payment_match_preview": {
          const paymentId = (payload as { request: { paymentId: string } }).request.paymentId;

          if (paymentId === "pay-1") {
            return {
              paymentId,
              isReconciled: false,
              decisionKind: "exact",
              candidates: [
                {
                  documentId: "act-1",
                  documentKind: "act",
                  title: "РђРєС‚ ACT-001",
                  openAmountStr: "1 000,00 РіСЂРЅ",
                  totalScore: 0.99,
                  sameIban: true,
                  referenceHit: true,
                  textHits: 2,
                  daysDistance: 0
                }
              ],
              autoMatch: {
                documentId: "act-1",
                documentKind: "act",
                title: "РђРєС‚ ACT-001",
                amountStr: "1 000,00 РіСЂРЅ"
              }
            };
          }

          throw new Error("preview failed");
        }
        default:
          throw new Error(`unexpected command: ${command}`);
      }
    });

    await paymentsStore.load();

    await paymentsStore.reconcile("pay-1");
    expect(snapshot(paymentsStore).matchPreview?.paymentId).toBe("pay-1");

    await paymentsStore.reconcile("pay-2");
    expect(snapshot(paymentsStore).matchPreview).toBeNull();
    expect(snapshot(paymentsStore).selectedCandidateId).toBeNull();
    expect(snapshot(paymentsStore).message).toContain("preview failed");
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
              darkMode: true
            }
          };
          return {
            ok: true,
            message: "РќР°Р»Р°С€С‚СѓРІР°РЅРЅСЏ РІРёРіР»СЏРґСѓ Р·Р±РµСЂРµР¶РµРЅРѕ",
            screen
          };
        case "settings_save_company":
          screen = {
            ...screen,
            company: {
              ...screen.company,
              fullName: "РўРћР’ РђРєС‚ РџР»СЋСЃ"
            }
          };
          return {
            ok: true,
            message: "РљРѕРјРїР°РЅС–СЋ Р·Р±РµСЂРµР¶РµРЅРѕ",
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
            message: "Р†РЅС‚РµРіСЂР°С†С–СЋ РѕРЅРѕРІР»РµРЅРѕ",
            screen
          };
        case "settings_team_invite":
          screen = {
            ...screen,
            team: [
              {
                name: "РћР»РµРЅР°",
                email: "olena@example.com",
                role: "admin",
                lastActive: "С‰РѕР№РЅРѕ"
              }
            ]
          };
          return {
            ok: true,
            message: "Р—Р°РїСЂРѕС€РµРЅРЅСЏ СЃС‚РІРѕСЂРµРЅРѕ",
            screen
          };
        case "settings_backup_now":
          return {
            ok: true,
            message: "Р РµР·РµСЂРІРЅСѓ РєРѕРїС–СЋ СЃС‚РІРѕСЂРµРЅРѕ",
            screen
          };
        case "settings_backup_open_latest":
          return {
            ok: true,
            message: "Р РµР·РµСЂРІРЅСѓ РєРѕРїС–СЋ РІС–РґРєСЂРёС‚Рѕ",
            path: "storage/backups/latest.zip"
          };
        default:
          throw new Error(`unexpected command: ${command}`);
      }
    });

    await settingsStore.load();
    expect(snapshot(settingsStore).screen?.company.fullName).toBe("РўРћР’ РђРєС‚");

    settingsStore.updatePreference("darkMode", true);
    await settingsStore.savePreferences();
    expect(snapshot(settingsStore).screen?.preferences).toEqual({
      darkMode: true
    });

    settingsStore.updateCompanyField("fullName", "РўРћР’ РђРєС‚ РџР»СЋСЃ");
    await settingsStore.saveCompany();
    expect(snapshot(settingsStore).screen?.company.fullName).toBe("РўРћР’ РђРєС‚ РџР»СЋСЃ");

    await settingsStore.configureIntegration("bas");
    expect(snapshot(settingsStore).screen?.integrations[0]?.enabled).toBe(true);

    await settingsStore.inviteTeam();
    expect(snapshot(settingsStore).screen?.team).toHaveLength(1);

    await settingsStore.backupNow();
    expect(snapshot(settingsStore).message).toBe("Р РµР·РµСЂРІРЅСѓ РєРѕРїС–СЋ СЃС‚РІРѕСЂРµРЅРѕ");

    await settingsStore.openLatestBackup();
    expect(snapshot(settingsStore).message).toContain("storage/backups/latest.zip");
  });

  it("keeps the sidebar theme toggle on the persisted settings flow", () => {
    expect(appSource).toContain("async function onQuickThemeToggle()");
    expect(appSource).toContain("settings.savePreferences()");
    expect(appSource).toContain("await shell.load();");
    expect(appSource).toContain("theme.setMode(saved.screen.preferences.darkMode ? \"dark\" : \"light\")");
    expect(appSource).toContain("on:click={onQuickThemeToggle}");
    expect(appSource).not.toContain("on:click={() => theme.toggle()}");
  });

  it("keeps unused document commands out of the public Tauri invoke surface", () => {
    for (const command of [
      "document_prepare_new"
    ]) {
      expect(frontendApiSource).not.toContain(command);
      expect(tauriLibSource).not.toContain(command);
      expect(tauriDocumentsSource).not.toContain(`fn ${command}`);
    }
  });
});

