# Design Foundation — Wave 1: Tokens + Shell + Components

**Date:** 2026-05-05  
**Branch:** codex/p1-ui-polish-followup  
**Scope:** Design Foundation (Wave 1 of full handoff implementation)  
**Source:** `docs/Acta-handoff/acta/project/`

---

## Мета

Імплементувати design foundation з handoff-специфікації (Acta Design Tokens v2.0) у наш Svelte/Tauri проект. Хвиля 1 охоплює: токени, shell (sidebar + topbar), і 8 reusable Svelte-компонентів.

**Межа Wave 1:** компоненти створюються і перевіряються "smoke usage" — тобто використовуються в shell, palette та dashboard де це природньо (Button, Card, KPI, StatusBadge). Повне впровадження Table, Modal, FormField, CommandBar у DocumentsScreen, PaymentsScreen, форми документів — **наступні хвилі**. Screens/forms Wave 1 не торкається.

---

## Рішення

| Питання | Рішення |
|---------|---------|
| Naming токенів | `--acta-*` prefix, 1:1 з handoff CSS |
| Orphaned токени | Видаляємо (`--bg-glass`, `--page-bg` gradient, `--money-*`, etc.) |
| Міграція | Big-bang: один PR — нові токени + оновлення всіх CSS файлів |
| Компоненти | Окремі `.svelte` файли у `frontend/src/lib/components/` |
| Shell | Повний перероблення HTML + CSS в `App.svelte` |

---

## 1. Design Tokens

### Файл

Замінити `frontend/src/lib/styles/tokens.css` вмістом `docs/Acta-handoff/acta/project/handoff/design-tokens.css` із адаптацією:
- Залишити `--acta-*` prefix як є
- Додати shimmer animation (`.sk`) — потрібна для SkeletonRow/SkeletonCard
- Додати `--acta-color-bg-overlay: rgba(0,0,0,0.5)` для Modal backdrop
- Видалити всі старі токени без `--acta-` prefix

### Ключові зміни значень

**Кольори (light):**
- Page bg: `#f4f1ea` warm paper → `#F7F8FA` cool grey
- Sidebar bg: `#e9e2d5` → `#FFFFFF` (білий!)
- Elevated bg: `#fffdf8` → `#FFFFFF`
- Accent: `#315ee8` → `#3D75F4`
- Border: `rgba(58,66,74,0.12)` → `#E1E5EB` (solid)
- Success: `#1eb16e` → `#0F8B4F`
- Warning: `#c77a1c` → `#B5651A`
- Danger: `#c0433b` → `#C0322B`

**Кольори (dark):**
- Page bg: `#22262f` → `#0F1419` (темний navy)
- Sidebar bg: `#1b1f27` → `#0B0E13`
- Elevated: `#2c313b` → `#171C24`

**Нові токени (яких у нас не було):**
- `--acta-shadow-card`, `--acta-shadow-card-hover`, `--acta-shadow-popover`, `--acta-shadow-modal`
- `--acta-motion-fast: 120ms`, `--acta-motion-base: 180ms`, `--acta-motion-slow: 240ms`
- Всі `--acta-density-*` (table-row 36px, button-h 32px, input-h 36px, topbar-h 56px, sidebar-w 240px, sidebar-rail-w 3px)
- `--acta-radius-sm: 3px` (не 4px як у нас), `--acta-radius-md: 4px`, `--acta-radius-xl: 8px`, `--acta-radius-2xl: 10px`
- `.acta-num` / `.acta-mono` utility classes

**Radii (повне скорочення):**

| | Старий | Новий |
|---|---|---|
| sm | 4px | 3px |
| md | 6px | 4px |
| lg | 8px | 6px |
| xl | 12px | 8px |
| 2xl | 16px | 10px |

### CSS-міграція у всіх файлах

Всі CSS-файли проекту оновлюються: `--bg` → `--acta-color-bg-page`, `--text` → `--acta-color-text`, тощо.

Повна таблиця маппінгу старих → нових імен:

