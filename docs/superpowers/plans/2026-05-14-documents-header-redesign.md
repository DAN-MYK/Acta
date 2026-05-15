# Documents Header Redesign Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Перебудувати шапку Documents screen у 2 рядки, прибрати дублюючі quick/create controls, додати smart Create та floating filter popover.

**Architecture:** Зміна локалізована у Svelte-компоненті DocumentsScreen, стилях `documents.css` і component tests. Store, Rust, Tauri commands, SQL і доменні типи не змінюються; `DOCUMENT_FILTER_PRESETS` лишається для store `applyPreset`, але більше не рендериться на екрані.

**Tech Stack:** Svelte 4, TypeScript, Vitest/jsdom, CSS variables Acta design system.

---

## File Structure

- Modify: `frontend/src/lib/screens/DocumentsScreen.svelte`
  - Прибрати імпорт/рендер `DOCUMENT_FILTER_PRESETS`, local state `createKind`, `createButton`, `onSelectCreateKind`, `focusCreateButton`.
  - Додати refs `filterButton`, `filterPopover`, `createMenuButton`, `createMenuPopover` і state `createMenuOpen`.
  - Замінити окремі kind chips / filter toolbar / create bar на один `documents-toolbar`.
  - Перенести filter panel у popover і додати create picker popover.
- Modify: `frontend/src/styles/documents.css`
  - Видалити стилі для `.documents-presets-row`, `.documents-presets-label`, `.documents-create-kind-chips`, `.documents-create-bar`, `.documents-filter-toolbar`, inline `.documents-filter-panel`.
  - Додати `.documents-toolbar`, `.documents-toolbar-actions`, `.documents-toolbar-popover-anchor`, `.filter-popover`, `.filter-popover-btn-active`, `.create-picker-popover`, `.create-picker-item`.
  - Оновити responsive rules під новий toolbar.
- Modify: `frontend/src/lib/screens/__tests__/DocumentsScreen.test.ts`
  - Прибрати очікування quick presets і create kind strip.
  - Додати покриття smart Create, create picker, filter popover open/close і актуальних CSS contracts.

---

### Task 1: Оновити component tests під нову шапку

**Files:**
- Modify: `frontend/src/lib/screens/__tests__/DocumentsScreen.test.ts`

- [ ] **Step 1: Замінити smoke expectations для create controls**

У тесті `renders the main shell, item summary and existing PDF flow` прибрати очікування одночасної присутності "Створити акт" і "Створити накладну". Перевіряти дефолтний smart button:

```ts
expect(target.textContent).toContain("Створити ▾");
expect(target.textContent).not.toContain("Створити накладну");
```

- [ ] **Step 2: Оновити hierarchy test**

Перейменувати опис:

```ts
it("uses canonical button hierarchy in toolbar and editor header", () => {
```

Залишити перевірку `documents-create-button` як `btn-primary`; editor header assertions не міняти.

- [ ] **Step 3: Замінити create-strip test на smart create default behavior**

Замінити тест `creates a draft without a preliminary counterparty selection` на:

```ts
it("opens the create picker when no document kind filter is selected", async () => {
  setDocumentsStateWithoutDraftContext();
  const { component, target } = renderDocuments();

  const createButton = target.querySelector('[data-testid="documents-create-button"]') as HTMLButtonElement;
  expect(createButton.textContent).toContain("Створити ▾");
  expect(target.querySelector('[data-testid="documents-create-picker"]')).toBeNull();

  createButton.click();
  await tick();

  const picker = target.querySelector('[data-testid="documents-create-picker"]');
  expect(picker).toBeTruthy();
  expect(mocks.create).not.toHaveBeenCalled();

  (target.querySelector('[data-testid="documents-create-picker-act"]') as HTMLButtonElement).click();
  await tick();

  expect(mocks.create).toHaveBeenCalledWith(undefined, "act");
  expect(target.querySelector('[data-testid="documents-create-picker"]')).toBeNull();

  component.$destroy();
});
```

