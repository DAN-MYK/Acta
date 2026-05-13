# UI/UX Audit — Wave 2 Critical Fixes — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Виправити критичні баги виявлені в UI/UX-аудиті 2026-05-01 (mojibake, overflow, dev-leak) і впорядкувати дизайн-фундамент (семантика табів, типографіка, undefined CSS-змінні), щоб додаток виглядав ближче до production-quality 7/10 без переробки архітектурних UX-патернів.

**Architecture:**
- Локальні правки (не зачіпають Tauri-команди, БД, бізнес-логіку).
- Frontend-only: Svelte-шаблони + CSS-токени + .css модулі під екрани.
- Спершу TDD: vitest з @testing-library/svelte рендерить компонент → перевіряє наявність нормального тексту / класів / комп'ютд-стилів. Потім — мінімальна правка коду до зеленого тесту. Після — Playwright-скріншот для візуального підтвердження.

**Tech Stack:**
- Svelte 4, TypeScript, Vite 5
- Vitest + @testing-library/svelte (вже у проекті — див. `frontend/src/lib/screens/__tests__/*.test.ts`)
- CSS custom properties (вже централізовані у `frontend/src/lib/styles/tokens.css`)
- Playwright MCP — для візуальної верифікації після кожного task

**Out of scope (винесено в окремі майбутні плани):**
- Drill-down редактор замість editor-as-sheet → окремий план
- Справжня таблиця документів з фільтрами/sorted columns → окремий план
- Дашборд із sparkline/charts → окремий план
- Зміна aesthetic direction (paper/editorial vs dark-modern) → окремий план
- Mobile breakpoint redesign → окремий план

---

## File Structure

| Файл | Призначення | Дія |
|------|-------------|-----|
| `frontend/src/lib/screens/PaymentsScreen.svelte` | UI платежів | Modify (fix mojibake) |
| `frontend/src/lib/screens/SettingsScreen.svelte` | UI налаштувань | Modify (видалити dev-leak, переробити theme на segmented control) |
| `frontend/src/lib/screens/CounterpartiesScreen.svelte` | UI контрагентів | Modify (винести `.chain-summary` з `.chain-panel-header` flex) |
| `frontend/src/lib/styles/tokens.css` | Канонічні токени | Modify (font-body 14px, control-height 38px, видалити unused, додати `--accent-strong`, `--text-primary`, `--success-text`, `--line`, `--font-base`) |
| `frontend/src/styles.css` | Глобальні стилі + tabs | Modify (новий `.tabs-pill`, `.segmented`, прибрати `.settings-nav-button.active = primary`, додати `.currency` з `tabular-nums`) |
| `frontend/src/styles/settings.css` | Стилі Settings | Modify (.settings-nav-button — pill style, не primary) |
| `frontend/src/styles/tasks.css` | Стилі Tasks (також використовується в Reports) | Modify (.task-tabs → pill style) |
| `frontend/src/lib/screens/__tests__/PaymentsScreen.test.ts` | Тест Payments | Modify (новий test: split-draft texts no mojibake) |
| `frontend/src/lib/screens/__tests__/SettingsScreen.test.ts` | Тест Settings | Modify (новий test: theme як segmented control, без dev-leak phrase) |
| `frontend/src/lib/screens/__tests__/CounterpartiesScreen.test.ts` | Тест Counterparties | Modify (новий test: chain-summary не у flex-row з explanatory text) |
| `frontend/src/__tests__/AppShell.test.ts` | Глобальний тест дизайн-системи | Modify (новий test: tokens та undefined-vars resolution) |

---

## Task 1: Fix mojibake in PaymentsScreen

**Files:**
- Modify: `frontend/src/lib/screens/PaymentsScreen.svelte:407, 458, 460, 461, 465, 473, 477, 487, 497`
- Test: `frontend/src/lib/screens/__tests__/PaymentsScreen.test.ts`

**Контекст:**
9 рядків UI-тексту в PaymentsScreen.svelte збережені з cp1251→UTF-8 mojibake. Користувач бачить нечитабельні символи у "Manual picker" і "Чернетка розподілу". Жодних вторинних залежностей — це чисто текстова правка у `.svelte`-шаблоні.

Декодування mojibake → правильний текст:
| Рядок | Зараз | Має бути |
|-------|-------|----------|
| 407 | `Р”РѕРґР°С‚Рё РґРѕ СЂРѕР·РїРѕРґС–Р»Сѓ` | `Додати до розподілу` |
| 458 | `Р§РµСЂРЅРµС‚РєР° СЂРѕР·РїРѕРґС–Р»Сѓ` | `Чернетка розподілу` |
| 460 | `РЎСѓРјР° РїР»Р°С‚РµР¶Сѓ` | `Сума платежу` |
| 461 | `вЂў Р—Р°Р»РёС€РѕРє` | `• Залишок` |
| 465 | `Р”РѕРґР°Р№С‚Рµ РґРѕРєСѓРјРµРЅС‚Рё Р· manual picker, С‰РѕР± СЃС„РѕСЂРјСѓРІР°С‚Рё СЂРѕР·РїРѕРґС–Р».` | `Додайте документи з manual picker, щоб сформувати розподіл.` |
| 473 | `вЂў Р—Р°Р»РёС€РѕРє РґРѕРєСѓРјРµРЅС‚Р°` | `• Залишок документа` |
| 477 | `<span>РЎСѓРјР°</span>` | `<span>Сума</span>` |
| 487 | `РџСЂРёР±СЂР°С‚Рё` | `Прибрати` |
| 497 | `РџС–РґС‚РІРµСЂРґРёС‚Рё СЂРѕР·РїРѕРґС–Р»` | `Підтвердити розподіл` |

- [ ] **Step 1: Написати падаючий тест на наявність нормального тексту в split-draft**