| Старе | Нове |
|-------|------|
| `--bg` | `--acta-color-bg-page` |
| `--bg-elevated` | `--acta-color-bg-elevated` |
| `--bg-subtle` | `--acta-color-bg-subtle` |
| `--bg-hover` | `--acta-color-bg-hover` |
| `--bg-sidebar` | `--acta-color-bg-sidebar` |
| `--bg-stripe` | `--acta-color-bg-stripe` |
| `--border` | `--acta-color-border` |
| `--border-strong` | `--acta-color-border-strong` |
| `--border-hairline` | `--acta-color-border` |
| `--text` | `--acta-color-text` |
| `--text-muted` | `--acta-color-text-muted` |
| `--text-faint` | `--acta-color-text-faint` |
| `--accent` | `--acta-color-accent` |
| `--accent-hover` | `--acta-color-accent-hover` |
| `--accent-soft` | `--acta-color-accent-soft` |
| `--accent-text` | `--acta-color-accent-text` |
| `--success` | `--acta-color-success` |
| `--success-soft` | `--acta-color-success-soft` |
| `--warning` | `--acta-color-warning` |
| `--warning-soft` | `--acta-color-warning-soft` |
| `--danger` | `--acta-color-danger` |
| `--danger-soft` | `--acta-color-danger-soft` |
| `--info` | `--acta-color-info` |
| `--info-soft` | `--acta-color-info-soft` |
| `--font-sans` | `--acta-font-sans` |
| `--font-mono` | `--acta-font-mono` |
| `--font-body` | → `14px` (inline або через `var(--acta-text-body)`) |
| `--font-sm` | → `13px` |
| `--font-xs` | → `11px` |
| `--space-1..8` | `--acta-space-1..10` |
| `--radius-sm` | `--acta-radius-sm` |
| `--radius-md` | `--acta-radius-md` |
| `--radius-lg` | `--acta-radius-lg` |
| `--radius-xl` | `--acta-radius-xl` |
| `--control-height` | `--acta-density-input-h` |

**Токени без еквівалента (видаляємо, CSS де вони — переписуємо вручну):**
- `--bg-glass`, `--bg-overlay`, `--bg-card`, `--bg-card-strong`, `--bg-field`
- `--page-bg` (gradient) → замінити на `--acta-color-bg-page`
- `--money-positive`, `--money-negative`
- `--control-bg`, `--control-border`, `--control-shadow`, `--button-shadow`
- `--floating-shadow`, `--surface-blur`

**CSS файли що потребують оновлення:**
- `frontend/src/lib/styles/tokens.css` — повна заміна
- `frontend/src/styles.css` — shell стилі
- `frontend/src/styles/dashboard.css`
- `frontend/src/styles/documents.css`
- `frontend/src/styles/counterparties.css`
- `frontend/src/styles/tasks.css`
- `frontend/src/styles/reports.css`
- `frontend/src/styles/settings.css`
- `frontend/src/App.svelte` (inline стилі → нові токени)

---

## 2. Shell — App.svelte повний перероблення

### Загальна структура

```
<div class="app-shell">  ← display:grid; grid-template-columns: 240px 1fr; height:100vh
  <aside class="sidebar">
    ...
  </aside>
  <main class="main">
    <header class="topbar">...</header>
    <div class="shell-progress">...</div>  ← тільки якщо busy
    <div class="screen-outlet">
      {поточний екран}
    </div>
  </main>
  {#if palette.open}
    <div class="palette-backdrop">...</div>
    <section class="palette">...</section>
  {/if}
</div>
```

### 2.1 Sidebar

**Розміри:** width `240px` (--acta-density-sidebar-w), bg `--acta-color-bg-sidebar`, border-right `1px solid --acta-color-border`, padding `16px 12px`.

**Структура (зверху вниз):**

