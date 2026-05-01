function byTestId(testId) {
  return $(`[data-testid="${testId}"]`);
}

async function waitForAny(testIds) {
  await browser.waitUntil(
    async () => {
      for (const testId of testIds) {
        if (await byTestId(testId).isExisting()) {
          return true;
        }
      }

      return false;
    },
    {
      timeout: 30000,
      timeoutMsg: `Не знайдено жодного з маркерів: ${testIds.join(", ")}`
    }
  );
}

describe("Acta smoke для звітів і документів", () => {
  it("показує ключові блоки звітів після переходу зі shell", async () => {
    const body = await $("body");
    await body.waitForExist({ timeout: 60000 });

    const reportsNav = await byTestId("nav-reports");
    await reportsNav.waitForExist({ timeout: 30000 });
    await reportsNav.click();

    const reportsScreen = await byTestId("reports-screen");
    await reportsScreen.waitForExist({ timeout: 45000 });

    await byTestId("reports-focus-primary").waitForExist({ timeout: 30000 });
    await waitForAny(["reports-table-card", "reports-empty-state"]);
  });

  it("показує guided flow документів і валідний основний стан", async () => {
    const documentsNav = await byTestId("nav-documents");
    await documentsNav.waitForExist({ timeout: 30000 });
    await documentsNav.click();

    const documentsScreen = await byTestId("documents-screen");
    await documentsScreen.waitForExist({ timeout: 30000 });

    await byTestId("documents-create-strip").waitForExist({ timeout: 30000 });
    await byTestId("documents-focus-primary").waitForExist({ timeout: 30000 });
    await waitForAny(["documents-list", "documents-empty-state"]);
  });

  it("зберігає readable layout для звітів і документів на вузькому viewport", async () => {
    await browser.setWindowSize(820, 900);

    const reportsNav = await byTestId("nav-reports");
    await reportsNav.waitForExist({ timeout: 30000 });
    await reportsNav.click();
    await byTestId("reports-screen").waitForExist({ timeout: 30000 });

    const reportsResponsiveState = await browser.execute(() => {
      const focusGrid = document.querySelector(".reports-focus-grid");
      const filterGrid = document.querySelector(".reports-filter-grid");
      const tableRow = document.querySelector(".reports-table-row");

      if (!(focusGrid instanceof HTMLElement)) {
        throw new Error("reports-focus-grid не знайдено");
      }
      if (!(filterGrid instanceof HTMLElement)) {
        throw new Error("reports-filter-grid не знайдено");
      }
      if (!(tableRow instanceof HTMLElement)) {
        throw new Error("reports-table-row не знайдено");
      }

      return {
        focusColumns: window.getComputedStyle(focusGrid).gridTemplateColumns,
        filterColumns: window.getComputedStyle(filterGrid).gridTemplateColumns,
        tableColumns: window.getComputedStyle(tableRow).gridTemplateColumns,
        hasHorizontalOverflow: document.documentElement.scrollWidth > document.documentElement.clientWidth
      };
    });

    await expect(reportsResponsiveState.focusColumns.includes(" ")).toBe(false);
    await expect(reportsResponsiveState.filterColumns.includes(" ")).toBe(false);
    await expect(reportsResponsiveState.tableColumns.includes(" ")).toBe(false);
    await expect(reportsResponsiveState.hasHorizontalOverflow).toBe(false);

    const documentsNav = await byTestId("nav-documents");
    await documentsNav.waitForExist({ timeout: 30000 });
    await documentsNav.click();
    await byTestId("documents-screen").waitForExist({ timeout: 30000 });

    const documentsResponsiveState = await browser.execute(() => {
      const createStrip = document.querySelector(".create-strip");
      const focusGrid = document.querySelector(".documents-focus-grid");

      if (!(createStrip instanceof HTMLElement)) {
        throw new Error("create-strip не знайдено");
      }
      if (!(focusGrid instanceof HTMLElement)) {
        throw new Error("documents-focus-grid не знайдено");
      }

      return {
        createStripColumns: window.getComputedStyle(createStrip).gridTemplateColumns,
        focusColumns: window.getComputedStyle(focusGrid).gridTemplateColumns,
        hasHorizontalOverflow: document.documentElement.scrollWidth > document.documentElement.clientWidth
      };
    });

    await expect(documentsResponsiveState.createStripColumns.includes(" ")).toBe(false);
    await expect(documentsResponsiveState.focusColumns.includes(" ")).toBe(false);
    await expect(documentsResponsiveState.hasHorizontalOverflow).toBe(false);
  });
});
