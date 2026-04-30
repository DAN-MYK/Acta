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
});