```
Brand block + Company switcher (об'єднано в один елемент, 56px зона, border-bottom 1px)
  У shell.jsx brand block і company switcher — один button. Залишаємо єдиний функціональний button.
  padding: 8px 10px; radius 4px; margin-bottom: 16px
  bg: transparent (hover: --acta-color-bg-elevated + border)
  gap: 10px
  [logo 26×26 accent radius 6px | company name 13px/500 + sub 10.5px/faint | chevron 13px]

Nav (flex-col, gap: 2px)
  кожен item:
    position: relative; height: 36px; padding: 0 12px; radius 4px
    font: 14px/400; color: --acta-color-text-muted
    hover: bg --acta-color-bg-hover; color --acta-color-text
    active:
      bg: --acta-color-accent-soft; color: --acta-color-accent-text; font-weight: 500
      padding-left: 15px (компенсація rail)
      <span class="nav-rail"> position:absolute; left:0; top:0; bottom:0; width:3px
        bg: --acta-color-accent; border-radius: 0 2px 2px 0
    icon: 18px, color faint (hover: muted, active: accent)
    badge: 17px, min-width 18px, bg subtle, color muted, 10.5px/600/mono

Saved filters
  header: 10px/600/faint/uppercase/letter-spacing 1.4 + "+" icon button (18px)
  items: height 24px, padding 4px 10px, radius 3px
    star icon 11px | label flex:1 truncate | count 10.5px/faint/mono
    hover: bg --acta-color-bg-hover; color --acta-color-text

flex: 1 (spacer)

Settings nav item (same structure як nav item)
  margin-bottom: 4px

User pill
  border-top: 1px solid --acta-color-border; padding: 10px 8px; margin-top: 6px
  avatar 32×32 [radius 4px, bg accent-soft, color accent-text, 12px/600, initials]
  name 12.5px/500 truncate | sub 10.5px/faint truncate
  more button (··· IconButton)
```

**⚠️ Зміна:** Search trigger прибирається з sidebar — переноситься у topbar center.  
**⚠️ Зміна:** Saved filters залишаються в sidebar (є в поточному коді).

### 2.2 Topbar

**Розміри:** height `56px`, bg `--acta-color-bg-elevated`, border-bottom `1px solid --acta-color-border`, padding `0 24px`.

**Layout:** `display: grid; grid-template-columns: 1fr minmax(280px, 480px) 1fr; align-items: center; gap: 24px`

**LEFT** — `flex-col; gap: 2px`
- title: `font: 18px/24px; font-weight: 600; color: --acta-color-text; truncate`
- subtitle: `13px; color: --acta-color-text-muted; truncate` (company name)

**CENTER** — search trigger button
- height: 36px; bg: `--acta-color-bg-subtle`; border: `1px solid --acta-color-border`; radius: 4px; padding: `0 12px`
- hover: `border-color: --acta-color-border-strong`
- content: `[Search icon 16px/faint] [placeholder "Пошук документа…" 13px/faint flex:1] [kbd ⌘K 11px/mono]`
- click → відкриває palette (той самий механізм що зараз)

**RIGHT** — `flex; justify-content: flex-end; gap: 12px`
1. Theme toggle: IconButton 36×36, Moon/Sun 20px
2. Bell: IconButton 36×36 + notification badge (danger bg, 2px white border, 11px/600/mono). У Wave 1 — декоративний (badge не відображається, count = 0). Логіка нотифікацій — окрема хвиля.
3. User avatar: 32×32, radius 4px, accent-soft bg, accent-text color, 12px/600, initials з `shellState.chrome.userInitials`

**⚠️ Зміна:** topbar bg змінюється з `--page-bg` (gradient) → `--acta-color-bg-elevated` (білий/темний).  
**⚠️ Зміна:** додається user avatar у topbar (зараз тільки у sidebar footer).  
**⚠️ Зміна:** додається search center column (зараз немає).

### 2.3 Command Palette — рестайл + групи

**⚠️ Логіка залишається в `paletteStore` і `PaletteItemDto` (types.ts) без змін.** Групи — виключно presentation layer в `App.svelte` через `groupLabel(kind)`. Жодних змін у DTO, store або backend.

**Групи виводяться з `kind` field:**
- `kind === 'navigate'` → "Перехід"
- `kind.startsWith('create_')` → "Створити"
- `kind === 'open_document'` → "Документи"
- `kind === 'open_counterparty'` → "Контрагенти"

**Вигляд:**
```
Backdrop: fixed inset-0; rgba(10,10,12,0.35); blur(4px)

Container: width 620px; max-height 480px
  bg: --acta-color-bg-elevated; border; border-radius 12px; shadow modal
  flex-col; overflow hidden

Header row (border-bottom):
  padding: 14px 16px; gap: 10px
  [Search icon 16px/faint] [input flex:1; 14px; no-border; bg transparent] [kbd "esc"]

Results (flex:1, overflow-y auto, padding: 6px 0):
  per group:
    group label: 8px 16px 4px; 10px/600/faint; uppercase; letter-spacing 1.2
    items: padding 8px 16px; flex; gap 10px
      [icon 14px] [label 13px flex:1] [meta 11.5px/faint/mono] [kbd badges] [↵ icon if active]
      active bg: --acta-color-bg-subtle

Footer (border-top, bg subtle):
  padding: 8px 16px; 11px/faint
  [↑↓ навігація] [↵ вибрати] [spacer] [N результатів]
```

