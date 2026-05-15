# Documents Drawer — Restructure Form Fields

**Date:** 2026-05-15  
**Status:** Approved  
**Scope:** `DocumentsScreen.svelte`, `documents.css`

---

## Problem

The drawer editor form has inconsistent field ordering and no display of the user's own company. Specifically:

- Контрагент appears only for new documents, buried inside the form grid with no consistent position
- Контрагент is shown in the header for existing docs but nowhere in the form — invisible once editing
- Напрямок is at the bottom of the form, but logically describes the document before number/date
- Примітки are above Позиції документа, but notes are secondary context and should come last
- No indication of which company (власна) issues the document

---

## Solution

Restructure the editor form fields into a clear, top-down logical order. Remove the counterparty line from the drawer header (it moves into the form). Add a read-only Компанія field sourced from `shellStore`.

---

## New Field Order (inside `editor-grid`)

| # | Поле | Поведінка |
|---|------|-----------|
| 1 | **Компанія** | Read-only, always visible. Value: `$shell.chrome.companyName`. Styled with light indigo background to indicate non-editable. |
| 2 | **Напрямок** | Radio: Вихідний / Вхідний. `editor-grid-span`. Unchanged. |
| 3 | **Номер + Дата** | Two-column row. Unchanged. |
| 4 | **Контрагент** | Always visible (not only for `pendingNew`). For new docs: `<select>`. For existing docs: read-only display (same indigo style as Компанія). `editor-grid-span`. |
| 5 | **Позиції документа** | The existing `editor-items-card` — position unchanged relative to other sections. |
| 6 | **Примітки** | `<textarea>`, moves to after Позиції. `editor-grid-span`. |

---

## Header Change

Remove the `<p>{$documents.editor.form.counterpartyName}</p>` line from the drawer header — the counterparty now lives in the form as field #4.

The `editor-header-meta` (kind badge + status chip) and `<h3>` title stay unchanged.

---

## Data Sources

- **Компанія**: `$shell.chrome.companyName` via `shellStore` (already used in `App.svelte`).  
  Import: `import { shellStore } from "../stores/shell";`  
  Usage: `const shell = shellStore;` → `$shell.chrome?.companyName ?? ""`

- **Контрагент (read-only)**: `$documents.editor.form.counterpartyName` — already on the form state.

---

## Visual Style

- **Компанія** field: light indigo background (`#f0f4ff`), indigo border (`#c7d2fe`), "тільки перегляд" label in faint text. CSS class: `editor-field-readonly`.
- **Контрагент (existing doc)**: same `editor-field-readonly` style.
- No other visual changes — buttons, badges, items card, and animations remain unchanged.

---

## CSS Changes

Add one reusable class to `documents.css`:

```css
.editor-field-readonly {
  background: color-mix(in srgb, var(--acta-color-accent-soft) 38%, var(--acta-color-bg-elevated));
  border: 1px solid color-mix(in srgb, var(--acta-color-accent) 22%, var(--acta-color-border));
  border-radius: 8px;
  padding: 9px 12px;
  display: flex;
  align-items: center;
  gap: 10px;
}
.editor-field-readonly-label {
  font-size: 10px;
  font-weight: 700;
  color: var(--acta-color-accent-text);
  min-width: 76px;
}
.editor-field-readonly-value {
  font-weight: 600;
  color: var(--acta-color-text);
}
.editor-field-readonly-hint {
  font-size: 10px;
  color: var(--acta-color-text-faint);
  margin-left: auto;
}
```

---

## Out of Scope

- No changes to toolbar, table, filter panel, bulk actions, nav tabs
- No changes to the items table itself (`editor-items-card` internals)
- No changes to PDF section
- No changes to Rust/Tauri backend — purely frontend restructure

---

## HTML Structure

The `editor-sheet` is a CSS grid (`display: grid; gap: 18px`). Its children are:

```
editor-header
editor-grid          ← form fields (2-col grid)
editor-items-card    ← items section (outside grid)
editor-items-card.existing-pdf-card  ← PDF (optional)
```

Примітки currently lives **inside** `editor-grid`. To place it after `editor-items-card`, it moves **outside** as a sibling. Use a second single-element grid or a `<label>` with `display:grid; gap:8px` to preserve field styling:

```html
<!-- after editor-items-card -->
<label class="editor-notes-field">
  Примітки
  <textarea ...></textarea>
</label>
```

Add to `documents.css`:
```css
.editor-notes-field {
  display: grid;
  gap: 8px;
}
```

The `editor-grid` then contains only: Компанія, Напрямок, Номер, Дата, Контрагент.

---

## Files to Change

1. `frontend/src/lib/screens/DocumentsScreen.svelte` — reorder form fields, add shell import, change header
2. `frontend/src/styles/documents.css` — add `.editor-field-readonly` class family
3. `frontend/src/lib/screens/__tests__/DocumentsScreen.test.ts` — update snapshots/assertions if needed