Відкрити `frontend/src/lib/screens/__tests__/PaymentsScreen.test.ts`. У цьому файлі вже є хелпер `mocks` зі store-stubs. На самому низу файлу (всередині `describe(...)`-блоку) додати ці тести:

```typescript
  it("renders split-draft strings without mojibake", async () => {
    mocks.paymentsState.set({
      ...mocks.paymentsState.snapshot(),
      splitDraft: {
        paymentId: "pay-1",
        paymentAmountStr: "100,00 грн",
        remainingAmountStr: "20,00 грн",
        allocations: []
      }
    });
    const { container } = render(PaymentsScreen);
    await tick();
    const text = container.textContent ?? "";
    expect(text).toContain("Чернетка розподілу");
    expect(text).toContain("Сума платежу");
    expect(text).toContain("Залишок");
    expect(text).toContain("Додайте документи з manual picker");
    expect(text).not.toMatch(/Р[ЎЃ°·µ]/);
    expect(text).not.toContain("вЂў");
  });

  it("renders manual-picker confirm button without mojibake", async () => {
    mocks.paymentsState.set({
      ...mocks.paymentsState.snapshot(),
      manualPicker: {
        paymentId: "pay-1",
        query: "",
        candidates: [],
        selectedCandidateId: null
      }
    });
    const { container } = render(PaymentsScreen);
    await tick();
    expect(container.textContent ?? "").toContain("Додати до розподілу");
  });
```

(Якщо в існуючому файлі немає `import { render } from "@testing-library/svelte"` або `snapshot()`-методу на store-mock — додати/доповнити перед запуском.)

- [ ] **Step 2: Запустити тести й переконатися що падають**

```bash
npm run test --prefix . -- frontend/src/lib/screens/__tests__/PaymentsScreen.test.ts
```

Очікується: FAIL з повідомленням `expected ... to contain "Чернетка розподілу"`.

- [ ] **Step 3: Замінити mojibake-рядки на коректну українську**

У `frontend/src/lib/screens/PaymentsScreen.svelte`:

Рядок 407 — замінити кнопку:
```svelte
              <button class="btn-secondary" on:click={() => payments.addSelectedManualPickerCandidateToSplit()} disabled={$payments.loading}>
                Додати до розподілу
              </button>
```

Рядки 456-501 — замінити блок `splitDraft`:
```svelte
      {#if $payments.splitDraft}
        <section class="editor-items-empty" data-testid="payments-split-draft">
          <strong>Чернетка розподілу</strong>
          <p>
            Сума платежу: {$payments.splitDraft.paymentAmountStr}
            • Залишок: {$payments.splitDraft.remainingAmountStr}
          </p>

          {#if $payments.splitDraft.allocations.length === 0}
            <p>Додайте документи з manual picker, щоб сформувати розподіл.</p>
          {:else}
            <div class="documents-list">
              {#each $payments.splitDraft.allocations as allocation}
                <div class="doc-row payment-row">
                  <div class="task-row-main">
                    <div>
                      <strong>{allocation.title}</strong>
                      <p>{getDocumentKindLabel(allocation.documentKind)} • Залишок документа: {allocation.openAmountStr}</p>
                    </div>
                    <div class="task-row-meta">
                      <label>
                        <span>Сума</span>
                        <input
                          value={allocation.amount}
                          on:input={(event) => onSplitAllocationAmountChange(allocation.documentId, event)}
                        />
                      </label>
                    </div>
                  </div>
                  <div>
                    <button class="btn-ghost" on:click={() => payments.removeSplitAllocation(allocation.documentId)}>
                      Прибрати
                    </button>
                  </div>
                </div>
              {/each}
            </div>
            <div class="editor-actions">
              <button class="btn-primary" on:click={() => payments.confirmSplitDraft()} disabled={$payments.loading}>
                Підтвердити розподіл
              </button>
            </div>
          {/if}
        </section>
      {/if}
```

(Перед редагуванням ВАЖЛИВО: відкрити файл у UTF-8 (BOM-less). Якщо редактор автоматично декодує його як cp1251 — спершу зберегти явно як UTF-8, інакше нові символи теж зіпсуються.)

- [ ] **Step 4: Перевірити що сусідні рядки 460-499 не втратили логіку (allocation amount input, remove handler)**

Прочитати `git diff frontend/src/lib/screens/PaymentsScreen.svelte` і пересвідчитись:
- callback-и `onSplitAllocationAmountChange`, `payments.removeSplitAllocation`, `payments.confirmSplitDraft` ВЖЕ існують у компоненті/сторі (вони просто були в зіпсованому тексті). Якщо їх немає — не вигадувати, повернутись до автентичного blame через `git log -p frontend/src/lib/screens/PaymentsScreen.svelte` і взяти останній валідний commit.

- [ ] **Step 5: Запустити тести — мають пройти**

```bash
npm run test --prefix . -- frontend/src/lib/screens/__tests__/PaymentsScreen.test.ts
```

Очікується: PASS обидва нові тести.

- [ ] **Step 6: Запустити повний lint+test**

```bash
npm run check --prefix .
npm run test --prefix .
```

Очікується: 0 errors.

- [ ] **Step 7: Візуальна перевірка**

Запустити `npx vite --port 1420 --strictPort` (background). Через Playwright MCP:
- Відкрити `http://localhost:1420`, навігація `Платежі`
- Натиснути `Звести` на платежі що "Не зведено"
- Натиснути `Шукати інші` (manual picker)
- Скріншот `audit-after-task1-payments.png`
- Текст на UI має бути читабельний українською.

- [ ] **Step 8: Commit**