---

## 3. Component Library

### Нові файли

| Файл | Опис |
|------|------|
| `Button.svelte` | 4 variants × 3 sizes |
| `Card.svelte` | з named slots header/body/footer |
| `StatusBadge.svelte` | 5 tones + dot variant |
| `KPI.svelte` | wraps Card, mono numbers, delta arrow |
| `FormField.svelte` | label + slot (input) + error/help |
| `Table.svelte` | zebra, sort, bulk selection, empty state |
| `Modal.svelte` | overlay + header + scrollable body + footer |
| `CommandBar.svelte` | search + filters + saved views + primary slot |

### Порядок реалізації

1. Button (потрібен усюди)
2. StatusBadge (потрібен для Table)
3. Card (потрібен для KPI + Dashboard)
4. KPI (Dashboard)
5. Modal (форми)
6. FormField (форми)
7. Table (Documents, Payments, Counterparties)
8. CommandBar (Documents, Payments)

### Button.svelte — повна специфікація

```
Props:
  variant: 'primary' | 'secondary' | 'ghost' | 'danger' = 'secondary'
  size: 'default' | 'sm' | 'icon' = 'default'
  + $$restProps (disabled, aria-label, type, on:click тощо)
Slot: default

HTML: <button class="btn {variant} {size}" {...$$restProps}><slot/></button>

CSS:
.btn {
  display: inline-flex; align-items: center; gap: 8px;
  border-radius: var(--acta-radius-md);  /* 4px */
  font-family: var(--acta-font-sans); font-weight: 500;
  border: 1px solid transparent;
  cursor: pointer; user-select: none; white-space: nowrap;
  transition: background var(--acta-motion-fast), border-color var(--acta-motion-fast), filter var(--acta-motion-fast);
}
.btn:disabled { opacity: 0.5; cursor: not-allowed; }
.btn:focus-visible { outline: 2px solid var(--acta-color-accent); outline-offset: 2px; }

/* sizes */
.default { height: 32px; padding: 0 14px; font-size: 13px; }
.sm      { height: 26px; padding: 0 10px; font-size: 12px; gap: 6px; }
.icon    { width: 32px; height: 32px; padding: 0; justify-content: center; }

/* variants */
.primary  { background: var(--acta-color-accent); color: #fff; }
.primary:not(:disabled):hover  { background: var(--acta-color-accent-hover); }
.primary:not(:disabled):active { filter: brightness(0.94); }

.secondary { background: var(--acta-color-bg-elevated); color: var(--acta-color-text);
             border-color: var(--acta-color-border-strong); }
.secondary:not(:disabled):hover  { background: var(--acta-color-bg-hover); }
.secondary:not(:disabled):active { background: var(--acta-color-bg-subtle); }

.ghost { background: transparent; color: var(--acta-color-text-muted); }
.ghost:not(:disabled):hover { background: var(--acta-color-bg-hover); color: var(--acta-color-text); }

.danger { background: var(--acta-color-danger); color: #fff; }
.danger:not(:disabled):hover { filter: brightness(1.08); }
```

### Card.svelte — повна специфікація

```
Props: compact: boolean = false
Named slots: header, default (body), footer

HTML:
<div class="card" class:compact>
  {#if $$slots.header}
    <header class="card-header"><slot name="header"/></header>
  {/if}
  <div class="card-body"><slot/></div>
  {#if $$slots.footer}
    <footer class="card-footer"><slot name="footer"/></footer>
  {/if}
</div>

CSS:
.card {
  background: var(--acta-color-bg-elevated);
  border: 1px solid var(--acta-color-border);
  border-radius: var(--acta-radius-xl);  /* 8px */
  box-shadow: var(--acta-shadow-card);
  padding: 20px 24px;
}
.compact { padding: 16px 20px; }
.card-header {
  display: flex; align-items: center; justify-content: space-between; gap: 16px;
  padding-bottom: 16px; margin-bottom: 16px;
  border-bottom: 1px solid var(--acta-color-border);
}
.card-footer {
  display: flex; align-items: center; justify-content: space-between;
  padding-top: 16px; margin-top: 16px;
  border-top: 1px solid var(--acta-color-border);
  font-size: 13px; color: var(--acta-color-text-muted);
}
```

