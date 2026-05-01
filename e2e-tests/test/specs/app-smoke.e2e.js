describe("Acta desktop shell smoke", () => {
  it("initializes the real WebView and navigates core screens", async () => {
    const body = await $("body");
    await body.waitForExist({ timeout: 60000 });

    await $('[data-testid="dashboard-screen"]').waitForExist({ timeout: 60000 });

    await $('[data-testid="nav-documents"]').click();
    await $('[data-testid="documents-screen"]').waitForExist({ timeout: 30000 });

    await $('[data-testid="nav-payments"]').click();
    await $('[data-testid="payments-screen"]').waitForExist({ timeout: 30000 });

    await browser.keys(["Control", "1"]);
    await $('[data-testid="dashboard-screen"]').waitForExist({ timeout: 30000 });
  });

  it("keeps shell and dashboard cashflow readable on a narrow viewport", async () => {
    const body = await $("body");
    await body.waitForExist({ timeout: 60000 });
    await $('[data-testid="dashboard-screen"]').waitForExist({ timeout: 30000 });

    await browser.setWindowSize(820, 900);

    const responsiveState = await browser.execute(() => {
      const appShell = document.querySelector(".app-shell");
      const topbar = document.querySelector(".topbar");
      const topbarActions = document.querySelector(".topbar-actions");
      const cashflowRow = document.querySelector(".cashflow-row");

      if (!(appShell instanceof HTMLElement)) {
        throw new Error("app-shell не знайдено");
      }
      if (!(topbar instanceof HTMLElement)) {
        throw new Error("topbar не знайдено");
      }
      if (!(topbarActions instanceof HTMLElement)) {
        throw new Error("topbar-actions не знайдено");
      }
      if (!(cashflowRow instanceof HTMLElement)) {
        throw new Error("cashflow-row не знайдено");
      }

      return {
        appShellColumns: window.getComputedStyle(appShell).gridTemplateColumns,
        topbarDirection: window.getComputedStyle(topbar).flexDirection,
        topbarActionsDirection: window.getComputedStyle(topbarActions).flexDirection,
        topbarActionsWrap: window.getComputedStyle(topbarActions).flexWrap,
        cashflowColumns: window.getComputedStyle(cashflowRow).gridTemplateColumns,
        hasHorizontalOverflow: document.documentElement.scrollWidth > document.documentElement.clientWidth
      };
    });

    await expect(responsiveState.appShellColumns.includes(" ")).toBe(false);
    await expect(responsiveState.topbarDirection).toBe("column");
    await expect(responsiveState.topbarActionsDirection).toBe("column");
    await expect(["nowrap", "wrap"]).toContain(responsiveState.topbarActionsWrap);
    await expect(responsiveState.cashflowColumns).toBe("1fr");
    await expect(responsiveState.hasHorizontalOverflow).toBe(false);
  });

  it("toggles dark mode through the native Tauri runtime flow", async () => {
    const body = await $("body");
    await body.waitForExist({ timeout: 60000 });

    const getTheme = () => browser.execute(() => document.body.dataset.theme ?? "");
    const getThemeButtonText = () =>
      browser.execute(() => {
        const button = document.querySelector('[data-testid="theme-toggle"]');
        return button?.textContent?.trim() ?? "";
      });

    const initialTheme = await getTheme();
    const themeButton = await $('[data-testid="theme-toggle"]');
    await themeButton.waitForExist({ timeout: 30000 });

    await themeButton.click();

    await browser.waitUntil(async () => (await getTheme()) !== initialTheme, {
      timeout: 30000,
      timeoutMsg: "РџРµСЂРµРјРёРєР°С‡ С‚РµРјРё РЅРµ Р·РјС–РЅРёРІ body[data-theme] Сѓ native Tauri runtime"
    });

    const toggledTheme = await getTheme();
    const toggledButtonText = await getThemeButtonText();

    if (initialTheme === "light") {
      await expect(toggledTheme).toBe("dark");
      await expect(toggledButtonText).toContain("Тема:");
    } else {
      await expect(toggledTheme).toBe("light");
      await expect(toggledButtonText).toContain("Тема:");
    }

    await themeButton.click();

    await browser.waitUntil(async () => (await getTheme()) === initialTheme, {
      timeout: 30000,
      timeoutMsg: "РўРµРјР° РЅРµ РїРѕРІРµСЂРЅСѓР»Р°СЃСЊ Сѓ РїРѕС‡Р°С‚РєРѕРІРёР№ СЃС‚Р°РЅ РїС–СЃР»СЏ smoke toggle"
    });
  });
});
