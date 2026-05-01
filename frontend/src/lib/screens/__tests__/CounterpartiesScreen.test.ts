/**
 * @vitest-environment jsdom
 */
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { tick } from "svelte";
import CounterpartiesScreen from "../CounterpartiesScreen.svelte";
import type {
  CounterpartyDetailScreenDto,
  CounterpartyEditorDto,
  CounterpartiesScreenDto
} from "../../types";

const mocks = vi.hoisted(() => {
  function createMockStore<T>(initialValue: T) {
    let value = initialValue;
    const subscribers = new Set<(value: T) => void>();

    return {
      subscribe(run: (value: T) => void) {
        run(value);
        subscribers.add(run);
        return () => subscribers.delete(run);
      },
      set(nextValue: T) {
        value = nextValue;
        for (const run of subscribers) {
          run(value);
        }
      }
    };
  }

  const counterpartiesState = createMockStore({
    screen: null as CounterpartiesScreenDto | null,
    detail: null as CounterpartyDetailScreenDto | null,
    editor: null as CounterpartyEditorDto | null,
    selectedId: null as string | null,
    initialLoading: false,
    loading: false,
    error: null as string | null,
    message: null as string | null,
    query: ""
  });

  return {
    archiveCurrent: vi.fn(),
    closeEditor: vi.fn(),
    counterpartiesState,
    createDocument: vi.fn(),
    load: vi.fn(),
    navigationGo: vi.fn(),
    open: vi.fn(),
    openEditor: vi.fn(),
    updateFormField: vi.fn(),
    save: vi.fn(),
    documentsOpen: vi.fn()
  };
});

vi.mock("../../stores/counterparties", () => ({
  counterpartiesStore: {
    subscribe: mocks.counterpartiesState.subscribe,
    archiveCurrent: mocks.archiveCurrent,
    closeEditor: mocks.closeEditor,
    createDocument: mocks.createDocument,
    load: mocks.load,
    open: mocks.open,
    openEditor: mocks.openEditor,
    updateFormField: mocks.updateFormField,
    save: mocks.save
  }
}));

vi.mock("../../stores/documents", () => ({
  documentsStore: {
    open: mocks.documentsOpen
  }
}));

vi.mock("../../stores/navigation", () => ({
  navigationStore: {
    go: mocks.navigationGo
  }
}));

function makeScreen(): CounterpartiesScreenDto {
  return {
    items: [
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
    ]
  };
}

function makeDetail(): CounterpartyDetailScreenDto {
  return {
    info: {
      id: "cp-2",
      name: "ФОП Петренко",
      kind: "Постачальник",
      edrpou: "87654321",
      ipn: "3012345678",
      vat: "Без ПДВ",
      iban: "UA123456789012345678901234567",
      bank: "mono",
      address: "м. Київ, вул. Січових Стрільців, 10",
      director: "Петренко П.П.",
      phone: "+380501112233",
      email: "petrenko@example.com",
      clientSince: "2024-02-01",
      balanceStr: "-19 000,00 грн",
      balanceIsNegative: true,
      docCount: 4,
      overdueCount: 1,
      overdueAmountStr: "19 000,00 грн",
      lastContactDays: 6,
      lastContactDate: "2026-04-25"
    },
    documents: [
      {
        id: "doc-1",
        kind: "invoice",
        number: "INV-2026-0042",
        date: "2026-05-01",
        counterparty: "ФОП Петренко",
        amountStr: "19 000,00 грн",
        status: "issued",
        statusLabel: "Виставлено",
        linkedId: ""
      }
    ],
    payments: [
      {
        id: "pay-1",
        date: "2026-04-30",
        counterpartyId: "cp-2",
        counterparty: "ФОП Петренко",
        amountStr: "19 000,00 грн",
        direction: "out",
        matchedDoc: "INV-2026-0042",
        account: "mono"
      }
    ]
  };
}

function setCounterpartiesState(
  detail: CounterpartyDetailScreenDto | null = makeDetail(),
  screen: CounterpartiesScreenDto = makeScreen()
) {
  mocks.counterpartiesState.set({
    screen,
    detail,
    editor: null,
    selectedId: detail?.info.id ?? null,
    initialLoading: false,
    loading: false,
    error: null,
    message: null,
    query: ""
  });
}

function renderCounterparties() {
  const target = document.createElement("div");
  document.body.appendChild(target);
  const component = new CounterpartiesScreen({ target });

  return { component, target };
}

function buttonByText(target: HTMLElement, text: string): HTMLButtonElement {
  const button = Array.from(target.querySelectorAll("button")).find((candidate) =>
    candidate.textContent?.includes(text)
  );

  expect(button).toBeTruthy();
  return button as HTMLButtonElement;
}