### StatusBadge.svelte — повна специфікація

```
Props:
  tone: 'success' | 'warning' | 'danger' | 'info' | 'muted'
  dot: boolean = false
  size: 'default' | 'lg' = 'default'
Slot: default (label text)

HTML:
<span class="badge {tone} {size}">
  {#if dot}<span class="dot"></span>{/if}
  <slot/>
</span>

CSS:
.badge {
  display: inline-flex; align-items: center; gap: 6px;
  border-radius: var(--acta-radius-sm);  /* 3px */
  font-family: var(--acta-font-sans); font-weight: 500; white-space: nowrap;
}
.default { height: 20px; padding: 0 8px; font-size: 11px; text-transform: uppercase; letter-spacing: 0.5px; }
.lg      { height: 24px; padding: 0 10px; font-size: 12px; }
.dot { width: 6px; height: 6px; border-radius: 50%; background: currentColor; flex-shrink: 0; }

.success { background: var(--acta-color-success-soft); color: var(--acta-color-success); }
.warning { background: var(--acta-color-warning-soft); color: var(--acta-color-warning); }
.danger  { background: var(--acta-color-danger-soft);  color: var(--acta-color-danger); }
.info    { background: var(--acta-color-info-soft);    color: var(--acta-color-info); }
.muted   { background: var(--acta-color-bg-subtle);    color: var(--acta-color-text-muted); }
```

### KPI.svelte — повна специфікація

```
Props:
  caption: string
  value: number
  currency: 'UAH' | 'count' = 'UAH'
  delta: number | undefined = undefined  (% зміна, може бути від'ємне)
  direction: 'positive-up' | 'positive-down' = 'positive-up'
  context: string | undefined = undefined

Логіка delta:
  tone = delta == null || delta === 0 ? 'neutral'
       : (delta > 0 ? (direction==='positive-up' ? 'good' : 'bad')
                    : (direction==='positive-up' ? 'bad'  : 'good'))

HTML (обгорнути у <Card>):
<Card>
  <div class="caption">{caption}</div>
  <div class="row">
    <div class="value">
      {formatValue(value, currency)}
      {#if currency === 'UAH'}<span class="unit">грн</span>{/if}
    </div>
    {#if delta != null}
      <div class="delta {tone}">
        <!-- Arrow icon: TrendingUp / TrendingDown / Minus -->
        <span>{delta > 0 ? '+' : ''}{delta.toFixed(1)}%</span>
      </div>
    {/if}
  </div>
  {#if context}<div class="context">{context}</div>{/if}
</Card>

CSS: caption 11px/600/uppercase/faint; value 28px/600/mono/tabular-nums;
unit 18px/400/muted; delta 13px/500/mono; context 12px/muted
```

### FormField.svelte — повна специфікація

```
Props:
  label: string
  required: boolean = false
  error: string | undefined = undefined
  helpText: string | undefined = undefined
Slot: default (сам input/select/textarea)

HTML:
<div class="field">
  <label class="label">
    {label}{#if required}<span class="required">*</span>{/if}
  </label>
  <slot/>
  {#if error}
    <p class="error-text">⚠ {error}</p>
  {:else if helpText}
    <p class="help-text">{helpText}</p>
  {/if}
</div>

CSS input (глобальний клас .acta-input):
height: 36px; padding: 0 12px; width: 100%
bg: --acta-color-bg-elevated
border: 1px solid --acta-color-border (hover: border-strong, focus: accent)
border-radius: 4px; font: 14px; color: text
focus: outline none + box-shadow 0 0 0 3px --acta-color-accent-soft
error: border-color danger + box-shadow 0 0 0 3px danger-soft
disabled: bg subtle; opacity 0.6; cursor not-allowed
```

### Table.svelte — повна специфікація

