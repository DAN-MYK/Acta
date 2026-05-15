import { describe, expect, it } from "vitest";
import { invokeMock, loadStores, snapshot } from "./helpers";

describe("app shell sign-out flow", () => {
  it("clears shell and volatile screen state and returns to the dashboard", async () => {
    const {
      appShellStore,
      shellStore,
      navigationStore,
      paletteStore,
      documentsStore,
      importStore
    } = await loadStores();

    invokeMock.mockReset();
    invokeMock.mockImplementation(async (command) => {
      if (command === "shell_load") {
        return {
          chrome: {
            companyName: "ТОВ Акт",
            userName: "Олена",
            userInitials: "ОО",
            userRole: "Адміністратор",
            documentsBadge: 4,
            tasksBadge: 2
          },
          companyItems: [
            {
              id: "company-1",
              name: "ТОВ Акт",
              subtitle: "Основна компанія",
              initials: "ТА",
              badge: "",
              active: true
            }
          ],
          activeCompanyId: "company-1",
          isDark: false
        };
      }

      if (command === "shell_palette_search") {
        return {
          items: [
            {
              kind: "navigate",
              title: "Документи",
              subtitle: "Відкрити документи",
              shortcut: "Ctrl+2",
              payload: "screen:documents"
            }
          ]
        };
      }

      throw new Error(`Unexpected command: ${command}`);
    });

    await shellStore.load();
    navigationStore.go("documents");
    paletteStore.open();
    documentsStore.setDraftContext("cp-1", "ТОВ Ромашка");
    await importStore.chooseDirectory().catch(() => undefined);

    expect(snapshot(shellStore).state?.activeCompanyId).toBe("company-1");
    expect(snapshot(navigationStore)).toBe("documents");
    expect(snapshot(paletteStore).open).toBe(true);
    expect(snapshot(documentsStore).draftContext).toEqual({
      counterpartyId: "cp-1",
      counterpartyName: "ТОВ Ромашка"
    });

    (appShellStore as unknown as { signOut: () => void }).signOut();

    expect(snapshot(shellStore).state).toBeNull();
    expect(snapshot(navigationStore)).toBe("dashboard");
    expect(snapshot(paletteStore).open).toBe(false);
    expect(snapshot(documentsStore).draftContext).toBeNull();
    expect(snapshot(importStore).selectedDirectory).toBeNull();
  });
});
