# Documents Screen Header Redesign

**Date:** 2026-05-14
**Status:** Approved

## Problem

The Documents screen currently has 4–5 rows above the document list, making the header cluttered and confusing:

1. Nav tabs (Всі / Вихідні / Вхідні)
2. Kind filter chips (Всі / Акти / Рахунки / Накладні)
3. **Quick preset row** — "Швидкі: Усі / Чернетки / Неоплачені / Прострочені / Цього місяця"
4. Filter toolbar — "Фільтр" button
5. **Create bar** — kind chips (Рахунок / Акт / Накладна) + "Створити акт" button

Problems:
- Quick presets duplicate functionality already available in the filter panel and are visually noisy
- Create bar's kind chips are redundant with the filter chips one row above — users must maintain two separate "type" selections
- The filter panel expands inline, pushing the document list down abruptly
- Overall: 4 rows of controls before the user sees a single document

## Design Decisions

### Layout: 2 rows (Variant A)

**Row 1 — Nav tabs:**
```
[Всі]  Вихідні  Вхідні
```

**Row 2 — Kind chips + Filter + Create (single toolbar):**
```
[Всі]  Акти  Рахунки  Накладні        [⚙ Фільтр]  [+ Створити ▾]
```

Kind chips are left-aligned, buttons are right-aligned with a spacer in between.

### Removed elements

- **Quick presets row** (`documents-presets-row`, `DOCUMENT_FILTER_PRESETS` loop, `presetsLabel`) — removed entirely from the template. The same date/status combinations are available inside the filter panel.
- **Create bar kind chips** (`documents-create-kind-chips`, `DOCUMENT_KIND_OPTIONS` loop, `createKind` state, `onSelectCreateKind`) — removed. Type selection is now handled by the Create button's popover.

### Create button: smart behavior

The Create button reads `$documents.kindFilter` to determine its behavior:

| `kindFilter` | Button label | On click |
|---|---|---|
| `null` (Всі) | `+ Створити ▾` | Opens a small inline popover with 3 options |
| `"act"` | `📋 Створити акт` | Calls `documents.create(undefined, "act")` directly |
| `"invoice"` | `📄 Створити рахунок` | Calls `documents.create(undefined, "invoice")` directly |
| `"waybill"` | `📦 Створити накладну` | Calls `documents.create(undefined, "waybill")` directly |

The kind picker popover (shown when `kindFilter = null`) is a minimal dropdown below the Create button:
- Three menu items: Рахунок / Акт / Накладна
- Each item calls `documents.create(undefined, kind)` and closes the popover
- Closes on click outside or Escape
- Implemented with the same click-outside pattern as `chainMenuOpen`

The `createKind` local variable and `onSelectCreateKind` function are removed entirely.

### Filter: popover (Variant A)

The filter panel moves from an inline-expanding block to a floating popover anchored below the "⚙ Фільтр" button.

**Popover behavior:**
- Opens when user clicks "⚙ Фільтр"; button gets active styling (`background: #e8f0fe`, border accent color)
- Closes on click outside or Escape (same pattern as chain menu)
- Does **not** push the document list down — the list stays in place
- Width: `340px` fixed, right-aligned to the button
- Border-radius: `var(--acta-radius-2xl)`, white background, shadow

**Popover contents (unchanged functionality, new container):**
- ПЕРІОД — quick subpresets chips + date range inputs
- СТАТУС — checkboxes styled as chips
- КОНТРАГЕНТ — `<select>`
- СУМА, ГРН — min/max inputs
- Footer: Скинути + Застосувати buttons

**Active filter chips** — the `documents-active-filters` row below the toolbar is kept. It shows removable chips for each active filter dimension.

## Component State Changes

### Variables removed
- `createKind: DocumentKind` — no longer needed
- `createButton: HTMLButtonElement | null` — no longer needed (and `focusCreateButton` function)

### Variables added
- `createMenuOpen: boolean` — controls the Create kind picker popover
- `createMenuButton: HTMLButtonElement | null` — ref for focus return
- `createMenuPopover: HTMLElement | null` — ref for click-outside detection

### Variables renamed/repurposed
- `filtersOpen` → stays, now controls a popover instead of an inline panel
- `filterButton: HTMLButtonElement | null` — new ref, needed for click-outside detection

### Click-outside handling
Both `createMenuOpen` and `filtersOpen` use the existing `onWindowClickForChainMenu` pattern — a single `svelte:window on:click` handler that checks if the click target is inside the button or popover refs.

The existing `onWindowClickForChainMenu` is renamed to `onWindowClick` and handles all three menus (chain, filter, create picker).

## CSS Changes

### Removed classes
- `.documents-presets-row`
- `.documents-presets-label`
- `.documents-create-kind-chips`
- `.documents-create-bar` — replaced by `.documents-toolbar`

### New classes
- `.documents-toolbar` — flex row, `justify-content: space-between`, `align-items: center`, `gap: 6px`, `padding: 8px 16px`
- `.documents-toolbar-actions` — flex row for the right-side buttons
- `.filter-popover` — `position: absolute`, `top: calc(100% + 8px)`, `right: 0`, `width: 340px`, `z-index: 30`, white bg, shadow, border-radius
- `.filter-popover-btn-active` — active styling for the Фільтр button when open
- `.create-picker-popover` — `position: absolute`, `top: calc(100% + 8px)`, `right: 0`, `width: 200px`, `z-index: 30`
- `.create-picker-item` — menu item row inside the picker

### Modified classes
- `.documents-filter-panel` — removed (inline panel CSS is replaced by popover)
- `.documents-filter-toolbar` — removed (merged into `.documents-toolbar`)

## Config Changes

None — `DOCUMENT_FILTER_PRESETS` and `DOCUMENTS_FILTER_COPY.presetsLabel` are no longer rendered but may remain in config for potential future use. `createKind`-dependent logic in `getDocumentCreateLabel` is still used for the smart button label.

## Test Changes

`DocumentsScreen.test.ts` — remove or update tests that:
- Look for `documents-preset-*` test IDs (preset buttons removed)
- Test `createKind` selection logic
- Test the `documents-create-strip` test ID (replaced by `documents-toolbar`)

Add tests for:
- Create button shows "Створити ▾" when `kindFilter = null`
- Create button shows "Створити акт" when `kindFilter = "act"` and calls `create` directly
- Create picker opens on click when `kindFilter = null`
- Filter popover opens/closes via button click