```
// Svelte 4 не підтримує generic component syntax — типи визначаємо у окремому .ts файлі
// frontend/src/lib/components/table-types.ts:
export interface TableColumn {
  id: string;
  header: string;
  accessor: (row: Record<string, unknown>) => unknown;
  align?: 'left' | 'right' | 'center';
  width?: string;
  sortable?: boolean;
}

// Table.svelte props (через export let):
Props:
  columns: TableColumn[]
  rows: Record<string, unknown>[]
  getRowId: (row: Record<string, unknown>) => string
  selectedIds: string[] = []
  onSelectChange: ((ids: string[]) => void) | undefined = undefined
  onRowClick: ((row: Record<string, unknown>) => void) | undefined = undefined
  sortBy: string | undefined = undefined
  sortDir: 'asc' | 'desc' = 'asc'
  onSortChange: ((col: string, dir: 'asc' | 'desc') => void) | undefined = undefined
  emptyTitle: string = 'Немає даних'
  emptySubtitle: string = ''

// Caller передає rows як Record<string,unknown>[] через as-cast або прямо з store

HTML: <table> з <thead sticky> та <tbody>
  header: 32px; bg subtle; border-bottom border-strong
  row: 36px; zebra (парні: bg-elevated, непарні: bg-stripe)
  row:hover → bg-hover; selected → bg accent-soft
  cell: padding 8px 12px; 13px; border-bottom border
  right-align cells: font-mono + tabular-nums

Bulk action banner (animated slide-in):
  visible коли selectedIds.length > 0
  height 40px; bg accent-soft; sticky top; padding 8px 16px
  [Вибрано N] [actions...] [×]
  animation: transform translateY(-100%) → translateY(0); 180ms

Empty state (коли rows.length === 0):
  centered; padding 64px 24px
  icon Inbox 32px faint | title 15/600 | subtitle 13/muted | primary button
```

### Modal.svelte — повна специфікація

```
Props:
  open: boolean = false
  title: string
  maxWidth: number = 720
Named slots: default (body), footer
Events: on:close

HTML (коли open):
<div class="modal-backdrop" on:click={close} role="presentation">
  <div class="modal-container" on:click|stopPropagation
       style="max-width: {maxWidth}px"
       role="dialog" aria-modal="true" aria-labelledby="modal-title">
    <header class="modal-header">
      <h2 id="modal-title">{title}</h2>
      <button class="modal-close" on:click={close} aria-label="Закрити">✕</button>
    </header>
    <div class="modal-body"><slot/></div>
    {#if $$slots.footer}
      <footer class="modal-footer"><slot name="footer"/></footer>
    {/if}
  </div>
</div>

CSS:
backdrop: fixed inset-0; rgba(0,0,0,0.5); z-index 50; display flex; align-items center; justify-content center
  backdrop-filter: blur(4px)
  animation: fade-in 180ms

container: width 90vw; max-height 90vh; flex-col; overflow hidden
  bg: --acta-color-bg-elevated; border-radius 10px; shadow modal
  animation: scale(0.96) opacity 0 → scale(1) opacity 1; 180ms cubic-bezier(0.2,0,0,1)

modal-header: height 56px; padding 20px 24px; flex; space-between; border-bottom; 18px/600
modal-body: flex:1; overflow-y auto; padding 24px
modal-footer: padding 16px 24px; border-top; flex; justify-end; gap 8px

Keyboard: Esc → on:close
Focus trap: перший focusable element отримує focus при відкритті
```

### CommandBar.svelte — повна специфікація

```
Props:
  searchValue: string = ''
  filters?: Array<{ id: string; label: string; count?: number; active?: boolean }>
  savedViews?: Array<{ id: string; label: string; count: number; active?: boolean }>
  primaryLabel?: string
Named slots: primary (кнопка Створити)
Events: on:search(value), on:filterChange(id), on:viewChange(id)

HTML:
<div class="commandbar">
  {#if savedViews?.length}
    <div class="views">
      {#each savedViews as v}
        <button class="view-pill" class:active={v.active} on:click={...}>
          {v.label} <span class="view-count">{v.count}</span>
        </button>
      {/each}
    </div>
  {/if}
  <input class="commandbar-search acta-input" value={searchValue} on:input={...}
         placeholder="Пошук…" />
  {#each filters ?? [] as f}
    <button class="filter-btn" class:active={f.active} on:click={...}>
      {f.label} {#if f.active && f.count}<span class="filter-count">({f.count})</span>{/if}
    </button>
  {/each}
  <div class="commandbar-spacer"/>
  <slot name="primary"/>
</div>

CSS:
.commandbar: flex row; gap 8px; padding 12px 0; align-items center
.commandbar-search: width 280px; flex-shrink 0
.commandbar-spacer: flex 1
.view-pill: height 28px; padding 0 12px; border-radius 999px; 12px/500
  inactive: bg subtle, color muted, hover: bg-hover
  active: bg accent, color white
.filter-btn: secondary button style, 32px height, sm size
  active: bg accent-soft, border-color accent
```