```bash
git add frontend/src/lib/screens/PaymentsScreen.svelte frontend/src/lib/screens/__tests__/PaymentsScreen.test.ts
git commit -m "$(cat <<'EOF'
fix(payments): repair mojibake in split-draft and manual-picker labels

Nine UI strings in PaymentsScreen.svelte were saved with broken
encoding (UTF-8 → cp1251 mojibake), so users saw garbage instead
of "Чернетка розподілу", "Сума платежу", "Прибрати" etc. Restore
correct Ukrainian text and add regression tests asserting the
rendered output is free of mojibake markers (Р[ЎЃ°·µ] / вЂў).

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

---

## Task 2: Fix counterparty overview overflow

**Files:**
- Modify: `frontend/src/lib/screens/CounterpartiesScreen.svelte:195-227`
- Modify: `frontend/src/styles/counterparties.css:69-78` (видалити `.counterparty-overview` що ламає `.chain-summary`)
- Test: `frontend/src/lib/screens/__tests__/CounterpartiesScreen.test.ts`

**Контекст:**
`.chain-panel-header` — flex-row `space-between` з пояснювальним текстом зліва і `.chain-summary.counterparty-overview` (4 колонки) справа. На 1440px колонки стискаються і "Прострочка / 48 200,00 / грн" та "Тримати сценарій у русі" рендеряться обрізані. Виправлення: винести 4 KPI-блоки на окремий рядок `.counterparty-overview-grid` (повна ширина), а `.chain-panel-header` лишити лише з текстом.

- [ ] **Step 1: Написати падаючий тест на структуру оверв'ю**

У `frontend/src/lib/screens/__tests__/CounterpartiesScreen.test.ts` (всередині існуючого `describe`-блоку) додати:

```typescript
  it("renders chain-summary on its own row, not inside chain-panel-header", async () => {
    mocks.counterpartiesState.set({
      ...mocks.counterpartiesState.snapshot(),
      detail: SAMPLE_COUNTERPARTY_DETAIL
    });
    const { container } = render(CounterpartiesScreen);
    await tick();

    const overviewPanel = container.querySelector('[data-testid="counterparty-overview"]');
    expect(overviewPanel).toBeTruthy();

    const header = overviewPanel!.querySelector(".chain-panel-header");
    expect(header).toBeTruthy();
    // chain-summary НЕ має бути всередині chain-panel-header (раніше було зліва-справа,
    // що давало overflow на вузьких контейнерах).
    expect(header!.querySelector(".chain-summary")).toBeNull();

    // А натомість — як прямий нащадок overview-panel.
    const summaries = overviewPanel!.querySelectorAll(":scope > .chain-summary, :scope > .counterparty-overview-grid");
    expect(summaries.length).toBe(1);
  });
```

(`SAMPLE_COUNTERPARTY_DETAIL` уже існує в файлі — використати його; інакше створити мінімальний DTO.)

- [ ] **Step 2: Запустити, переконатись що падає**

```bash
npm run test --prefix . -- frontend/src/lib/screens/__tests__/CounterpartiesScreen.test.ts
```

Очікується: FAIL.

- [ ] **Step 3: Перенести `.chain-summary` з `.chain-panel-header` на окремий рядок**

У `frontend/src/lib/screens/CounterpartiesScreen.svelte` рядки 195-227 замінити на:

```svelte
          <div class="chain-panel counterparty-overview-panel" data-testid="counterparty-overview">
            <div class="chain-panel-header">
              <div>
                <strong>Операційна картка контрагента</strong>
                <p>
                  Праворуч зібрано не лише реквізити, а й поточний сценарій: хто це, який фінансовий стан,
                  які документи й платежі в роботі та що робити далі.
                </p>
              </div>
            </div>
            <div class="counterparty-overview-grid">
              <div class="chain-summary-block">
                <span>Баланс</span>
                <strong>{$counterparties.detail.info.balanceStr}</strong>
              </div>
              <div class="chain-summary-block">
                <span>Прострочка</span>
                <strong>{$counterparties.detail.info.overdueAmountStr}</strong>
              </div>
              <div class="chain-summary-block">
                <span>Останній контакт</span>
                <strong>{$counterparties.detail.info.lastContactDate}</strong>
              </div>
              <div class="chain-summary-block">
                <span>Наступна дія</span>
                <strong>{getScenarioTitle(
                  $counterparties.detail.info.overdueCount,
                  $counterparties.detail.info.lastContactDays,
                  $counterparties.detail.info.docCount
                )}</strong>
              </div>
            </div>
          </div>
```

- [ ] **Step 4: Додати CSS для нової grid-обгортки**

У `frontend/src/styles/counterparties.css` після рядка 78 (після `.counterparty-overview-panel`) додати:

```css
.counterparty-overview-grid {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: 12px;
  margin-top: 14px;
}

@media (max-width: 1100px) {
  .counterparty-overview-grid {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }
}

@media (max-width: 720px) {
  .counterparty-overview-grid {
    grid-template-columns: 1fr;
  }
}
```

І видалити стару конфліктну селекцію (рядки 69-78 у `counterparties.css`):

```css
/* DELETE this block — now overridden by .counterparty-overview-grid */
.counterparty-overview {
  grid-template-columns: repeat(4, minmax(0, 1fr));
}
```

(Залишити `.counterparty-overview-panel` декоратор — лише прибрати `.counterparty-overview` без -panel.)

- [ ] **Step 5: Запустити тести**

```bash
npm run test --prefix . -- frontend/src/lib/screens/__tests__/CounterpartiesScreen.test.ts
```

Очікується: PASS.

- [ ] **Step 6: Візуально перевірити в Playwright**

```bash
# Vite уже запущений з task 1
```
Через Playwright MCP: відкрити `Контрагенти`, обрати `ТОВ Ромашка`, скріншот `audit-after-task2-counterparties.png`.
Перевірити: 4 блоки (Баланс / Прострочка / Останній контакт / Наступна дія) — на повну ширину, текст не обрізаний.

- [ ] **Step 7: Commit**

```bash
git add frontend/src/lib/screens/CounterpartiesScreen.svelte frontend/src/styles/counterparties.css frontend/src/lib/screens/__tests__/CounterpartiesScreen.test.ts
git commit -m "$(cat <<'EOF'
fix(counterparties): move overview KPIs to their own row