- [ ] **Step 4: Додати direct create test для active kind filter**

Додати поруч:

```ts
it("creates the filtered document kind directly", async () => {
  mocks.documentsState.set({
    list: makeList(), editor: null, chain: null,
    draftContext: { counterpartyId: "counterparty-1", counterpartyName: "ТОВ Ромашка" },
    selectedIds: [], initialLoading: false,
    loading: false, error: null, message: null,
    activeTab: "all" as const, kindFilter: "act" as const,
    counterpartyFilterId: null, dateFrom: null, dateTo: null,
    statusFilter: [], amountMin: null, amountMax: null,
    overdueOnly: false, activePresetId: null
  });
  const { component, target } = renderDocuments();

  const createButton = target.querySelector('[data-testid="documents-create-button"]') as HTMLButtonElement;
  expect(createButton.textContent).toContain("Створити акт");

  createButton.click();
  await tick();

  expect(mocks.create).toHaveBeenCalledWith("counterparty-1", "act");
  expect(target.querySelector('[data-testid="documents-create-picker"]')).toBeNull();

  component.$destroy();
});
```

- [ ] **Step 5: Оновити stale draft counterparty test**

У тесті `does not reuse a stale draft counterparty after context is cleared` після click по create button додати click по `documents-create-picker-act`, бо без kind filter перший click відкриває picker:

```ts
(target.querySelector('[data-testid="documents-create-button"]') as HTMLButtonElement).click();
await tick();
(target.querySelector('[data-testid="documents-create-picker-act"]') as HTMLButtonElement).click();

expect(mocks.create).toHaveBeenCalledWith(undefined, "act");
```

- [ ] **Step 6: Оновити route actions test**

У тесті `routes create and editor actions into the documents store` замінити direct create click на відкриття picker і вибір акту:

```ts
(target.querySelector('[data-testid="documents-create-button"]') as HTMLButtonElement).click();
await tick();
(target.querySelector('[data-testid="documents-create-picker-act"]') as HTMLButtonElement).click();
```

Очікування `expect(mocks.create).toHaveBeenCalledWith("counterparty-1", "act");` лишити.

- [ ] **Step 7: Замінити compact CSS contract test**

У тесті `uses a compact mode that de-emphasizes idle bulk actions` прибрати checks для `.documents-create-bar` і `.documents-create-kind-chips`, додати:

```ts
expect(source).toContain('class="documents-toolbar"');
expect(source).not.toContain('class="documents-create-kind-chips"');
expect(styles).toMatch(/@media\s*\(max-width:\s*980px\)[\s\S]*\.documents-toolbar\s*\{[\s\S]*align-items:\s*stretch/);
expect(styles).toMatch(/@media\s*\(max-width:\s*720px\)[\s\S]*\.filter-popover\s*\{[\s\S]*width:\s*min\(340px,\s*calc\(100vw - 32px\)\)/);
```

- [ ] **Step 8: Видалити preset chip test**

Повністю видалити тест `preset chip calls applyPreset with correct id`, бо quick presets більше не рендеряться.

- [ ] **Step 9: Додати filter popover open/close test**

Додати:

```ts
it("opens and closes the filter popover from the toolbar", async () => {
  const { component, target } = renderDocuments();

  const filterButton = target.querySelector('[data-testid="documents-filter-button"]') as HTMLButtonElement;
  filterButton.click();
  await tick();

  expect(target.querySelector('[data-testid="documents-filter-panel"]')).toBeTruthy();
  expect(filterButton.className).toContain("filter-popover-btn-active");

  filterButton.click();
  await tick();

  expect(target.querySelector('[data-testid="documents-filter-panel"]')).toBeNull();

  component.$destroy();
});
```

- [ ] **Step 10: Додати click-outside/Escape tests для меню**

Додати два короткі tests:

```ts
it("closes floating document menus on outside click", async () => {
  const { component, target } = renderDocuments();

  (target.querySelector('[data-testid="documents-filter-button"]') as HTMLButtonElement).click();
  (target.querySelector('[data-testid="documents-create-button"]') as HTMLButtonElement).click();
  await tick();

  window.dispatchEvent(new MouseEvent("click", { bubbles: true }));
  await tick();

  expect(target.querySelector('[data-testid="documents-filter-panel"]')).toBeNull();
  expect(target.querySelector('[data-testid="documents-create-picker"]')).toBeNull();

  component.$destroy();
});

it("closes floating document menus on Escape", async () => {
  const { component, target } = renderDocuments();

  (target.querySelector('[data-testid="documents-filter-button"]') as HTMLButtonElement).click();
  (target.querySelector('[data-testid="documents-create-button"]') as HTMLButtonElement).click();
  await tick();

  window.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", code: "Escape", bubbles: true }));
  await tick();

  expect(target.querySelector('[data-testid="documents-filter-panel"]')).toBeNull();
  expect(target.querySelector('[data-testid="documents-create-picker"]')).toBeNull();

  component.$destroy();
});
```

- [ ] **Step 11: Запустити targeted test і побачити fail**

Run:

```bash
npm run test:frontend -- frontend/src/lib/screens/__tests__/DocumentsScreen.test.ts
```

Expected: FAIL, бо component ще має старі selectors/markup.

---

### Task 2: Перебудувати state та handlers у DocumentsScreen

**Files:**
- Modify: `frontend/src/lib/screens/DocumentsScreen.svelte`

- [ ] **Step 1: Почистити imports**

У `../config/ui` прибрати:

```ts
DOCUMENT_FILTER_PRESETS,
```

`DOCUMENT_KIND_OPTIONS` лишити для create picker.

- [ ] **Step 2: Замінити local variables**

Прибрати:

```ts
let createKind: DocumentKind = "act";
let createButton: HTMLButtonElement | null = null;
```

Додати поруч із menu refs:

```ts
let filterButton: HTMLButtonElement | null = null;
let filterPopover: HTMLElement | null = null;
let createMenuOpen = false;
let createMenuButton: HTMLButtonElement | null = null;
let createMenuPopover: HTMLElement | null = null;
```

- [ ] **Step 3: Замінити chain-only click outside handler**

Замінити `onWindowClickForChainMenu` на:

```ts
function onWindowClick(event: MouseEvent) {
  const target = event.target as Node | null;

  if (chainMenuOpen) {
    if (target && chainMenuButton?.contains(target)) return;
    if (target && chainMenuPopover?.contains(target)) return;
    closeChainMenu();
  }

  if (filtersOpen) {
    if (target && filterButton?.contains(target)) return;
    if (target && filterPopover?.contains(target)) return;
    filtersOpen = false;
  }

  if (createMenuOpen) {
    if (target && createMenuButton?.contains(target)) return;
    if (target && createMenuPopover?.contains(target)) return;
    createMenuOpen = false;
  }
}
```

- [ ] **Step 4: Оновити Escape handling**

У `onDrawerKeydown` перед editor close додати закриття floating menus:

```ts
if (filtersOpen) {
  event.preventDefault();
  filtersOpen = false;
  filterButton?.focus();
  return;
}

if (createMenuOpen) {
  event.preventDefault();
  createMenuOpen = false;
  createMenuButton?.focus();
  return;
}
```

Також додати global handler:

```ts
function onWindowKeydown(event: KeyboardEvent) {
  if (event.key !== "Escape") return;
  if (filtersOpen) {
    event.preventDefault();
    filtersOpen = false;
    filterButton?.focus();
    return;
  }
  if (createMenuOpen) {
    event.preventDefault();
    createMenuOpen = false;
    createMenuButton?.focus();
  }
}
```

- [ ] **Step 5: Додати smart create helpers**