---

## 4. Що залишається незмінним (Wave 1)

- Вся бізнес-логіка у stores (`dashboard`, `documents`, `payments`, etc.)
- Tauri API calls (`lib/api.ts`, `lib/browser-api.ts`)
- Всі типи (`lib/types.ts`)
- Screen компоненти (DashboardScreen, DocumentsScreen тощо) — оновлюються лише в частині використання нових компонентів, загальна структура залишається
- Тести stores — не чіпаємо
- `AppIcon.svelte`, `SkeletonRow.svelte`, `SkeletonCard.svelte` — рестайлюємо CSS але логіку не міняємо

---

## 5. Fonts

Обидва шрифти використовуються як **локальні assets**, закомічені у репозиторій. Google Fonts як runtime dependency не використовується (Tauri-аплікація може працювати offline).

**Дії:**
- Завантажити і закомітити у `frontend/public/fonts/`:
  - `JetBrainsSans-Regular.woff2`
  - `JetBrainsSans-SemiBold.woff2`
  - `JetBrainsMono-Regular.woff2`
  - `JetBrainsMono-Medium.woff2`
- Підключити через `@font-face` у `tokens.css` з `src: url('/fonts/...')` (Vite serve public/)

---

## 6. Файли що змінюються / створюються

### Нові файли
- `frontend/src/lib/components/Button.svelte`
- `frontend/src/lib/components/Card.svelte`
- `frontend/src/lib/components/StatusBadge.svelte`
- `frontend/src/lib/components/KPI.svelte`
- `frontend/src/lib/components/FormField.svelte`
- `frontend/src/lib/components/Table.svelte`
- `frontend/src/lib/components/Modal.svelte`
- `frontend/src/lib/components/CommandBar.svelte`
- `frontend/public/fonts/JetBrainsSans-Regular.woff2`
- `frontend/public/fonts/JetBrainsSans-SemiBold.woff2`

### Змінюються повністю
- `frontend/src/lib/styles/tokens.css` — повна заміна
- `frontend/src/App.svelte` — HTML + CSS повна заміна, логіка залишається
- `frontend/src/styles.css` — shell стилі з новими токенами

### Змінюються частково (токен-маппінг)
- `frontend/src/styles/dashboard.css`
- `frontend/src/styles/documents.css`
- `frontend/src/styles/counterparties.css`
- `frontend/src/styles/tasks.css`
- `frontend/src/styles/reports.css`
- `frontend/src/styles/settings.css`

### Stores — мінімальні зміни
- `frontend/src/lib/types.ts` → `PaletteItemDto` вже має поле `kind: string` (`"navigate"`, `"open_document"`, `"create_document_draft"`, тощо)
- Групи у palette виводяться **в frontend** з `kind`, без змін backend/Rust:
  ```ts
  function groupLabel(kind: string): string {
    if (kind === 'navigate') return 'Перехід';
    if (kind.startsWith('create_')) return 'Створити';
    if (kind === 'open_document') return 'Документи';
    if (kind === 'open_counterparty') return 'Контрагенти';
    return 'Інше';
  }
  ```

---

## Критерії завершення Wave 1

- [ ] Всі CSS файли використовують `--acta-*` токени, жодних старих `--bg`, `--text` тощо
- [ ] Sidebar: 240px, білий bg, active rail 3px, user pill з border-top
- [ ] Topbar: 3-column grid, search center, user avatar right, white bg
- [ ] Command palette: групований список + footer
- [ ] Шрифт JetBrains Sans завантажений і рендериться
- [ ] 8 компонентів імплементовано; Button/Card/KPI/StatusBadge — smoke usage у shell/dashboard; Table/Modal/FormField/CommandBar — створені але не впроваджені у screens (Wave 2+)
- [ ] Темна тема коректно перемикається з новими токенами
- [ ] `npm run check` (svelte-check) проходить без помилок
- [ ] `npm run build` проходить без помилок
