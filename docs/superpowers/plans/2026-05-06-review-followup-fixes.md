# Code Review Followup Fixes — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix all 10 groups of issues found in the code review of `codex/p1-ui-polish-followup`.

**Architecture:** Fixes span frontend (Svelte screens + stores + CSS tokens) and scripts (encoding guard, vite launcher). No backend changes needed — all reviewed Rust files had only CRLF/stat noise.

**Tech Stack:** Svelte 5 / TypeScript, CSS custom properties (--acta-* token system), Node ESM scripts.

---

## Task 1 (C1): Fix mojibake in money.test.ts + tighten encoding regex

**Files:**
- Modify: `frontend/src/lib/__tests__/money.test.ts:98`
- Modify: `scripts/check-text-encoding.mjs:9-14`

- [ ] Replace `вЂ"` (broken em-dash) with `—` on line 98 of money.test.ts
- [ ] Add single-occurrence em-dash mojibake pattern to mojibakePatterns array
- [ ] Run encoding check: `node scripts/check-text-encoding.mjs`

---

## Task 2 (C2): Reactive dayLabel in TasksScreen

**Files:**
- Modify: `frontend/src/lib/screens/TasksScreen.svelte`

- [ ] Add `onDestroy` import, create `dayLabel` variable, daily refresh via setInterval
- [ ] Replace `{getDayLabel()}` with `{dayLabel}` in template

---

## Task 3 (W1+I1): Remove query + isEditorDirty from stores

**Files:**
- Modify: `frontend/src/lib/stores/tasks.ts`
- Modify: `frontend/src/lib/stores/counterparties.ts`
- Modify: `frontend/src/lib/screens/__tests__/CounterpartiesScreen.test.ts`

- [ ] tasks.ts: remove `query` from interface + state, simplify `load()` to always pass `""`, remove `isEditorDirty()`
- [ ] counterparties.ts: remove `isEditorDirty()`
- [ ] CounterpartiesScreen.test.ts: remove `isEditorDirty` mock

---

## Task 4 (W2): Unify inert pattern in CounterpartiesScreen

**Files:**
- Modify: `frontend/src/lib/screens/CounterpartiesScreen.svelte`

- [ ] Remove `let panelElement: HTMLElement | null = null`
- [ ] Remove reactive block with setAttribute/removeAttribute
- [ ] Replace `bind:this={panelElement}` with declarative `inert={...} aria-hidden={...}` on `<section>`

---

## Task 5 (W3): Escape key via svelte:window

**Files:**
- Modify: `frontend/src/lib/screens/TasksScreen.svelte`

- [ ] Remove `on:keydown={onBackdropKeydown}` from backdrop div
- [ ] Add `<svelte:window>` handler that calls `requestClose()` on Escape when editor is open

---

## Task 6 (W5): Fix vite path in ensure-vite-dev.mjs

**Files:**
- Modify: `scripts/ensure-vite-dev.mjs`

- [ ] Add `import { createRequire } from "node:module";`
- [ ] Replace `path.resolve(scriptDir, ...)` with `createRequire(import.meta.url).resolve("vite/bin/vite.js")`
- [ ] Remove now-unused `path`, `fileURLToPath`, `scriptDir` declarations

---

## Task 7 (W6): Complete PaymentsScreen token migration

**Files:**
- Modify: `frontend/src/lib/screens/PaymentsScreen.svelte`

- [ ] Replace `--bg-card` → `--acta-color-bg-elevated` (all occurrences)
- [ ] Replace `--border-hairline` → `--acta-color-border` (all occurrences)
- [ ] Replace `--accent-soft` → `--acta-color-accent-soft` (all occurrences)
- [ ] Replace `--accent)` → `--acta-color-accent)` (exact, not --accent-text etc.)
- [ ] Replace `--text-muted` → `--acta-color-text-muted` (all occurrences)
- [ ] Replace `--danger, #c2410c` → `--acta-color-danger` (remaining occurrences)

---

## Task 8 (W7+W8): CounterpartiesScreen inline styles + Dashboard semantics

**Files:**
- Modify: `frontend/src/lib/screens/CounterpartiesScreen.svelte`
- Modify: `frontend/src/styles/counterparties.css`
- Modify: `frontend/src/lib/screens/DashboardScreen.svelte`
- Modify: `frontend/src/styles/dashboard.css`
- Modify: `frontend/src/lib/screens/__tests__/DashboardScreen.test.ts`

- [ ] CounterpartiesScreen: replace `style="margin: 28px;"` with class `.cp-inset-pad`
- [ ] CounterpartiesScreen: replace `style="margin-top: 8px;"` with class `.cp-badge-row`
- [ ] CounterpartiesScreen: replace `style="font-size: 18px;"` with class `.cp-metric-date-val`
- [ ] counterparties.css: add those three classes
- [ ] Dashboard: add `<h2 class="sr-only">Дашборд</h2>`, restore `<article>` for KPI cards
- [ ] dashboard.css: add `.sr-only` utility if not already global
- [ ] DashboardScreen.test.ts: no changes needed (both texts present)

---

## Task 9 (I2-I5): TasksScreen CSS cleanup

**Files:**
- Modify: `frontend/src/lib/screens/TasksScreen.svelte`

- [ ] I2: Replace `rgba(10, 12, 16, 0.38)` → `var(--acta-color-bg-overlay)`, `rgba(0, 0, 0, 0.1)` → `var(--acta-shadow-modal)` (box-shadow value)
- [ ] I3: Remove entire local `.btn-primary`, `.btn-secondary`, `.btn-ghost`, `.btn-danger`, `.btn-sm` CSS blocks (global ones in styles.css take over)
- [ ] I4: Replace `<div style="flex: 1" />` spacer with `margin-left: auto` on the preceding button
- [ ] I5: Replace inline style on skeleton KPI strip with class `.kpi-skeleton`

---

## Task 10 (I7): Remove code-simplifier from project settings

**Files:**
- Modify: `.claude/settings.json`

- [ ] Remove `"code-simplifier@claude-plugins-official": true` entry

---