The .chain-summary 4-column grid was nested inside .chain-panel-header
(flex space-between with explanatory text), so on 1440px viewports the
columns squeezed to ~120px and 'Прострочка / 48 200,00 / грн' wrapped
mid-amount. Lift the KPI grid to a dedicated row inside the overview
panel and use a responsive 4→2→1 breakpoint cascade.

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

---

## Task 3: Strip dev-leak comment from Settings → Appearance

**Files:**
- Modify: `frontend/src/lib/screens/SettingsScreen.svelte:106-138`
- Test: `frontend/src/lib/screens/__tests__/SettingsScreen.test.ts`

**Контекст:**
Sub-tab "Зовнішній вигляд" містить два артефакти, що проникли в production-UI:
1. Chip `Системний foundation` (рядок 113) — внутрішній термін команди, нерелевантний користувачу.
2. Параграф "Налаштування щільності поки прибрано: selector не впливав на layout і створював хибне очікування" (рядок 136) — пояснення розробника для розробника.

Прибрати обидва. Замінити пояснювальний параграф на коротке нейтральне речення під заголовком.

- [ ] **Step 1: Падаючий тест**

У `frontend/src/lib/screens/__tests__/SettingsScreen.test.ts` (всередині `describe("appearance section")` або створити такий блок) додати:

```typescript
  it("does not leak internal dev terminology in appearance section", async () => {
    mocks.settingsState.set({
      ...mocks.settingsState.snapshot(),
      section: "appearance",
      screen: SAMPLE_SETTINGS_SCREEN
    });
    const { container } = render(SettingsScreen);
    await tick();
    const text = container.textContent ?? "";
    expect(text).not.toContain("Системний foundation");
    expect(text).not.toContain("selector не впливав");
    expect(text).not.toContain("Налаштування щільності поки прибрано");
  });
```

- [ ] **Step 2: Запустити, переконатися що FAIL**

```bash
npm run test --prefix . -- frontend/src/lib/screens/__tests__/SettingsScreen.test.ts
```

Очікується: FAIL з `expected ... not to contain "Системний foundation"`.

- [ ] **Step 3: Видалити обидва артефакти**

У `frontend/src/lib/screens/SettingsScreen.svelte` секція appearance (рядки 106-138):

Замість:
```svelte
        <div class="settings-card">
          <div class="settings-section-head">
            <div>
              <h3>Зовнішній вигляд</h3>
              <p>Фіксуємо канонічні стани інтерфейсу без експериментальних перемикачів.</p>
            </div>
            <span class="state-chip is-loading">Системний foundation</span>
          </div>

          <div class="settings-actions-row">
            ...buttons...
          </div>

          <p class="hint">
            Налаштування щільності поки прибрано: selector не впливав на layout і створював хибне очікування.
          </p>
        </div>
```

Поставити:
```svelte
        <div class="settings-card">
          <div class="settings-section-head">
            <div>
              <h3>Зовнішній вигляд</h3>
              <p>Тема інтерфейсу</p>
            </div>
          </div>

          <div class="settings-actions-row">
            ...buttons (без змін у цьому task — переробляються у Task 5)...
          </div>
        </div>
```

(Зберегти існуючу структуру `<button class="btn-primary/secondary">Світла тема</button>` як є — переробка у Task 5.)

- [ ] **Step 4: Запустити тести — мають пройти**

```bash
npm run test --prefix . -- frontend/src/lib/screens/__tests__/SettingsScreen.test.ts
```

Очікується: PASS.

- [ ] **Step 5: Візуальна перевірка**

Через Playwright MCP: `Налаштування` → `Зовнішній вигляд`, скріншот `audit-after-task3-settings-appearance.png`.
Перевірити: ані chip "Системний foundation", ані параграф про "selector" не видно.

- [ ] **Step 6: Commit**

```bash
git add frontend/src/lib/screens/SettingsScreen.svelte frontend/src/lib/screens/__tests__/SettingsScreen.test.ts
git commit -m "$(cat <<'EOF'
fix(settings): remove dev-leak copy from Appearance section

Two internal artifacts had leaked into the production UI:
- 'Системний foundation' state-chip (internal team jargon).
- Paragraph explaining that density selector was removed because
  it 'did not affect layout' (developer-facing rationale).

Both are noise to the end user; replace the section header copy
with a neutral 'Тема інтерфейсу' subtitle.

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

---

## Task 4: Tabs as pill controls (not primary CTAs)

**Files:**
- Modify: `frontend/src/styles.css` (нова композиція `.tabs-pill`)
- Modify: `frontend/src/styles/tasks.css:58-70` (`.task-tabs` — посилається на новий стиль)
- Modify: `frontend/src/styles/settings.css:24-29` (`.settings-nav button.active` — pill-style)
- Test: `frontend/src/__tests__/AppShell.test.ts` (computed style для активного табу — без `--accent` filled background; має underline border)

**Контекст:**
Активна вкладка в Settings/Tasks/Reports має `background: var(--accent); color: var(--text-on-accent)` — виглядає як primary CTA. Семантично таб — це навігація, не дія. Перевести у pill-style: активна tab має м'який фон `accent-soft`, текст `accent-text`, тонку bottom-border `accent`.

`.task-tabs` уже використовується спільно у Tasks і Reports (`ReportsScreen.svelte:174`), тому правка в одному `.task-tabs` поправить обидва.

- [ ] **Step 1: Падаючий тест на стиль активного табу**

У `frontend/src/__tests__/AppShell.test.ts` (або новому `frontend/src/__tests__/tabs.test.ts`) додати:

```typescript
import { describe, it, expect } from "vitest";

