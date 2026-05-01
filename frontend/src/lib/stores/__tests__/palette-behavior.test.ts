import { beforeEach, describe, expect, it, vi } from "vitest";
import { loadStores, invokeMock, snapshot } from "./helpers";

type PaletteSearchResult = {
  items: Array<{
    kind: string;
    title: string;
    subtitle: string;
    shortcut: string;
    payload: string;
  }>;
};

describe("palette store behavior", () => {
  beforeEach(() => {
    vi.resetModules();
    invokeMock.mockReset();
  });

  it("primes predictable default results when the palette opens", async () => {
    const { paletteStore } = await loadStores();

    invokeMock.mockImplementation(async (command) => {
      if (command === "shell_palette_search") {
        return {
          items: [
            {
              kind: "navigate",
              title: "Звіти",
              subtitle: "Гроші та дебіторка",
              shortcut: "Ctrl+5",
              payload: "screen:reports"
            }
          ]
        };
      }

      if (command === "shell_palette_activate") {
        return {
          kind: "navigate",
          screen: "reports",
          documentId: null,
          counterpartyId: null,
          documentEditor: null,
          message: null
        };
      }

      throw new Error(`unexpected command: ${command}`);
    });

    paletteStore.open();
    await vi.waitFor(() => {
      expect(snapshot(paletteStore).items).toHaveLength(1);
    });

    expect(snapshot(paletteStore).open).toBe(true);
    expect(snapshot(paletteStore).query).toBe("");
    expect(invokeMock).toHaveBeenCalledWith("shell_palette_search", {
      request: {
        query: "",
        selectedCounterpartyId: undefined
      }
    });
  });

  it("ignores stale search results after the palette closes", async () => {
    const { paletteStore } = await loadStores();

    let resolveSearch: (value: PaletteSearchResult) => void = () => {};

    invokeMock.mockImplementation(
      (command) =>
        new Promise<PaletteSearchResult>((resolve, reject) => {
          if (command === "shell_palette_search") {
            resolveSearch = resolve;
            return;
          }

          reject(new Error(`unexpected command: ${command}`));
        })
    );

    paletteStore.open();
    paletteStore.close();
    resolveSearch({
      items: [
        {
          kind: "navigate",
          title: "Документи",
          subtitle: "Чернетки і ланцюжки",
          shortcut: "Ctrl+2",
          payload: "screen:documents"
        }
      ]
    });

    await vi.waitFor(() => {
      expect(snapshot(paletteStore).loading).toBe(false);
    });

    expect(snapshot(paletteStore)).toEqual({
      open: false,
      query: "",
      items: [],
      loading: false,
      error: null
    });
  });
  it("reopens from a clean query and replaces previous results with fresh defaults", async () => {
    const { paletteStore } = await loadStores();

    invokeMock.mockImplementation(async (command, payload) => {
      if (command !== "shell_palette_search") {
        throw new Error(`unexpected command: ${command}`);
      }

      const query = (payload as { request: { query: string } }).request.query;
      if (query === "") {
        return {
          items: [
            {
              kind: "navigate",
              title: "Дашборд",
              subtitle: "Повернутися на головний екран",
              shortcut: "Ctrl+1",
              payload: "screen:dashboard"
            }
          ]
        };
      }

      return {
        items: [
          {
            kind: "navigate",
            title: "Документи",
            subtitle: "Відкрити список документів",
            shortcut: "Ctrl+2",
            payload: "screen:documents"
          }
        ]
      };
    });

    paletteStore.open();
    await vi.waitFor(() => {
      expect(snapshot(paletteStore).items[0]?.title).toBe("Дашборд");
    });

    await paletteStore.search("док");
    expect(snapshot(paletteStore)).toMatchObject({
      open: true,
      query: "док"
    });
    expect(snapshot(paletteStore).items[0]?.title).toBe("Документи");

    paletteStore.close();
    paletteStore.open();

    await vi.waitFor(() => {
      expect(snapshot(paletteStore).items[0]?.title).toBe("Дашборд");
    });

    expect(snapshot(paletteStore)).toMatchObject({
      open: true,
      query: "",
      loading: false,
      error: null
    });
  });

  it("keeps the latest open cycle deterministic across open-close-open", async () => {
    const { paletteStore } = await loadStores();

    const pendingSearches: Array<(value: PaletteSearchResult) => void> = [];

    invokeMock.mockImplementation(
      (command) =>
        new Promise<PaletteSearchResult>((resolve, reject) => {
          if (command === "shell_palette_search") {
            pendingSearches.push(resolve);
            return;
          }

          reject(new Error(`unexpected command: ${command}`));
        })
    );

    paletteStore.open();
    paletteStore.close();
    paletteStore.open();

    await vi.waitFor(() => {
      expect(pendingSearches).toHaveLength(2);
    });

    expect(invokeMock).toHaveBeenCalledTimes(2);

    pendingSearches[0]({
      items: [
        {
          kind: "navigate",
          title: "Документи",
          subtitle: "Застарілий результат",
          shortcut: "Ctrl+2",
          payload: "screen:documents"
        }
      ]
    });

    await vi.waitFor(() => {
      expect(snapshot(paletteStore).open).toBe(true);
    });

    expect(snapshot(paletteStore)).toMatchObject({
      open: true,
      query: "",
      items: [],
      error: null
    });

    pendingSearches[1]({
      items: [
        {
          kind: "navigate",
          title: "Звіти",
          subtitle: "Актуальний результат",
          shortcut: "Ctrl+5",
          payload: "screen:reports"
        }
      ]
    });

    await vi.waitFor(() => {
      expect(snapshot(paletteStore).loading).toBe(false);
    });

    expect(snapshot(paletteStore).items).toEqual([
      {
        kind: "navigate",
        title: "Звіти",
        subtitle: "Актуальний результат",
        shortcut: "Ctrl+5",
        payload: "screen:reports"
      }
    ]);
  });
});
