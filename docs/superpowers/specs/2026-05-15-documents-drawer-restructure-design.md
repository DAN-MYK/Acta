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

## New Visual Order

Fields are rendered top-to-bottom. Items 1–4 are **inside** `editor-grid`; items 5–6 are **siblings** of `editor-grid` inside `editor-sheet`.

| # | Поле | Де | Поведінка |
|---|------|----|-----------|
| 1 | **Компанія** | `editor-grid`, full-width span | Read-only, always visible. See Data Sources. `editor-field-readonly` style. |
| 2 | **Напрямок** | `editor-grid`, full-width span | Radio: Вихідний / Вхідний. Unchanged. |
| 3 | **Номер** | `editor-grid`, half-column | Input. Unchanged. |
| 3 | **Дата** | `editor-grid`, half-column | Date input. Unchanged. |
| 4 | **Контрагент** | `editor-grid`, full-width span | For `pendingNew`: `<select>`. For existing: read-only `editor-field-readonly`. Always rendered. |
| 5 | **Позиції документа** | Outside `editor-grid`, sibling | `editor-items-card`. No change to internals. |
| 6 | **Примітки** | Outside `editor-grid`, sibling after items | `<label class="editor-notes-field">`. See HTML Structure. |

---

## Header Change

Remove the `<p>{$documents.editor.form.counterpartyName}</p>` line from the drawer header — the counterparty now lives in the form as field #4.

The `editor-header-meta` (kind badge + status chip) and `<h3>` title stay unchanged.

---

## Data Sources

- **Компанія**: `$shell.state?.chrome.companyName` via `shellStore`.  
  Import: `import { shellStore } from "../stores/shell";`  
  Usage: `const shell = shellStore;` → `$shell.state?.chrome.companyName ?? ""`  
  (In `App.svelte` line 83: `$: shellState = $shell.state` — верхній рівень store є `state`, не `chrome` напряму.)

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

Примітки currently lives **inside** `editor-grid`. To place it after `editor-items-card`, it moves **outside** as a sibling `<label>`.

`styles.css` applies `width`, `border`, `padding`, `background`, `min-height` and `resize` to `.editor-grid textarea` (lines 1285–1309). A textarea outside `.editor-grid` loses all of these. The `.editor-notes-field` CSS must replicate them explicitly:

```html
<!-- after editor-items-card, before existing-pdf-card -->
<label class="editor-notes-field">
  Примітки
  <textarea
    rows="3"
    value={$documents.editor.form.notes}
    on:input={onEditorNotesChange}
    disabled={$documents.loading}
  ></textarea>
</label>
```

Add to `documents.css`:
```css
.editor-notes-field {
  display: grid;
  gap: 8px;
}

.editor-notes-field textarea {
  width: 100%;
  box-sizing: border-box;
  min-height: 96px;
  border: 1px solid var(--acta-color-border);
  border-radius: var(--acta-radius-lg);
  padding: 8px 12px;
  background: var(--acta-color-bg-elevated);
  color: inherit;
  resize: vertical;
  font: inherit;
}
```

The `editor-grid` then contains only: Компанія, Напрямок, Номер, Дата, Контрагент.

---

## Files to Change

1. `frontend/src/lib/screens/DocumentsScreen.svelte` — reorder form fields, add shell import, change header
2. `frontend/src/styles/documents.css` — add `.editor-field-readonly` class family
3. `frontend/src/lib/screens/__tests__/DocumentsScreen.test.ts` — explicit test updates required:
   - Existing-doc editor: assert counterparty is rendered inside the form (`.editor-field-readonly`), NOT in the header `<p>` tag
   - Existing-doc editor: assert header has no `<p>` with counterparty name
   - Any editor state: assert company name from shell is rendered (`.editor-field-readonly` with `shellStore` value)
   - New-doc editor: assert counterparty `<select>` renders at position 4 (after Напрямок, after Номер/Дата)