describe("design-system: tabs pill style", () => {
  it("active .task-tabs button uses pill style, not accent fill", () => {
    document.head.innerHTML = "";
    const link = document.createElement("link");
    link.rel = "stylesheet";
    // у jsdom CSS не парситься повноцінно; натомість проганяємо через style-сетер:
    const style = document.createElement("style");
    style.textContent = require("fs")
      .readFileSync("frontend/src/lib/styles/tokens.css", "utf8") +
      require("fs").readFileSync("frontend/src/styles.css", "utf8") +
      require("fs").readFileSync("frontend/src/styles/tasks.css", "utf8");
    document.head.appendChild(style);

    const wrapper = document.createElement("div");
    wrapper.className = "task-tabs";
    wrapper.innerHTML = `<button class="active">Active</button><button>Other</button>`;
    document.body.appendChild(wrapper);
    const active = wrapper.querySelector("button.active") as HTMLElement;
    const cs = getComputedStyle(active);
    // Активний таб НЕ має бути сполошним accent-fill (як btn-primary).
    // Перевіряємо що він НЕ має color === text-on-accent.
    expect(cs.color).not.toMatch(/255,\s*253,\s*248/); // --text-on-accent #fffdf8
    // А має accent-text-кольору (рідкий синій) і помітний border-bottom.
    expect(cs.borderBottomWidth).not.toBe("0px");
  });
});
```

(Якщо jsdom погано парсить `color-mix()` — впевнитись, що тест не валиться через це; в крайньому випадку assert на наявність CSS-property з очікуваною підстрокою з самого `style.textContent`.)

- [ ] **Step 2: Запустити, FAIL**

```bash
npm run test --prefix . -- frontend/src/__tests__/AppShell.test.ts
```

Очікується: FAIL (зараз active має accent fill).

- [ ] **Step 3: Додати утиліту `.tabs-pill` у `frontend/src/styles.css`**

Після блоку `.btn-danger:hover { ... }` (рядок ~848) додати:

```css
/* --- Pill-tabs (not primary CTAs) --- */

.tabs-pill {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
  padding: 4px;
  border-radius: var(--radius-xl);
  background: var(--bg-subtle);
  border: 1px solid var(--border-hairline);
  width: fit-content;
}

.tabs-pill > button {
  border: 0;
  background: transparent;
  border-radius: var(--radius-lg);
  padding: 8px 14px;
  cursor: pointer;
  color: var(--text-muted);
  font-weight: 500;
  transition: background 160ms ease, color 160ms ease;
}

.tabs-pill > button:hover:not(.active) {
  background: var(--bg-hover);
  color: var(--text);
}

.tabs-pill > button.active {
  background: var(--bg-elevated);
  color: var(--text);
  box-shadow: 0 1px 0 rgba(0, 0, 0, 0.04), 0 1px 2px rgba(0, 0, 0, 0.06);
}
```

- [ ] **Step 4: Замінити стиль `.task-tabs` у `frontend/src/styles/tasks.css:58-70`**

Видалити:
```css
.task-tabs button,
.task-row button:last-child {
  border: 0;
  background: var(--bg-card);
  border-radius: var(--radius-xl);
  padding: 10px 14px;
  cursor: pointer;
}

.task-tabs button.active {
  background: var(--accent);
  color: var(--text-on-accent);
}
```

Поставити:
```css
.task-tabs {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
  padding: 4px;
  border-radius: var(--radius-xl);
  background: var(--bg-subtle);
  border: 1px solid var(--border-hairline);
  width: fit-content;
}

.task-tabs button {
  border: 0;
  background: transparent;
  border-radius: var(--radius-lg);
  padding: 8px 14px;
  cursor: pointer;
  color: var(--text-muted);
  font-weight: 500;
}

.task-tabs button:hover:not(.active) {
  background: var(--bg-hover);
  color: var(--text);
}

.task-tabs button.active {
  background: var(--bg-elevated);
  color: var(--text);
  box-shadow: 0 1px 0 rgba(0, 0, 0, 0.04), 0 1px 2px rgba(0, 0, 0, 0.06);
}

.task-row button:last-child {
  border: 0;
  background: var(--bg-card);
  border-radius: var(--radius-xl);
  padding: 10px 14px;
  cursor: pointer;
}
```

- [ ] **Step 5: Замінити стиль `.settings-nav button.active` у `frontend/src/styles/settings.css:24-29`**

Видалити:
```css
.settings-nav button.active {
  background: var(--accent);
  color: var(--text-on-accent);
  border-color: transparent;
  box-shadow: var(--button-shadow);
}
```

Поставити:
```css
.settings-nav-button {
  position: relative;
  background: transparent;
  border: 0;
  border-left: 2px solid transparent;
  border-radius: 0;
  padding: 10px 14px;
}

.settings-nav button.active {
  background: var(--accent-soft);
  color: var(--accent-text);
  border-left-color: var(--accent);
  box-shadow: none;
}

.settings-nav button:hover:not(.active) {
  background: var(--bg-hover);
}
```

(Sidebar tabs — vertical, тому замість bottom-border використовуємо left-border-accent + м'який fill.)

- [ ] **Step 6: Запустити тести**

```bash
npm run test --prefix .
```

Очікується: всі PASS.

- [ ] **Step 7: Візуальна перевірка**

Через Playwright MCP:
- `Завдання` → screenshot `audit-after-task4-tasks-tabs.png` (3 таби "У фокусі / Завершені / Усі")
- `Звіти` → screenshot `audit-after-task4-reports-tabs.png` (4 таби "Рух грошей / P&L / Нам мають / Ми винні")
- `Налаштування` → screenshot `audit-after-task4-settings-nav.png` (vertical nav)

Перевірити: активна вкладка має м'який фон / left-accent-border, **НЕ виглядає як primary action**.

- [ ] **Step 8: Commit**

```bash
git add frontend/src/styles.css frontend/src/styles/tasks.css frontend/src/styles/settings.css frontend/src/__tests__/AppShell.test.ts
git commit -m "$(cat <<'EOF'
refactor(design-system): tabs are pill-controls, not primary CTAs

