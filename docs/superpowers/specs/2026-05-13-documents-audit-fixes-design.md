# Documents — виправлення з візуального аудиту

**Date:** 2026-05-13
**Scope:** 9 фіксів зі сторінки Документи: 3 баги, 4 структурні зміни, 2 косметичні
**Status:** Approved, ready for implementation
**Audit artifacts:** `audit-documents-1440.png`, `audit-documents-1024.png`, `audit-documents-filters-open.png`, `audit-documents-drawer.png`

---

## Огляд

Аудит сторінки `DocumentsScreen` виявив 9 проблем різного рівня. Спека групує їх у 3 секції за характером:

| Секція | Фікси | Складність |
|---|---|---|
| **A. Bugs** | A1 responsive 1024 checkbox, A2 формат дати, A5 розузгодженість статусів | низька–середня |
| **B. Structural** | B3 split-button create, B4 conditional bulk-bar, B7 grouping кнопок drawer, B9 direction toggle | середня |
| **C. Polish** | C6 дубль типу в рядку, C8 перевантаження синім | низька |

Що НЕ входить (винесено в окрему майбутню розмову):
- Уніфікація 6 горизонтальних смуг фільтрації над списком (концептуальна архітектура — `nav-tabs` + `kind-chips` + `presets` + `filter-toolbar`).
- Зміна порядку та сутності direction filter (tabs Вихідні/Вхідні vs kind chips).
- Перегляд presets row як цілого (зокрема перетворення на dropdown).
- Зміни Tauri-команд, store API, моделей даних (крім `clearSelection` у docstore — див. B4).

---

## Секція A — Bug fixes

### A1. Responsive 1024 — checkbox-орфан

**Проблема:** `frontend/src/styles/documents.css:788–810` має `@media (max-width: 1080px)`, який перемикає `.doc-row` у `flex-direction: column`. На viewport ~1024 px checkbox опиняється самостійним блоком над карткою документа (видно у `audit-documents-1024.png`).

**Рішення:**
1. Підняти breakpoint з `1080px` до `720px` (узгоджено з іншими responsive-правилами файлу — `documents-create-bar`).
2. Прибрати `flex-direction: column` з `.doc-row` (зовнішній flex). Залишити wrap тільки для `.doc-row-body` (внутрішній flex з title і meta).

**Зміна (`documents.css:788`):**
```css
/* БУЛО */
@media (max-width: 1080px) {
  .chain-summary,
  .editor-item-head,
  .editor-item,
  .existing-pdf-replace {
    grid-template-columns: 1fr;
  }
  .editor-items-summary,
  .editor-actions {
    justify-content: flex-start;
  }
  .doc-row,
  .doc-row-body {
    flex-direction: column;
    align-items: flex-start;
  }
  .doc-row-meta {
    justify-content: flex-start;
  }
}

/* СТАЛО */
@media (max-width: 1080px) {
  .chain-summary,
  .editor-item-head,
  .editor-item,
  .existing-pdf-replace {
    grid-template-columns: 1fr;
  }
  .editor-items-summary,
  .editor-actions {
    justify-content: flex-start;
  }
}

@media (max-width: 720px) {
  .doc-row-body {
    flex-direction: column;
    align-items: flex-start;
  }
  .doc-row-meta {
    justify-content: flex-start;
  }
}
```

`.doc-row` залишається `flex-row` із checkbox зліва і `.doc-row-open` як основним блоком. На 720 px виставлений вже існуючий `.documents-create-bar { grid-template-columns: 1fr }` — узгоджується.

---

### A2. Формат дати — гіпотеза `lang="uk"` з fallback

**Проблема:** native `<input type="date">` показує `mm/dd/yyyy` у date-picker і у заповненому стані, бо webview успадковує локаль ОС/Chrome (US за замовчуванням). Стандарт проєкту — `dd.mm.yyyy` (`.claude/lessons.md` — `%d.%m.%Y`).

**Гіпотеза (Крок 1):** `frontend/index.html` — додати `lang="uk"` на корінь:
```html
<!DOCTYPE html>
<html lang="uk">
```

WebView2/Chrome **зазвичай** використовує `lang` атрибут на `<html>` для визначення формату date-picker. Але поведінка не гарантована і може залежати від версії Chromium/WebView, локалі ОС, і user accept-language preferences. На Windows WebView2 інколи фолбекає на системний format-pattern незалежно від `lang`.

**Fallback (Крок 2, якщо Крок 1 не спрацював на цільовій ОС):**
- Варіант A: явно ставити `lang="uk"` на самих `<input type="date">` (вибірково, у date-полях DocumentsScreen).
- Варіант B: ввести display-layer — поряд із прихованим `<input type="date">` показувати форматований span з `Intl.DateTimeFormat("uk-UA")` для відображення, а picker викликати програмно. Це збільшує scope і виходить за межі цієї спеки — окрема задача.