describe("CounterpartiesScreen component", () => {
  beforeEach(() => {
    setCounterpartiesState();
    for (const fn of [
      mocks.archiveCurrent,
      mocks.closeEditor,
      mocks.createDocument,
      mocks.load,
      mocks.navigationGo,
      mocks.open,
      mocks.openEditor,
      mocks.updateFormField,
      mocks.save,
      mocks.documentsOpen
    ]) {
      fn.mockReset();
    }
  });

  afterEach(() => {
    document.body.innerHTML = "";
  });

  it("renders the counterparty as an operational risk card with scenario sections and CTA hierarchy", () => {
    const { component, target } = renderCounterparties();

    expect(target.textContent).toContain("Контрагенти");
    expect(target.textContent).toContain("ФОП Петренко");
    expect(target.textContent).toContain("Потребує уваги: прострочено 1 документ");
    expect(target.textContent).toContain("Хто це");
    expect(target.textContent).toContain("Фінансовий стан");
    expect(target.textContent).toContain("Документи");
    expect(target.textContent).toContain("Платежі");
    expect(target.textContent).toContain("Наступна дія");
    expect(target.textContent).toContain("Останній контакт 2026-04-25");
    expect(target.textContent).toContain("Директор");
    expect(target.textContent).toContain("Банк");
    expect(target.textContent).toContain("VAT");
    expect(target.textContent).toContain("Петренко П.П.");
    expect(target.textContent).toContain("19 000,00 грн");
    expect(target.textContent).toContain("INV-2026-0042");
    expect(target.textContent).toContain("mono");

    expect(buttonByText(target, "Створити документ").className).toContain("btn-primary");
    expect(buttonByText(target, "Редагувати").className).toContain("btn-secondary");
    expect(buttonByText(target, "Архівувати").className).toContain("btn-danger");

    component.$destroy();
  });

  it("wires key CTA actions and opens detail from the list row", async () => {
    const { component, target } = renderCounterparties();

    buttonByText(target, "ТОВ Ромашка").click();
    buttonByText(target, "Редагувати").click();
    buttonByText(target, "Створити документ").click();
    buttonByText(target, "Архівувати").click();
    await tick();

    expect(mocks.open).toHaveBeenCalledWith("cp-1");
    expect(mocks.openEditor).toHaveBeenCalledWith("cp-2");
    expect(mocks.createDocument).toHaveBeenCalled();
    expect(mocks.archiveCurrent).toHaveBeenCalled();

    component.$destroy();
  });

  it("opens related documents through the owning stores", async () => {
    const { component, target } = renderCounterparties();

    buttonByText(target, "INV-2026-0042").click();
    await tick();

    expect(mocks.navigationGo).toHaveBeenCalledWith("documents");
    expect(mocks.documentsOpen).toHaveBeenCalledWith("doc-1");

    component.$destroy();
  });

  it("shows an explicit empty state with a useful next step", async () => {
    setCounterpartiesState(null);
    const { component, target } = renderCounterparties();

    expect(target.textContent).toContain("Оберіть контрагента");
    expect(target.textContent).toContain("Оберіть зліва вже відомого контрагента");
    expect(target.textContent).toContain("або створіть нового, щоб одразу побачити");
    expect(target.textContent).toContain("баланс");
    expect(target.textContent).toContain("прострочки");
    expect(target.textContent).toContain("наступного кроку");
    expect(buttonByText(target, "Новий контрагент").className).toContain("btn-primary");
    buttonByText(target, "Новий контрагент").click();
    await tick();

    expect(mocks.openEditor).toHaveBeenCalledWith();

    component.$destroy();
  });

  it("exposes stable smoke markers for list and detail states", () => {
    const { component, target } = renderCounterparties();

    expect(target.querySelector('[data-testid="counterparties-screen"]')).toBeTruthy();
    expect(target.querySelector('[data-testid="counterparties-list"]')).toBeTruthy();
    expect(target.querySelector('[data-testid="counterparty-detail"]')).toBeTruthy();
    expect(target.querySelector('[data-testid="counterparty-overview"]')).toBeTruthy();
    expect(target.querySelector('[data-testid="counterparty-scenario"]')).toBeTruthy();

    component.$destroy();
  });

  it("shows list skeletons during initial loading while chrome stays visible", () => {
    mocks.counterpartiesState.set({
      screen: null,
      detail: null,
      editor: null,
      selectedId: null,
      initialLoading: true,
      loading: false,
      error: null,
      message: null,
      query: ""
    });

    const { component, target } = renderCounterparties();

    expect(target.textContent).toContain("Контрагенти");
    expect(target.textContent).toContain("Новий контрагент");
    expect(target.querySelectorAll('[data-testid="skeleton-row-item"]')).toHaveLength(6);
    expect(target.querySelector('[data-testid="counterparties-list"]')).toBeTruthy();
    expect(target.querySelector('[data-testid="counterparties-empty-state"]')).toBeNull();

    component.$destroy();
  });
});