Замінити `onCreateDraft`, `onSelectCreateKind`, `focusCreateButton` на:

```ts
$: selectedCreateKind = $documents.kindFilter;

$: createButtonKind = selectedCreateKind ?? null;

$: createButtonLabel = createButtonKind
  ? getDocumentCreateLabel(createButtonKind, $documents.activeTab)
  : "Створити ▾";

function onCreateDraft() {
  if (!createButtonKind) {
    createMenuOpen = !createMenuOpen;
    return;
  }
  void documents.create(createCounterpartyId || undefined, createButtonKind);
}

function onCreateMenuKind(kind: DocumentKind) {
  void documents.create(createCounterpartyId || undefined, kind);
  createMenuOpen = false;
}
```

У template (Task 3) кнопка Створити рендериться умовно: якщо `createButtonKind !== null` — показувати `AppIcon` з іконкою типу документа; якщо null — тільки текст `"Створити ▾"` без AppIcon (уникаємо потенційно відсутнього icon name).

- [ ] **Step 6: Синхронно закривати menus при недоступних станах**

Поруч з існуючим reactive block для `chainMenuOpen` додати:

```ts
$: if ($documents.kindFilter && createMenuOpen) {
  createMenuOpen = false;
}

$: if ($documents.loading) {
  filtersOpen = false;
  createMenuOpen = false;
}
```

- [ ] **Step 7: Запустити targeted test**

Run:

```bash
npm run test:frontend -- frontend/src/lib/screens/__tests__/DocumentsScreen.test.ts
```

Expected: still FAIL, бо markup/CSS ще старі.

---

### Task 3: Перебудувати DocumentsScreen markup

**Files:**
- Modify: `frontend/src/lib/screens/DocumentsScreen.svelte`

- [ ] **Step 1: Замінити window listeners**

Знайти `svelte:window` для click/key handlers біля drawer markup. Замінити click handler на `onWindowClick`; додати keydown, якщо його немає:

```svelte
<svelte:window on:click={onWindowClick} on:keydown={onWindowKeydown} />
```

Якщо вже є `on:keydown={onDrawerKeydown}`, не дублювати закриття drawer; лишити drawer-specific handler там, де він прив'язаний до drawer, а global keydown використовувати тільки для popovers.

- [ ] **Step 2: Видалити quick presets block**

Повністю видалити старий блок `documents-presets-row`, включно з label `DOCUMENTS_FILTER_COPY.presetsLabel`, loop `DOCUMENT_FILTER_PRESETS`, `data-testid={\`documents-preset-${preset.id}\`}` і click handler `documents.applyPreset(preset.id)`.

- [ ] **Step 3: Об'єднати kind chips, filter і create у toolbar**

Замінити старі blocks `documents-kind-chips`, `documents-filter-toolbar`, inline filter panel і `documents-create-bar` на:

```svelte
<div class="documents-toolbar" data-testid="documents-toolbar">
  <div class="documents-kind-chips" role="group" aria-label="Тип документа">
    {#each kindChips as chip}
      <button
        type="button"
        class="kind-chip"
        class:kind-chip-active={$documents.kindFilter === chip.value}
        on:click={() => documents.setKindFilter(chip.value)}
        disabled={$documents.loading}
      >
        {chip.label}
      </button>
    {/each}
  </div>

  <div class="documents-toolbar-actions">
    <div class="documents-toolbar-popover-anchor">
      <button
        bind:this={filterButton}
        class="btn-secondary"
        class:filter-popover-btn-active={filtersOpen}
        data-testid="documents-filter-button"
        type="button"
        aria-expanded={filtersOpen}
        aria-controls="documents-filter-popover"
        on:click={toggleFilters}
        disabled={$documents.loading}
      >
        <span>{filterButtonLabel}</span>
      </button>

      {#if filtersOpen}
        <div
          bind:this={filterPopover}
          id="documents-filter-popover"
          class="filter-popover"
          data-testid="documents-filter-panel"
          role="dialog"
          aria-label="Фільтр документів"
        >
          <fieldset class="filter-panel-section">
            <legend>{DOCUMENTS_FILTER_COPY.periodLabel}</legend>
            <div class="filter-panel-subpresets">
              <button type="button" class="kind-chip" on:click={() => onDateSubpreset('today')}>{DOCUMENTS_FILTER_COPY.periodSubpresets.today}</button>
              <button type="button" class="kind-chip" on:click={() => onDateSubpreset('week')}>{DOCUMENTS_FILTER_COPY.periodSubpresets.week}</button>
              <button type="button" class="kind-chip" on:click={() => onDateSubpreset('month')}>{DOCUMENTS_FILTER_COPY.periodSubpresets.month}</button>
              <button type="button" class="kind-chip" on:click={() => onDateSubpreset('quarter')}>{DOCUMENTS_FILTER_COPY.periodSubpresets.quarter}</button>
              <button type="button" class="kind-chip" on:click={() => onDateSubpreset('year')}>{DOCUMENTS_FILTER_COPY.periodSubpresets.year}</button>
            </div>
            <div class="filter-panel-grid-2">
              <label>{DOCUMENTS_FILTER_COPY.periodFrom}<input type="date" bind:value={panelDateFrom} /></label>
              <label>{DOCUMENTS_FILTER_COPY.periodTo}<input type="date" bind:value={panelDateTo} /></label>
            </div>
            {#if dateRangeError}
              <p class="filter-error" role="alert">{dateRangeError}</p>
            {/if}
          </fieldset>

          <fieldset class="filter-panel-section">
            <legend>{DOCUMENTS_FILTER_COPY.statusLabel}</legend>
            <div class="filter-panel-statuses">
              {#each DOCUMENT_STATUS_OPTIONS as opt}
                <label class="status-checkbox">
                  <input type="checkbox" value={opt.value}
                    checked={panelStatuses.includes(opt.value)}
                    on:change={() => toggleStatus(opt.value, !panelStatuses.includes(opt.value))} />
                  {opt.label}
                </label>
              {/each}
            </div>
          </fieldset>

          <fieldset class="filter-panel-section">
            <legend>{DOCUMENTS_FILTER_COPY.counterpartyLabel}</legend>
            <select
              bind:value={filterCounterpartyId}
              disabled={$documents.loading}
              data-testid="documents-counterparty-filter"
              aria-label="Фільтр за контрагентом"
            >
              <option value="">{DOCUMENTS_FILTER_COPY.counterpartyAll}</option>
              {#each $counterparties.screen?.items ?? [] as cp}
                <option value={cp.id}>{cp.name}</option>
              {/each}
            </select>
          </fieldset>

          <fieldset class="filter-panel-section">
            <legend>{DOCUMENTS_FILTER_COPY.amountLabel}</legend>
            <div class="filter-panel-grid-2">
              <label>{DOCUMENTS_FILTER_COPY.amountFrom}<input type="text" inputmode="decimal" bind:value={panelAmountMin} placeholder="0,00" /></label>
              <label>{DOCUMENTS_FILTER_COPY.amountTo}<input type="text" inputmode="decimal" bind:value={panelAmountMax} placeholder="0,00" /></label>
            </div>
            {#if amountRangeError}
              <p class="filter-error" role="alert">{amountRangeError}</p>
            {/if}
          </fieldset>

          <div class="documents-filter-actions">
            <button class="btn-ghost" type="button" on:click={resetPanelDraft} disabled={$documents.loading}>
              {DOCUMENTS_FILTER_COPY.reset}
            </button>
            <button class="btn-primary" type="button" on:click={applyPanel} disabled={$documents.loading || !!dateRangeError || !!amountRangeError}>
              {DOCUMENTS_FILTER_COPY.apply}
            </button>
          </div>
        </div>
      {/if}
    </div>

    {#if activeFilterCount > 0}
      <button
        class="btn-ghost"
        type="button"
        data-testid="documents-clear-filters"
        on:click={onClearAllFilters}
        disabled={$documents.loading}
      >
        {DOCUMENTS_FILTER_COPY.clearAll}
      </button>
    {/if}

    <div class="documents-toolbar-popover-anchor">
      <button
        bind:this={createMenuButton}
        class="btn-primary"
        data-testid="documents-create-button"
        type="button"
        disabled={$documents.loading}
        on:click={onCreateDraft}
        aria-expanded={createMenuOpen}
        aria-controls="documents-create-picker"
        aria-busy={$documents.loading ? "true" : "false"}
      >
        <AppIcon name={createButtonIcon} surface={true} />
        <span>{createButtonLabel}</span>
      </button>

      {#if createMenuOpen}
        <div
          bind:this={createMenuPopover}
          id="documents-create-picker"
          class="create-picker-popover"
          data-testid="documents-create-picker"
          role="menu"
          aria-label="Створити документ"
        >
          {#each DOCUMENT_KIND_OPTIONS as option}
            <button
              type="button"
              class="create-picker-item"
              data-testid={`documents-create-picker-${option.value}`}
              role="menuitem"
              on:click={() => onCreateMenuKind(option.value)}
            >
              <AppIcon name={documentKindMeta[option.value].icon} surface={true} />
              <span>{option.label}</span>
            </button>
          {/each}
        </div>
      {/if}
    </div>
  </div>
</div>
```