**Acceptance:** вручну на цільовій ОС (Windows + WebView2). Відкрити Documents → Фільтр → date input має показати `дд.мм.рррр` placeholder і вибраний день у форматі `13.05.2026`. Якщо ні — створити окрему задачу на fallback Варіант A/B.

**Тести:** jsdom (vitest) не рендерить native date picker, тому існуючі тести не зачепить.

---

### A5. Розузгодженість статусів — drawer vs список

**Проблема:** У списку рядок INV-2026-0042 має chip «Виставлено», у drawer тієї ж картки — «Очікує». Перевірка коду:
- Список: `item.statusLabel` з `DocumentRowDto` (рядок 784) — поточний статус документа.
- Drawer (`DocumentsScreen.svelte:873`): `getCurrentChainStatus()` повертає `chain.steps[steps.length - 1].status` — статус ОСТАННЬОГО кроку у ланцюгу документів (може бути інший документ: act → invoice → payment).

Це не баг — різні семантики. Але користувач бачить плутанину, бо chip має однаковий вигляд.

**Рішення:** показати у drawer header **дві окремі візуально-різні plaque-и**:

```svelte
<!-- БУЛО — рядок 869–874 -->
<div class="editor-header-meta">
  <span class="doc-kind-badge">
    <AppIcon name={getEditorKindIcon($documents.editor.form.kind)} size={14} />
    <span>{getDocumentKindLabel($documents.editor.form.kind)}</span>
  </span>
  <span class="doc-status-chip">{getCurrentChainStatus()}</span>
</div>

<!-- СТАЛО -->
<div class="editor-header-meta">
  <span class="doc-kind-badge">
    <AppIcon name={getEditorKindIcon($documents.editor.form.kind)} size={14} />
    <span>{getDocumentKindLabel($documents.editor.form.kind)}</span>
  </span>
  <span class="doc-status-chip" data-testid="documents-drawer-document-status">
    {getDocumentStatusLabel()}
  </span>
  {#if hasChainBeyondSelf()}
    <span class="chain-stage-chip" data-testid="documents-drawer-chain-status">
      <AppIcon name="git-branch" size={12} />
      <span>Ланцюг: {getCurrentChainStatus()}</span>
    </span>
  {/if}
</div>
```

**Нові функції в `<script>`:**

```ts
function getDocumentStatusLabel(): string {
  // Знайти поточний документ у chain.steps за form.id і повернути UI label.
  // Fallback на "Чернетка" — для щойно створеного документа, який ще не у ланцюгу.
  const id = $documents.editor?.form.id ?? "";
  const steps = $documents.chain?.steps ?? [];
  const own = steps.find((s) => s.documentId === id);
  return own?.statusLabel ?? "Чернетка";
}

function hasChainBeyondSelf(): boolean {
  // load_document_chain ЗАВЖДИ повертає 3 кроки (invoice/act/waybill), включно
  // з virtual exists=false. Тому не достатньо `documentId !== id` —
  // virtual кроки мають documentId=null і завжди дають true.
  // Реальні related docs = exists=true AND інший documentId.
  const id = $documents.editor?.form.id ?? "";
  const steps = $documents.chain?.steps ?? [];
  return steps.some((s) => s.exists && s.documentId && s.documentId !== id);
}
```