Active tabs in Tasks, Reports and Settings used 'background: var(--accent);
color: var(--text-on-accent)', visually identical to btn-primary. Users
mistook navigation tabs for call-to-action buttons.

Introduce a .tabs-pill utility (soft container + neutral active state)
and apply it to .task-tabs (shared by Tasks and Reports) and the Settings
vertical nav (left-accent-border instead of fill).

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

---

## Task 5: Settings theme — segmented control instead of dual buttons

**Files:**
- Modify: `frontend/src/lib/screens/SettingsScreen.svelte:116-133`
- Modify: `frontend/src/styles.css` (новий компонент `.segmented`)
- Test: `frontend/src/lib/screens/__tests__/SettingsScreen.test.ts`

**Контекст:**
Вибір теми зараз — дві окремі кнопки `Світла тема` і `Темна тема`, одна `btn-primary`, інша `btn-secondary`. Це бінарний стан, отже коректний паттерн — segmented control: один контейнер, дві половини, активна — заповнена, неактивна — прозора.

- [ ] **Step 1: Падаючий тест на структуру segmented control**

У `frontend/src/lib/screens/__tests__/SettingsScreen.test.ts`:

```typescript
  it("renders theme picker as segmented control with active option", async () => {
    mocks.settingsState.set({
      ...mocks.settingsState.snapshot(),
      section: "appearance",
      screen: { ...SAMPLE_SETTINGS_SCREEN, preferences: { ...SAMPLE_SETTINGS_SCREEN.preferences, darkMode: false } }
    });
    const { container } = render(SettingsScreen);
    await tick();

    const seg = container.querySelector('[data-testid="theme-segmented"]');
    expect(seg).toBeTruthy();
    expect(seg!.classList.contains("segmented")).toBe(true);

    const buttons = seg!.querySelectorAll("button");
    expect(buttons.length).toBe(2);
    expect(buttons[0].classList.contains("active")).toBe(true);   // light
    expect(buttons[1].classList.contains("active")).toBe(false);  // dark
    // НЕ btn-primary, НЕ btn-secondary
    expect(buttons[0].classList.contains("btn-primary")).toBe(false);
    expect(buttons[1].classList.contains("btn-secondary")).toBe(false);
  });
```

- [ ] **Step 2: Запустити, FAIL**

```bash
npm run test --prefix . -- frontend/src/lib/screens/__tests__/SettingsScreen.test.ts
```

- [ ] **Step 3: Замінити дві кнопки на segmented control у `SettingsScreen.svelte:116-133`**

Видалити:
```svelte
          <div class="settings-actions-row">
            <button
              class={!$settings.screen?.preferences.darkMode ? "btn-primary" : "btn-secondary"}
              on:click={() => onSettingsThemeChange(false)}
              ...
            >
              Світла тема
            </button>
            <button
              class={$settings.screen?.preferences.darkMode ? "btn-primary" : "btn-secondary"}
              on:click={() => onSettingsThemeChange(true)}
              ...
            >
              Темна тема
            </button>
          </div>
```

Поставити:
```svelte
          <div class="segmented" data-testid="theme-segmented" role="radiogroup" aria-label="Тема інтерфейсу">
            <button
              type="button"
              role="radio"
              aria-checked={!$settings.screen?.preferences.darkMode}
              class:active={!$settings.screen?.preferences.darkMode}
              on:click={() => onSettingsThemeChange(false)}
              disabled={$settings.loading}
            >
              Світла
            </button>
            <button
              type="button"
              role="radio"
              aria-checked={$settings.screen?.preferences.darkMode}
              class:active={$settings.screen?.preferences.darkMode}
              on:click={() => onSettingsThemeChange(true)}
              disabled={$settings.loading}
            >
              Темна
            </button>
          </div>
```

- [ ] **Step 4: Додати CSS `.segmented` у `frontend/src/styles.css`**

Після блоку `.tabs-pill` (з Task 4) додати:

```css
/* --- Segmented control (binary/ternary state picker) --- */

.segmented {
  display: inline-flex;
  padding: 4px;
  border-radius: var(--radius-xl);
  background: var(--bg-subtle);
  border: 1px solid var(--border-hairline);
  gap: 0;
}

.segmented > button {
  border: 0;
  background: transparent;
  border-radius: var(--radius-lg);
  padding: 8px 18px;
  cursor: pointer;
  color: var(--text-muted);
  font-weight: 500;
  min-width: 96px;
  transition: background 160ms ease, color 160ms ease;
}

.segmented > button:hover:not(.active):not(:disabled) {
  color: var(--text);
}

.segmented > button.active {
  background: var(--bg-elevated);
  color: var(--text);
  box-shadow: 0 1px 0 rgba(0, 0, 0, 0.04), 0 1px 2px rgba(0, 0, 0, 0.06);
}

.segmented > button:disabled {
  cursor: not-allowed;
}
```

- [ ] **Step 5: Запустити тести**

```bash
npm run test --prefix . -- frontend/src/lib/screens/__tests__/SettingsScreen.test.ts
```

Очікується: PASS.

- [ ] **Step 6: Візуальна перевірка**

Playwright: `Налаштування → Зовнішній вигляд`, screenshot `audit-after-task5-theme-segmented.png`. Перемкнути тему — переконатись що active-стан переходить між половинами.

- [ ] **Step 7: Commit**