- [ ] **Step 4: Перевірити перенесені contents filter panel**

Всередині `.filter-popover` мають бути ті самі 4 секції, що були у старому inline panel: period/date inputs, statuses, counterparty select, amount range, а також footer buttons `resetPanelDraft` і `applyPanel`. Не лишати зовнішній inline wrapper `.documents-filter-panel`.

- [ ] **Step 5: Перевірити активні chips**

Залишити `documents-active-filters` одразу після toolbar:

```svelte
{#if activeFilterCount > 0}
  <div class="documents-active-filters" data-testid="documents-active-filters">
```

- [ ] **Step 6: Запустити targeted test**

Run:

```bash
npm run test:frontend -- frontend/src/lib/screens/__tests__/DocumentsScreen.test.ts
```

Expected: FAIL тільки на CSS contracts або дрібних selectors; core interactions мають уже проходити.

---

### Task 4: Оновити CSS під toolbar і popovers

**Files:**
- Modify: `frontend/src/styles/documents.css`

- [ ] **Step 1: Видалити старі selectors**

Прибрати blocks:

Selectors to remove: `.documents-create-bar`, `.documents-create-kind-chips`, `.documents-presets-row`, `.documents-presets-label`, `.documents-filter-toolbar`, and the old inline `.documents-filter-panel`.

Також прибрати responsive rules для `.documents-create-bar`, `.documents-create-kind-chips`, `.documents-filter-panel`.

- [ ] **Step 2: Додати toolbar CSS**

На початку файлу додати:

```css
.documents-toolbar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 12px;
  margin-top: 12px;
}

.documents-toolbar-actions {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 8px;
  margin-left: auto;
}

.documents-toolbar-popover-anchor {
  position: relative;
  display: inline-flex;
}

.filter-popover-btn-active {
  border-color: var(--acta-color-accent);
  background: #e8f0fe;
  color: var(--acta-color-accent-text);
}
```

- [ ] **Step 3: Додати popover CSS**

Додати:

```css
.filter-popover {
  position: absolute;
  top: calc(100% + 8px);
  right: 0;
  z-index: 30;
  display: grid;
  width: 340px;
  gap: 16px;
  padding: 16px;
  border: 1px solid var(--acta-color-border);
  border-radius: var(--acta-radius-2xl);
  background: var(--acta-color-bg-elevated);
  box-shadow: 0 18px 44px rgba(15, 23, 42, 0.18);
}

.create-picker-popover {
  position: absolute;
  top: calc(100% + 8px);
  right: 0;
  z-index: 30;
  display: grid;
  width: 200px;
  gap: 4px;
  padding: 6px;
  border: 1px solid var(--acta-color-border);
  border-radius: var(--acta-radius-xl);
  background: var(--acta-color-bg-elevated);
  box-shadow: 0 16px 36px rgba(15, 23, 42, 0.16);
}

.create-picker-item {
  display: flex;
  align-items: center;
  gap: 8px;
  min-height: 36px;
  width: 100%;
  padding: 0 10px;
  border: 0;
  border-radius: var(--acta-radius-md);
  background: transparent;
  color: var(--acta-color-text);
  font: inherit;
  font-size: 13px;
  font-weight: 600;
  text-align: left;
  cursor: pointer;
}

.create-picker-item:hover,
.create-picker-item:focus-visible {
  background: var(--acta-color-bg-subtle);
  outline: none;
}
```

- [ ] **Step 4: Оновити responsive CSS**

У `@media (max-width: 980px)` лишити nav/kind scroll та bulk rules, додати:

```css
.documents-toolbar {
  align-items: stretch;
  gap: 8px;
}

.documents-toolbar-actions {
  flex-shrink: 0;
}
```

У `@media (max-width: 720px)` додати:

```css
.documents-toolbar {
  flex-direction: column;
  align-items: stretch;
}

.documents-toolbar-actions {
  justify-content: stretch;
  margin-left: 0;
}

.documents-toolbar-actions .btn-secondary,
.documents-toolbar-actions .btn-primary {
  width: 100%;
}

.filter-popover {
  right: auto;
  left: 0;
  width: min(340px, calc(100vw - 32px));
  padding: 12px;
}

.create-picker-popover {
  right: auto;
  left: 0;
  width: min(220px, calc(100vw - 32px));
}
```

- [ ] **Step 5: Просканувати старі class names**

Run:

```bash
rg -n "documents-presets-row|documents-presets-label|documents-create-kind-chips|documents-create-bar|documents-filter-toolbar|\\.documents-filter-panel" frontend/src
```

Expected: no matches.

- [ ] **Step 6: Запустити targeted test**

Run:

```bash
npm run test:frontend -- frontend/src/lib/screens/__tests__/DocumentsScreen.test.ts
```

Expected: PASS.

---

### Task 5: Повна frontend verification

**Files:**
- No source changes expected unless verification finds issues.

- [ ] **Step 1: Typecheck Svelte/TS**

Run:

```bash
npm run check
```

Expected: PASS. Якщо є Svelte warning про a11y/roles для popover buttons, виправити markup у `DocumentsScreen.svelte`.

- [ ] **Step 2: Run full frontend tests**

Run:

```bash
npm run test:frontend
```

Expected: PASS.

- [ ] **Step 3: Optional build check**

Run:

```bash
npm run build
```

Expected: PASS.

- [ ] **Step 5: Git review**

Run:

```bash
git diff -- frontend/src/lib/screens/DocumentsScreen.svelte frontend/src/styles/documents.css frontend/src/lib/screens/__tests__/DocumentsScreen.test.ts
```

Expected:
- quick presets row removed from template;
- create kind chips removed from template;
- smart Create uses `$documents.kindFilter`;
- filter panel is a floating `.filter-popover`;
- tests cover create picker and filter popover behavior.

---

## Self-Review

- Spec coverage: covered removed quick presets, removed create kind chips/createKind, 2-row header, smart Create, filter popover, click-outside/Escape, CSS, and test updates.
- Intentional non-change: `DOCUMENT_FILTER_PRESETS` and `documents.applyPreset` stay in config/store because active chip removal uses `applyPreset("all")` and the spec allows config to remain.
- Risk: exact icon name for generic create button must be checked against `AppIcon.svelte`; fallback is an existing document kind icon if `plus` is unavailable.
