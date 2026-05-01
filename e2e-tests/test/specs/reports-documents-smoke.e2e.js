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
    await reportsScreen.waitForExist({ timeout: 30000 });

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
});