```bash
git add frontend/src/lib/screens/SettingsScreen.svelte frontend/src/styles.css frontend/src/lib/screens/__tests__/SettingsScreen.test.ts
git commit -m "$(cat <<'EOF'
refactor(settings): theme picker as segmented control

Light/Dark theme was rendered as two adjacent buttons (one primary,
one secondary), implying two distinct CTAs. It is a binary state
picker, so use a proper segmented control: one rounded container,
two equal halves, active half elevated.

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

---

## Task 6: Typography & control-height pass + tabular-nums

**Files:**
- Modify: `frontend/src/lib/styles/tokens.css:7-13, 32` (font-body, control-height, видалити невикористаний `--font-serif`)
- Modify: `frontend/src/styles.css` (додати `.currency` utility)
- Test: `frontend/src/__tests__/AppShell.test.ts`

**Контекст:**
- `--font-body: 13px` → `14px` (стандарт desktop, без втрати щільності)
- `--control-height: 46px` → `38px` (стандарт; зараз форми "роздуті")
- `--font-serif: "Source Serif 4"` ніде не використовується → видалити
- Грошові суми не мають `font-variant-numeric: tabular-nums` → додати utility-клас `.currency` і використати на критичних місцях (KPI-картки, таблиці)

- [ ] **Step 1: Падаючий тест на токени і `.currency`**

У `frontend/src/__tests__/AppShell.test.ts` додати:

```typescript
import { describe, it, expect } from "vitest";
import { readFileSync } from "fs";

describe("design-system: typography tokens", () => {
  const tokens = readFileSync("frontend/src/lib/styles/tokens.css", "utf8");
  const styles = readFileSync("frontend/src/styles.css", "utf8");

  it("--font-body is 14px", () => {
    expect(tokens).toMatch(/--font-body:\s*14px/);
  });

  it("--control-height is 38px", () => {
    expect(tokens).toMatch(/--control-height:\s*38px/);
  });

  it("--font-serif is removed (was unused)", () => {
    expect(tokens).not.toMatch(/--font-serif:/);
  });

  it(".currency utility uses tabular-nums", () => {
    expect(styles).toMatch(/\.currency\s*\{[^}]*font-variant-numeric:\s*tabular-nums/);
  });
});
```

- [ ] **Step 2: Запустити, FAIL**

```bash
npm run test --prefix . -- frontend/src/__tests__/AppShell.test.ts
```

- [ ] **Step 3: Виправити токени у `frontend/src/lib/styles/tokens.css`**

Замінити рядки 3-5:
```css
  --font-sans: "Geist", "Segoe UI", sans-serif;
  --font-serif: "Source Serif 4", Georgia, serif;
  --font-mono: "JetBrains Mono", "Cascadia Code", monospace;
```

На:
```css
  --font-sans: "Geist", "Segoe UI", sans-serif;
  --font-mono: "JetBrains Mono", "Cascadia Code", monospace;
```

Замінити рядок 9 `--font-body: 13px;` → `--font-body: 14px;`

Замінити рядок 32 `--control-height: 46px;` → `--control-height: 38px;`

Замінити рядок 33 `--control-height-multiline: 110px;` → `--control-height-multiline: 96px;`

Замінити рядок 34 `--control-padding-x: 14px;` → `--control-padding-x: 12px;`

Замінити рядок 35 `--control-padding-y: 12px;` → `--control-padding-y: 8px;`

- [ ] **Step 4: Додати `.currency` у `frontend/src/styles.css`**

Після `.title-with-icon { ... }` (рядок ~553) додати:

```css
.currency,
.dashboard-kpi-card strong,
.documents-focus-card strong,
.reports-focus-card strong,
.chain-summary-block strong,
.editor-item-total strong,
.cashflow-net,
.cashflow-income,
.cashflow-expense {
  font-variant-numeric: tabular-nums;
}
```

- [ ] **Step 5: Запустити тести**

```bash
npm run test --prefix .
```

Очікується: PASS. Якщо існуючі screen-тести впали через зміну висоти — оновити очікування (висота — імплементаційна деталь, тести мають перевіряти семантику).

- [ ] **Step 6: Візуальна перевірка**

Через Playwright: дашборд + платежі + редактор документа.
Screenshots `audit-after-task6-{dashboard,payments,doc-editor}.png`.

Перевірити: форми компактніші, не "роздуті"; цифри в KPI-картках і таблицях вирівняні по символу.

- [ ] **Step 7: Commit**

```bash
git add frontend/src/lib/styles/tokens.css frontend/src/styles.css frontend/src/__tests__/AppShell.test.ts
git commit -m "$(cat <<'EOF'
refactor(design-system): tighten typography and control sizing

- --font-body 13px → 14px (desktop-standard).
- --control-height 46px → 38px; padding 14/12 → 12/8 (less inflated forms).
- Remove unused --font-serif token (Source Serif 4 was declared but
  never referenced by any rule).
- New .currency utility (font-variant-numeric: tabular-nums) applied
  to KPI strongs and ledger columns so digits align by glyph width.

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

---

## Task 7: Resolve undefined CSS variables

**Files:**
- Modify: `frontend/src/lib/styles/tokens.css` (додати missing vars)
- Test: `frontend/src/__tests__/AppShell.test.ts`

**Контекст:**
Існуючий код посилається на CSS-змінні, що не оголошені в `:root`:
- `--font-base` — `frontend/src/styles.css:261, 288`
- `--accent-strong` — `frontend/src/styles.css:272`
- `--text-primary` — `frontend/src/styles.css:394`
- `--success-text` — `frontend/src/styles/counterparties.css:66`
- `--line` — `frontend/src/styles/counterparties.css:120`

Ці посилання резолвляться в `initial`-значення і ламають візуал на тих елементах. Або додати їх у tokens.css, або замінити на існуючі. Найбезпечніше — додати їх як алейаси, бо це невелика правка і не зачіпає інших файлів.

