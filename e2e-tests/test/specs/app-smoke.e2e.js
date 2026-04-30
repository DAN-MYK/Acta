describe("Acta desktop shell smoke", () => {
  it("initializes the real WebView and navigates core screens", async () => {
    const body = await $("body");
    await body.waitForExist({ timeout: 60000 });

    const dashboardHeading = await $("h2=Дашборд");
    await dashboardHeading.waitForExist({ timeout: 60000 });

    await $("button=Документи").click();
    await $("h2=Документи").waitForExist({ timeout: 30000 });

    await $("button=Платежі").click();
    await $("h2=Платежі").waitForExist({ timeout: 30000 });

    await browser.keys(["Control", "1"]);
    await $("h2=Дашборд").waitForExist({ timeout: 30000 });
  });

  it("toggles dark mode through the native Tauri runtime flow", async () => {
    const body = await $("body");
    await body.waitForExist({ timeout: 60000 });

    const getTheme = () => browser.execute(() => document.body.dataset.theme ?? "");
    const getThemeButtonText = () =>
      browser.execute(() => {
        const buttons = Array.from(document.querySelectorAll("button"));
        const button = buttons.find((candidate) => candidate.textContent?.includes("Тема:"));
        return button?.textContent?.trim() ?? "";
      });

    const initialTheme = await getTheme();
    const themeButton = await $("button*=Тема:");
    await themeButton.waitForExist({ timeout: 30000 });

    await themeButton.click();

    await browser.waitUntil(async () => (await getTheme()) !== initialTheme, {
      timeout: 30000,
      timeoutMsg: "Перемикач теми не змінив body[data-theme] у native Tauri runtime"
    });

    const toggledTheme = await getTheme();
    const toggledButtonText = await getThemeButtonText();

    if (initialTheme === "light") {
      await expect(toggledTheme).toBe("dark");
      await expect(toggledButtonText).toContain("Тема: dark");
    } else {
      await expect(toggledTheme).toBe("light");
      await expect(toggledButtonText).toContain("Тема: light");
    }

    await themeButton.click();

    await browser.waitUntil(async () => (await getTheme()) === initialTheme, {
      timeout: 30000,
      timeoutMsg: "Тема не повернулась у початковий стан після smoke toggle"
    });
  });
});
