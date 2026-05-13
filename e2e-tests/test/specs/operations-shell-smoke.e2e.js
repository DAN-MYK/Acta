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

describe("Acta smoke для shell, задач і контрагентів", () => {
  it("відкриває command palette з підготовленими результатами", async () => {
    const body = await $("body");
    await body.waitForExist({ timeout: 60000 });

    const paletteToggle = await byTestId("palette-toggle");
    await paletteToggle.waitForExist({ timeout: 30000 });
    await paletteToggle.click();

    const palette = await byTestId("palette");
    await palette.waitForExist({ timeout: 30000 });
    await byTestId("palette-items").waitForExist({ timeout: 30000 });
    await byTestId("palette-item-0").waitForExist({ timeout: 30000 });

    await browser.keys("Escape");
  });

  it("показує focus workflow задач після переходу зі shell", async () => {
    const tasksNav = await byTestId("nav-tasks");
    await tasksNav.waitForExist({ timeout: 30000 });
    await tasksNav.click();

    const tasksScreen = await byTestId("tasks-screen");
    await tasksScreen.waitForExist({ timeout: 30000 });

    await byTestId("tasks-focus-primary").waitForExist({ timeout: 30000 });
    await byTestId("tasks-today-panel").waitForExist({ timeout: 30000 });
    await byTestId("tasks-list").waitForExist({ timeout: 30000 });
  });

  it("показує список і валідний detail-state контрагентів", async () => {
    const counterpartiesNav = await byTestId("nav-counterparties");
    await counterpartiesNav.waitForExist({ timeout: 30000 });
    await counterpartiesNav.click();

    const counterpartiesScreen = await byTestId("counterparties-screen");
    await counterpartiesScreen.waitForExist({ timeout: 30000 });

    await byTestId("counterparties-list").waitForExist({ timeout: 30000 });
    await waitForAny(["counterparty-detail", "counterparties-empty-state"]);
    await waitForAny(["counterparty-scenario", "counterparties-empty-state"]);
  });
});