- [ ] **Step 1: Падаючий тест**

У `frontend/src/__tests__/AppShell.test.ts`:

```typescript
describe("design-system: undefined CSS variables", () => {
  const tokens = readFileSync("frontend/src/lib/styles/tokens.css", "utf8");

  it.each([
    "--font-base",
    "--accent-strong",
    "--text-primary",
    "--success-text",
    "--line"
  ])("declares %s", (name) => {
    expect(tokens).toMatch(new RegExp(`${name}:\\s*[^;]+;`));
  });
});
```

- [ ] **Step 2: Запустити, FAIL для всіх 5**

```bash
npm run test --prefix . -- frontend/src/__tests__/AppShell.test.ts
```

- [ ] **Step 3: Додати ці змінні у `frontend/src/lib/styles/tokens.css`**

У `:root { ... }` після рядка 64 (після `--accent-text`) додати:

```css
  --accent-strong: var(--accent-hover);
  --text-primary: var(--text);
  --success-text: color-mix(in srgb, var(--success) 78%, var(--text));
  --line: var(--border);
  --font-base: var(--font-body);
```

І в `body[data-theme="dark"] { ... }` після `--accent-text` (рядок ~123) додати такі самі алейаси (з тими ж токенами — вони перерахуються через каскад):

```css
  --accent-strong: var(--accent-hover);
  --text-primary: var(--text);
  --success-text: color-mix(in srgb, var(--success) 70%, var(--text));
  --line: var(--border);
  --font-base: var(--font-body);
```

- [ ] **Step 4: Запустити тести — мають PASS**

```bash
npm run test --prefix . -- frontend/src/__tests__/AppShell.test.ts
```

- [ ] **Step 5: Перевірити що всі тести у репо проходять**

```bash
npm run test --prefix .
npm run check --prefix .
```

- [ ] **Step 6: Візуальна перевірка**

Дашборд + контрагенти + документи (там де використовуються вищеназвані токени). Жодних регресій.

- [ ] **Step 7: Commit**

```bash
git add frontend/src/lib/styles/tokens.css frontend/src/__tests__/AppShell.test.ts
git commit -m "$(cat <<'EOF'
fix(design-system): declare previously undefined CSS variables

Five vars were referenced by component CSS but never declared in
tokens.css, so they fell back to 'initial' and silently broke the
intended visual:

  --font-base       (styles.css:261, 288)
  --accent-strong   (styles.css:272)
  --text-primary    (styles.css:394)
  --success-text    (counterparties.css:66)
  --line            (counterparties.css:120)

Declare each as an alias to an existing token, both for light and
dark themes. No call-site changes; existing references now resolve.

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

---

## Final verification

- [ ] **Step F1: Повний прогін тестів і lint**

```bash
npm run test --prefix .
npm run check --prefix .
cargo build --tests   # лишити Rust сторону у валідному стані
```

Очікується: 0 failures, 0 type-errors, 0 lint-errors.

- [ ] **Step F2: E2E smoke**

```bash
npm run test:e2e --prefix . -- --spec='./e2e-tests/test/specs/app-smoke.e2e.js'
```

(Якщо запущено в Tauri-контексті — `cd src-tauri && cargo tauri build` і перевірка smoke на bundled-білді не входить у scope цього плану.)

- [ ] **Step F3: Manual exploratory pass**

Через Playwright MCP пройти 7 екранів і зафіксувати фінальні скріншоти `audit-final-{01..07}.png`. Порівняти з оригінальним аудитом 2026-05-01.

- [ ] **Step F4: Оновити документ аудиту**

У майбутньому commit-і додати summary внизу `docs/superpowers/plans/2026-05-01-uiux-audit-wave-2-critical-fixes.md`:

```markdown
---
## Wave 2 — Completed YYYY-MM-DD

| Task | Status |
|------|--------|
| 1. Mojibake | ✅ |
| 2. Counterparty overflow | ✅ |
| 3. Settings dev-leak | ✅ |
| 4. Tabs as pill | ✅ |
| 5. Theme segmented | ✅ |
| 6. Typography pass | ✅ |
| 7. Undefined CSS vars | ✅ |

Audit score: 5/10 → ~7/10. Out-of-scope items deferred to subsequent
waves (drill-down editor, real document table, dashboard charts,
aesthetic direction, mobile redesign).
```

---

## Self-Review Notes

**Coverage check:**
- Task 1 → P0.1 mojibake ✓
- Task 2 → P0.2 counterparty overflow ✓
- Task 3 → P0.3 dev-leak comment ✓
- Task 4 → P1.5 tabs semantics (Settings + Tasks + Reports) ✓
- Task 5 → P1.5 theme picker semantics ✓
- Task 6 → P1.4 typography (font-body, control-height, font-serif unused, tabular-nums) ✓
- Task 7 → P3.9 undefined CSS vars ✓

**Deferred (документуємо явно як майбутні плани):**
- Editor-as-sheet → drill-down — потребує проектування навігаційного state-machine ⇒ окремий план.
- Documents справжня таблиця з фільтрами — окремий план.
- Dashboard charts — окремий план.
- Aesthetic direction selection — окремий brainstorming.
- Mobile breakpoint — окремий план.
- Skeleton-loaders — є існуючий план `2026-05-01-skeleton-loaders.md`, не повторюємо.

**Type/identifier consistency:**
- `.tabs-pill` — оголошено в Task 4, посилання тільки в межах того самого task; `.task-tabs` лишається існуючим, але переписується щоб використати ту саму логіку. ✓
- `.segmented` — в Task 5; жодного перетину з `.tabs-pill`. ✓
- Усі додані `--*` змінні в Task 6/7 узгоджені (font-body 14px, control-height 38px) і використовуються там, де вже були потрібні. ✓