**DTO зміна (обов'язкова):** Поточний `ChainStepDto` (`src/tauri_api/documents/dto.rs:95–101`) має поля `doc_type`, `doc_number`, `amount_str`, `status`, `exists` — без id, і `status` зберігає **код** (`issued`/`draft`/…), а не UI label. Додаємо два поля:

```rust
// src/tauri_api/documents/dto.rs
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ChainStepDto {
    pub document_id: Option<String>,  // ← новий, None для exists=false
    pub doc_type: String,
    pub doc_number: String,
    pub amount_str: String,
    pub status: String,                // код (issued/draft/…) — як зараз
    pub status_label: String,          // ← новий, UI label (узгоджено з list.statusLabel)
    pub exists: bool,
}
```

Заповнення у `load_document_chain` (`api.rs:700+`):
- `document_id`: `Some(doc.id.to_string())` для існуючих кроків (`exists: true`), `None` для віртуальних (`exists: false` — placeholder для ще не створених документів типу invoice/act/waybill).
- `status_label`: викликати `.status.label()` (як вже робиться у `list_documents` — `api.rs:881`, `:899`, `:917`) для existing, для virtual — порожній рядок або «Не створено». Конкретне значення virtual випадку визначити під час реалізації, узгодити з UI.

TypeScript дзеркало у `frontend/src/lib/types.ts`:
```ts
export interface ChainStep {
  documentId: string | null;
  docType: string;
  docNumber: string;
  amountStr: string;
  status: string;
  statusLabel: string;
  exists: boolean;
}
```

**Ripple у fixtures і тестах** — додати поля у наступних файлах (інакше TS strict моде зламається):
- `frontend/src/lib/browser-fixtures.ts` — кожен `ChainStep` mock.
- `frontend/src/lib/stores/__tests__/documents-store.test.ts` — chain steps fixtures.
- `frontend/src/lib/stores/__tests__/shell-documents.test.ts` — chain steps fixtures (якщо є).
- `frontend/src/lib/screens/__tests__/DocumentsScreen.test.ts` — drawer chain fixtures.
- Rust integration тести у `tests/db_integration.rs` і `tests/tauri_vertical_slice.rs` — якщо створюють `ChainStepDto` напряму.

`cargo sqlx prepare` не потрібен: `ChainStepDto` будується програмно у Rust, не через `query_as!`.

**CSS (`documents.css`):**
```css
.chain-stage-chip {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  min-height: var(--acta-density-chip-h);
  padding: 0 10px;
  border-radius: 999px;
  font-size: 11px;
  font-weight: 600;
  background: var(--acta-color-bg-subtle);
  color: var(--acta-color-text-muted);
  border: 1px dashed var(--acta-color-border);
}
```

Дашед-border + приглушені кольори чітко відрізняють chain-stage від doc-status (solid).

---

## Секція B — Structural changes

### B3. Split-button «Створити»

**Проблема:** `documents-create-bar` має нижній ряд з 3 chips (Рахунок / Акт / Накладна) + primary button. Ці chips візуально ідентичні `kind-chips` фільтра вище — користувачі плутаються. Окрім того, ряд з'їдає цілу вертикаль для рідкої дії.

**Рішення:** замінити на split-button — primary CTA з default «Створити <останній обраний>» + caret-dropdown для зміни типу.

**HTML (заміна 635–662 у `DocumentsScreen.svelte`):**
```svelte
<div class="documents-create-bar" data-testid="documents-create-strip">
  <div class="split-button" class:split-button-open={createMenuOpen}>
    <button
      bind:this={createButton}
      class="split-button-primary btn-primary"
      data-testid="documents-create-button"
      type="button"
      disabled={$documents.loading}
      on:click={onCreateDraft}
      aria-busy={$documents.loading ? "true" : "false"}
    >
      <AppIcon name={documentKindMeta[createKind].icon} surface={true} />
      <span>{getDocumentCreateLabel(createKind, $documents.activeTab)}</span>
    </button>
    <button
      class="split-button-caret btn-primary"
      type="button"
      aria-haspopup="menu"
      aria-expanded={createMenuOpen}
      aria-label="Вибрати інший тип документа"
      data-testid="documents-create-menu-trigger"
      disabled={$documents.loading}
      on:click|stopPropagation={toggleCreateMenu}
    >▾</button>
    <div
      bind:this={createMenuPopover}
      class="split-button-menu"
      role="menu"
      hidden={!createMenuOpen}
    >
      {#each DOCUMENT_KIND_OPTIONS as option}
        <button
          role="menuitem"
          type="button"
          class="split-button-menu-item"
          data-testid={`documents-create-menu-${option.value}`}
          disabled={$documents.loading}
          on:click={() => onPickCreateKind(option.value)}
        >
          <AppIcon name={documentKindMeta[option.value].icon} size={16} />
          <span>{option.label}</span>
        </button>
      {/each}
    </div>
  </div>
</div>
```

**Скрипт (нові функції):**
```ts
let createMenuOpen = false;
let createMenuRoot: HTMLElement | null = null;  // bind на <div class="split-button"> — обгортка обох кнопок і menu

function toggleCreateMenu() { createMenuOpen = !createMenuOpen; }
function closeCreateMenu() { createMenuOpen = false; }

function onPickCreateKind(kind: DocumentKind) {
  createKind = kind;
  try { localStorage.setItem("acta:documents:lastCreateKind", kind); } catch {}
  closeCreateMenu();
  void documents.create(createCounterpartyId || undefined, kind);
}

// На init — підняти останній kind з localStorage
let createKind: DocumentKind = (() => {
  try {
    const stored = localStorage.getItem("acta:documents:lastCreateKind");
    if (stored && DOCUMENT_KIND_OPTIONS.some((o) => o.value === stored)) {
      return stored as DocumentKind;
    }
  } catch {}
  return "act";
})();
```

**HTML коректива:** обгортка `<div class="split-button">` має `bind:this={createMenuRoot}`, primary button і caret button — всередині цього root. Menu також всередині root. Це означає, що `createMenuRoot.contains(target)` істинне для будь-якого кліку **в межах split-button** (primary, caret, menu items).

**Закрити меню при click на primary:** primary CTA повинен **завжди** закривати меню (бо одразу створює документ — нема сенсу залишати дропдаун відкритим). Додаємо у `onCreateDraft`:

```ts
function onCreateDraft() {
  closeCreateMenu();
  void documents.create(createCounterpartyId || undefined, createKind);
}
```

**Window click listener** — переробити поточний `onWindowClickForChainMenu` у уніфікований handler:
```ts
function onWindowClickGlobalMenus(event: MouseEvent) {
  const target = event.target as Node | null;

  if (chainMenuOpen) {
    if (target && chainMenuButton?.contains(target)) return;
    if (target && chainMenuPopover?.contains(target)) return;
    closeChainMenu();
  }

  if (createMenuOpen) {
    // Клік у межах split-button (primary/caret/menu) НЕ закриває meny через handler —
    // primary та menu item самі закривають через closeCreateMenu().
    // Клік ПОЗА split-button → закрити.
    if (target && createMenuRoot?.contains(target)) return;
    closeCreateMenu();
  }
}
```

І замінити `<svelte:window on:click={onWindowClickForChainMenu}>` на `on:click={onWindowClickGlobalMenus}`.

**CSS (`documents.css`):**
```css
.split-button {
  position: relative;
  display: inline-flex;
  isolation: isolate;
}
.split-button-primary {
  border-top-right-radius: 0;
  border-bottom-right-radius: 0;
}
.split-button-caret {
  border-top-left-radius: 0;
  border-bottom-left-radius: 0;
  padding: 0 12px;
  border-left: 1px solid color-mix(in srgb, white 22%, transparent);
  font-size: 12px;
  min-width: 36px;
}
.split-button-menu {
  position: absolute;
  top: calc(100% + 6px);
  right: 0;
  min-width: 220px;
  display: flex;
  flex-direction: column;
  padding: 6px;
  border-radius: var(--acta-radius-2xl);
  border: 1px solid var(--acta-color-border);
  background: var(--acta-color-bg-elevated);
  box-shadow: 0 12px 32px -12px color-mix(in srgb, #0b1220 28%, transparent);
  z-index: 60;
}
.split-button-menu[hidden] { display: none; }
.split-button-menu-item {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  padding: 10px 12px;
  border: 0;
  border-radius: var(--acta-radius-lg);
  background: transparent;
  color: var(--acta-color-text);
  text-align: left;
  cursor: pointer;
  font: inherit;
  font-weight: 500;
}
.split-button-menu-item:hover:not(:disabled) {
  background: color-mix(in srgb, var(--acta-color-accent-soft) 60%, var(--acta-color-bg-elevated));
}
```

**Видалити:**
- HTML: блок `.documents-create-kind-chips` із усіма `kind-chip` всередині `.documents-create-bar`.
- CSS: `.documents-create-kind-chips` (рядки 9–14 у `documents.css`).
- Скрипт: функція `onSelectCreateKind` (заміщена `onPickCreateKind`).

**Залишається без змін:**
- `onCreateDraft()` — primary click тригерить існуючу логіку з поточним `createKind`.
- `createCounterpartyId` логіка — без змін.

---

### B4. Conditional bulk-bar

**Проблема:** `bulk-actions` (рядки 666–697) видимий завжди, з 3 кнопками + checkbox. При 0 виборах — кнопки disabled, а ряд займає ~50 px вертикалі. Це шум.

**Рішення:**
1. Виносимо «Вибрати все» checkbox у заголовок списку — він видимий, але мінімальний.
2. Кнопки масових дій («Оновити статус», «Видалити») рендеримо тільки коли `selectedIds.length > 0`, з `transition:slide`.

**HTML — нова структура:**

```svelte
{#if ($documents.list?.items.length ?? 0) > 0}
  <div class="documents-list-header" data-testid="documents-list-header">
    <label class="bulk-select-all">
      <input
        type="checkbox"
        checked={
          ($documents.list?.items.length ?? 0) > 0 &&
          ($documents.list?.items ?? []).every((item) => $documents.selectedIds.includes(item.id))
        }
        on:click|stopPropagation={onToggleSelectAll}
      />
      <span>Вибрати все ({$documents.list?.items.length ?? 0})</span>
    </label>
    {#if $documents.selectedIds.length > 0}
      <span class="bulk-count" data-testid="documents-bulk-count">
        Вибрано: {$documents.selectedIds.length}
      </span>
    {/if}
  </div>

  {#if $documents.selectedIds.length > 0}
    <div
      class="bulk-actions"
      data-testid="documents-bulk-actions"
      transition:slide={{ duration: 140 }}
    >
      <button class="btn-secondary" on:click={onBulkAdvanceStatus} disabled={$documents.loading}>
        Оновити статус
      </button>
      <button class="btn-danger" on:click={onBulkDelete} disabled={$documents.loading}>
        Видалити
      </button>
      <button class="btn-ghost" on:click={() => documents.clearSelection()}>
        Скасувати вибір
      </button>
    </div>
  {/if}
{/if}
```

**Store (`stores/documents.ts`):**
Метод `clearSelection()` вже існує (`stores/documents.ts:266–268`) — використати його напряму через `documents.clearSelection()`. Жодних змін у store не потрібно.

**CSS зміни:**
```css
.documents-list-header {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-top: 12px;
}
.bulk-count {
  font-size: 11px;
  font-weight: 600;
  color: var(--acta-color-accent-text);
  padding: 2px 8px;
  background: var(--acta-color-accent-soft);
  border-radius: 999px;
}
.bulk-actions {
  display: flex;
  gap: 8px;
  margin-top: 8px;
  align-items: center;
}
/* Видалити .bulk-actions-idle, .bulk-select-all (overridden), і пов'язані mobile rules для bulk-actions-idle. */
```

**Імпорт:** додати у `<script>`:
```ts
import { slide } from "svelte/transition";
```

---

### B7. Drawer header grouping

**Проблема:** 5 кнопок у одному flex-ряду: `Зберегти / Дії далі ▾ / PDF / Видалити / Закрити`. Деструктивні дії візуально не відокремлені; «PDF» текст-only зливається; «Дії далі» — vague назва.

**Рішення:** 3 групи з flex spacers; перейменування лейблів; виокремлення «Наступний крок» як chip у header-meta.

**HTML (заміна 866–946):**
```svelte
<div class="editor-header">
  <div>
    <div class="editor-header-meta">
      <span class="doc-kind-badge">
        <AppIcon name={getEditorKindIcon($documents.editor.form.kind)} size={14} />
        <span>{getDocumentKindLabel($documents.editor.form.kind)}</span>
      </span>
      <span class="doc-status-chip" data-testid="documents-drawer-document-status">
        {getDocumentStatusLabel()}
      </span>
      {#if hasChainBeyondSelf()}
        <span class="chain-stage-chip" data-testid="documents-drawer-chain-status">
          <AppIcon name="git-branch" size={12} />
          <span>Ланцюг: {getCurrentChainStatus()}</span>
        </span>
      {/if}
      <button
        class="next-status-chip"
        type="button"
        data-testid="documents-next-status"
        on:click={() => void documents.advanceStatus()}
        disabled={$documents.loading}
      >
        Наступний крок →
      </button>
    </div>
    <h3 id="documents-drawer-title" tabindex="-1">{$documents.editor.form.title}</h3>
    <p>{$documents.editor.form.counterpartyName}</p>
  </div>

  <div class="editor-actions">
    <div class="editor-actions-group editor-actions-primary">
      <button
        class="btn-primary"
        on:click={() => documents.save()}
        disabled={$documents.loading}
        aria-busy={$documents.loading ? "true" : "false"}
      >
        Зберегти
      </button>
    </div>

    <div class="editor-actions-group editor-actions-secondary">
      <div class="chain-menu" class:chain-menu-open={chainMenuOpen}>
        <button
          bind:this={chainMenuButton}
          class="btn-secondary chain-menu-trigger"
          type="button"
          aria-haspopup="menu"
          aria-expanded={chainMenuOpen}
          on:click|stopPropagation={toggleChainMenu}
          disabled={$documents.loading}
        >
          <span>{DOCUMENTS_COPY.chainMenuLabel}</span>
          <span aria-hidden="true" class="chain-menu-caret">▾</span>
        </button>
        <div
          bind:this={chainMenuPopover}
          class="chain-menu-popover"
          role="menu"
          hidden={!chainMenuOpen}
        >
          {#each getDocumentChainTargets($documents.editor.form.kind) as targetKind}
            <button
              role="menuitem"
              type="button"
              class="chain-menu-item"
              data-testid={`documents-chain-create-${targetKind}`}
              on:click={() => onChainMenuCreateChain(targetKind)}
              disabled={$documents.loading}
            >
              <AppIcon name={documentKindMeta[targetKind].icon} size={16} />
              <span>Створити {documentKindMeta[targetKind].actionLabel}</span>
            </button>
          {/each}
        </div>
      </div>

      {#if supportsDocumentPdfGeneration($documents.editor.form.kind)}
        <button class="btn-ghost" on:click={() => documents.generatePdf()} disabled={$documents.loading}>
          <AppIcon name="file-text" size={14} />
          <span>{DOCUMENTS_COPY.generatePdfLabel}</span>
        </button>
      {/if}
    </div>

    <div class="editor-actions-group editor-actions-destructive">
      <button class="btn-danger" on:click={onDeleteCurrent} disabled={$documents.loading} data-testid="documents-delete-current-btn">
        Видалити
      </button>
      <button class="btn-ghost" on:click={requestCloseDrawer} disabled={$documents.loading}>
        Закрити
      </button>
    </div>
  </div>
</div>
```

**Зміни `ui.ts`:** додати в `DOCUMENTS_COPY`:
```ts
chainMenuLabel: "Створити пов'язаний",
generatePdfLabel: "Згенерувати PDF",
```

**Видалити функцію `onChainMenuAdvanceStatus`** (замінена `next-status-chip` у header-meta). Видалити пункт «Наступний статус» зі `.chain-menu-popover` (тільки створення пов'язаних документів залишається).

**CSS (`documents.css`):**
```css
.editor-actions {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
  align-items: center;
}
.editor-actions-group {
  display: flex;
  gap: 8px;
  align-items: center;
}
.editor-actions-secondary {
  margin-left: auto;
}
.editor-actions-destructive {
  padding-left: 12px;
  margin-left: 4px;
  border-left: 1px solid var(--acta-color-border);
}
.next-status-chip {
  display: inline-flex;
  align-items: center;
  min-height: var(--acta-density-chip-h);
  padding: 0 10px;
  border-radius: 999px;
  border: 1px solid color-mix(in srgb, var(--acta-color-accent) 30%, var(--acta-color-border));
  background: var(--acta-color-bg-elevated);
  color: var(--acta-color-accent-text);
  font-size: 11px;
  font-weight: 600;
  cursor: pointer;
}
.next-status-chip:hover:not(:disabled) {
  background: var(--acta-color-accent-soft);
}
.next-status-chip:disabled {
  cursor: default;
  opacity: 0.5;
}
```

**Видалити з `documents.css`:** `.editor-actions-close` (більше не використовується).

---

### B9. Direction toggle

**Проблема:** `editor-direction-fieldset` (рядки 971–995) — фієлдсет з legend і двома radio, span на всю ширину editor-grid. Для бінарного вибору забагато простору і важкий стиль.

**Рішення:** inline segmented control біля поля Дата.

**HTML (заміна 971–995):**

`editor-grid` тепер містить Номер, Дата, Направлення в один ряд, Примітки span на всю ширину:

```svelte
<div class="editor-grid">
  <label>
    Номер
    <input value={$documents.editor.form.number} on:input={onEditorNumberChange} disabled={$documents.loading} />
  </label>
  <label class="editor-date-field">
    Дата
    <input
      type="date"
      value={$documents.editor.form.date}
      on:input={onEditorDateChange}
      disabled={$documents.loading}
    />
  </label>
  <div class="editor-direction-field">
    <span class="editor-direction-label">Напрямок</span>
    <div class="editor-direction-toggle" role="radiogroup" aria-label="Напрямок документа">
      {#each DOCUMENT_DIRECTION_OPTIONS as opt}
        <button
          role="radio"
          type="button"
          aria-checked={$documents.editor?.form.direction === opt.value}
          class="editor-direction-option"
          class:editor-direction-active={$documents.editor?.form.direction === opt.value}
          on:click={() => documents.updateFormField("direction", opt.value)}
          disabled={$documents.loading}
        >
          <span aria-hidden="true">{opt.value === "outgoing" ? "↑" : "↓"}</span>
          <span>{opt.label}</span>
        </button>
      {/each}
    </div>
  </div>
  <label class="editor-grid-span">
    Примітки
    <textarea
      rows="3"
      value={$documents.editor.form.notes}
      on:input={onEditorNotesChange}
      disabled={$documents.loading}
    ></textarea>
  </label>
</div>
```

**CSS:**
```css
.editor-direction-field {
  display: grid;
  gap: 8px;
}
.editor-direction-label {
  font-size: 11px;
  color: var(--acta-color-text-muted);
}
.editor-direction-toggle {
  display: inline-flex;
  border: 1px solid var(--acta-color-border);
  border-radius: var(--acta-radius-lg);
  overflow: hidden;
  background: var(--acta-color-bg-elevated);
}
.editor-direction-option {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 6px 12px;
  border: 0;
  background: transparent;
  color: var(--acta-color-text-muted);
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
}
.editor-direction-option + .editor-direction-option {
  border-left: 1px solid var(--acta-color-border);
}
.editor-direction-active {
  background: var(--acta-color-accent-soft);
  color: var(--acta-color-accent-text);
}
.editor-direction-option:disabled {
  cursor: default;
  opacity: 0.5;
}
```

**Видалити з scoped style блоку (`DocumentsScreen.svelte:1196–1214`):** `.editor-direction-fieldset`, `.editor-direction-fieldset legend`, `.editor-direction-option` (стара inline-flex з radio).

---

## Секція C — Polish

### C6. Прибрати дубль типу у рядку

**Проблема:** У `.doc-row-meta` (рядки 780–784) є `doc-kind-badge` з іконкою та лейблом типу, хоча та сама іконка вже є у `.doc-row-title` зліва.

**Зміна (`DocumentsScreen.svelte:777–788`):**
```svelte
<!-- БУЛО -->
<div class="doc-row-meta">
  <span>{item.date}</span>
  <span class="money-value" data-negative={isFormattedMoneyNegative(item.amountStr)}>{item.amountStr}</span>
  <span class="doc-kind-badge">
    <AppIcon name={resolveDocumentKindMeta(item.kind).icon} size={14} />
    <span>{getDocumentKindLabel(item.kind)}</span>
  </span>
  <span class="doc-status-chip">{item.statusLabel}</span>
  <span class="doc-direction-badge" data-direction={item.direction}>
    {DOCUMENT_DIRECTION_LABELS[item.direction] ?? item.direction}
  </span>
</div>

<!-- СТАЛО -->
<div class="doc-row-meta">
  <span>{item.date}</span>
  <span class="money-value" data-negative={isFormattedMoneyNegative(item.amountStr)}>{item.amountStr}</span>
  <span class="doc-status-chip">{item.statusLabel}</span>
  <span class="doc-direction-badge" data-direction={item.direction}>
    {DOCUMENT_DIRECTION_LABELS[item.direction] ?? item.direction}
  </span>
</div>
```

`.doc-kind-badge` CSS залишається — він ще використовується у drawer header (B7).

---

### C8. Перевантаження синім

**Проблема:** `kind-chip-active` (filter chips, presets, kind filter) — повний `accent` фон з білим текстом. Активні елементи фільтра конкурують з primary CTA «Створити». Око тоне в синьому.

**Рішення:** softer active state для chips. Повний синій залишається тільки за primary buttons (Створити, Зберегти).

**Зміна (`DocumentsScreen.svelte` scoped style, 1177–1181):**
```css
/* БУЛО */
.kind-chip-active {
  background: var(--acta-color-accent);
  border-color: var(--acta-color-accent);
  color: #fff;
}

/* СТАЛО */
.kind-chip-active {
  background: color-mix(in srgb, var(--acta-color-accent-soft) 60%, var(--acta-color-bg-elevated));
  border-color: color-mix(in srgb, var(--acta-color-accent) 40%, var(--acta-color-border));
  color: var(--acta-color-accent-text);
  font-weight: 600;
}
```

Контраст AA перевіряється: `--acta-color-accent-text` на `accent-soft` фоні має ≥4.5:1 для small text. У `.status-checkbox:has(input:checked)` (documents.css:169) вже використовується аналогічна комбінація — паттерн уніфікований.

Tab underline (`.nav-tab[aria-selected="true"]`) залишається синім — це єдиний сильний акцент на рівні навігації, не конкурує з CTA.

---

## Файли, що змінюються

| Файл | Секції | Зміна |
|---|---|---|
| `frontend/index.html` | A2 | `<html lang="uk">` |
| `frontend/src/styles/documents.css` | A1, B3, B4, B7, B9, C8 | Responsive breakpoint, split-button styles, list-header, action groups, direction-toggle, kind-chip token softer, видалити direction-fieldset/create-kind-chips/editor-actions-close |
| `frontend/src/lib/screens/DocumentsScreen.svelte` | A5, B3, B4, B7, B9, C6, C8 | Split-button розмітка, list-header + conditional bulk-bar, header groups + next-status-chip + два status chips, direction-toggle inline, прибрати duplicate kind badge у списку, scoped style оновити (.kind-chip-active soft) |
<!-- B4: stores/documents.ts — clearSelection() вже існує, нічого додавати -->
| `frontend/src/lib/config/ui.ts` | B7 | `DOCUMENTS_COPY`: `chainMenuLabel: "Створити пов'язаний"`, `generatePdfLabel: "Згенерувати PDF"` |
| `frontend/src/lib/screens/__tests__/DocumentsScreen.test.ts` | B3, B4, B7 | Split-button menu test, conditional bulk-bar test, два status chips test |
| `src/tauri_api/documents/dto.rs` | A5 | `ChainStepDto.document_id: Option<String>` |
| `src/tauri_api/documents/api.rs` | A5 | `load_document_chain` — заповнити `document_id` (Some для existing, None для virtual) |
| `frontend/src/lib/types.ts` | A5 | `ChainStep.documentId: string \| null` |

---

## Що НЕ змінюється

- Логіка фільтрів, presets row, `nav-tabs`, `kind-chips` (filter) — поза цією спекою. 6 вертикальних смуг — тема окремої майбутньої розмови.
- Tabs Всі/Вихідні/Вхідні vs kind-chips розузгодженість — не чіпаємо.
- Drawer animation, backdrop, dirty-banner, confirm-delete-banner.
- Active filter chips і filter panel content.
- Empty state, skeleton, error banners.
- Tauri commands (крім додавання `document_id` + `status_label` у `ChainStepDto`).
- Store API (`clearSelection` вже є).
- `presets-label`, `documents-active-filters`, `documents-filter-toolbar` — без змін.

---

## Тести

**Frontend (vitest, jsdom):**
- `DocumentsScreen.test.ts`:
  - Split-button: primary click → `documents.create` викликаний з `createKind`; menu item click → updates `createKind` і викликає create; localStorage `acta:documents:lastCreateKind` persist.
  - Conditional bulk-bar: при `selectedIds = []` — `bulk-actions` НЕ в DOM; при `selectedIds = [...]` — є + Cancel button чистить.
  - Drawer header: при наявному chain з 2+ кроків — обидва chips (`documents-drawer-document-status`, `documents-drawer-chain-status`) видимі; при відсутньому — тільки document-status.
  - Direction toggle: click на «Вхідний» → `documents.updateFormField("direction", "incoming")`; aria-checked правильний.
  - Дубль badge у списку прибрано: `documents-list` НЕ містить `doc-kind-badge` всередині `.doc-row-meta`.

**Візуальна перевірка (вручну):**
- 1440×900: drawer header — 3 групи з spacer, не wrap.
- 1024×800: список — checkbox inline зі своєю карткою (не орфан).
- 720×600: meta-чіпи рядків переходять у column; bulk-bar conditional працює.
- Dark mode: `kind-chip-active`, `chain-stage-chip`, `next-status-chip`, `editor-direction-active`, `.split-button-menu` — токени `--acta-*` коректні.
- Filter date input і drawer date input показують `dd.mm.yyyy` після `lang="uk"`.

---

## Порядок реалізації

Рекомендую 4 PR (можна об'єднати в один, якщо зручніше):

1. **PR-1: Bugs A1 + A2** — малий, незалежний. Один файл `documents.css` + один `index.html`. Низький ризик.
2. **PR-2: Polish C8** — один CSS блок у scoped `<style>` DocumentsScreen. Малий, можна mergeity швидко.
3. **PR-3: B3 split-button** — новий компонент-блок, видаляє `create-kind-chips`, додає dropdown логіку і persist. Самодостатнє.
4. **PR-4: A5 + B4 + B7 + B9 + C6** — drawer полишення + list polish. Найбільший, але внутрішньо узгоджений.

Якщо є залежність по DTO (`ChainStepDto.document_id`), вона входить у PR-4 і потребує `cargo sqlx prepare`.

---

## Acceptance criteria

- ✅ На 1024×800 viewport чекбокс рядка inline з карткою (не орфан).
- ✅ (manual) Усі date input показують `dd.mm.yyyy` на цільовій ОС після `lang="uk"`. Якщо ні — створено follow-up задачу на fallback.
- ✅ Drawer header показує 2 chips (документ статус + ланцюг) тільки коли є інший exists=true документ у ланцюгу. Для самотнього документа — тільки 1 chip (статус самого документа).
- ✅ Статус документа і chain-stage показуються через `statusLabel` (UI text), не raw status code.
- ✅ Split-button: primary CTA — створює default kind; dropdown — змінює kind + одразу створює; вибір persist у localStorage.
- ✅ Bulk-bar не видимий при 0 виборах; з'являється з slide-transition при першому виборі.
- ✅ Drawer header кнопки в 3 групи з візуальним розділенням деструктивних.
- ✅ «Дії далі ▾» → «Створити пов'язаний ▾»; «PDF» → «Згенерувати PDF» з іконкою.
- ✅ «Наступний крок» доступний як chip у header-meta (не через dropdown).
- ✅ Direction toggle inline (не fieldset).
- ✅ У `.doc-row-meta` нема дубль `doc-kind-badge`.
- ✅ Active chips (kind, preset) — soft accent (не повний синій).
- ✅ Всі існуючі тести `DocumentsScreen.test.ts` проходять; додано тести під нові data-testid.
- ✅ `cargo build --tests` + `npm run check` clean.
