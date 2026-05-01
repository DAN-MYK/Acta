# UI Roadmap Wave 1 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [x]`) syntax for tracking.

**Goal:** Довести першу хвилю UI/UX roadmap до перевірюваного стану: shell і command palette мають передбачувані стани, standalone browser-dev режим працює без Tauri runtime, а `Reports`, `Settings`, `Documents` і `Payments` використовують узгоджену систему контролів, дат і мікрокопі.

**Architecture:** Додаємо окремий шар frontend fallback для `invoke`, який в browser-dev режимі повертає стабільні DTO-дані замість падіння на `@tauri-apps/api`. Поверх цього уніфікуємо shell/app-frame, button hierarchy, date controls і ключову мікрокопі на екранах, щоб візуальний аудит можна було виконувати прямо в Vite UI.

**Tech Stack:** Svelte 4, TypeScript, Vitest, jsdom, Tauri invoke API, CSS token-based UI styles.

---

### Task 1: Browser Fallback Contract

**Files:**
- Create: `frontend/src/lib/browser-fixtures.ts`
- Create: `frontend/src/lib/browser-api.ts`
- Modify: `frontend/src/lib/api.ts`
- Test: `frontend/src/lib/__tests__/browser-api.test.ts`

- [x] **Step 1: Write the failing test**
- [x] **Step 2: Run `npm run test:frontend -- browser-api.test.ts` and confirm the fallback path is missing**
- [x] **Step 3: Implement browser-only invoke fallback with stable fixture DTOs for shell, documents, counterparties, payments, reports, settings, dashboard, tasks, palette and BAS import**
- [x] **Step 4: Re-run `npm run test:frontend -- browser-api.test.ts` and confirm green**

### Task 2: Shell And Palette Reliability

**Files:**
- Modify: `frontend/src/App.svelte`
- Modify: `frontend/src/lib/stores/palette.ts`
- Modify: `frontend/src/styles.css`
- Test: `frontend/src/__tests__/App.test.ts`

- [x] **Step 1: Write failing tests for Ukrainian shell labels, `Esc` close behavior, disabled shell actions during reload, and palette reset semantics**
- [x] **Step 2: Run `npm run test:frontend -- App.test.ts` and confirm the new expectations fail**
- [x] **Step 3: Replace broken microcopy, harden shell busy-state gating, and keep palette state deterministic between opens/closes**
- [x] **Step 4: Re-run `npm run test:frontend -- App.test.ts` and confirm green**

### Task 3: Reports And Settings As Canonical UI Patterns

**Files:**
- Modify: `frontend/src/lib/screens/ReportsScreen.svelte`
- Modify: `frontend/src/lib/screens/SettingsScreen.svelte`
- Modify: `frontend/src/styles.css`
- Test: `frontend/src/lib/screens/__tests__/ReportsScreen.test.ts`
- Modify: `frontend/src/lib/screens/__tests__/SettingsScreen.test.ts`

- [x] **Step 1: Write failing tests for Ukrainian reports microcopy, canonical button classes, and localized density labels/settings integration states**
- [x] **Step 2: Run targeted Vitest commands and confirm failures**
- [x] **Step 3: Implement canonical actions, localized labels, scenario-first copy, and cleaner BAS/settings action rows**
- [x] **Step 4: Re-run the targeted tests until green**

### Task 4: Documents And Payments Control Consistency

**Files:**
- Modify: `frontend/src/lib/screens/DocumentsScreen.svelte`
- Modify: `frontend/src/lib/screens/PaymentsScreen.svelte`
- Modify: `frontend/src/styles.css`
- Modify: `frontend/src/lib/screens/__tests__/DocumentsScreen.test.ts`
- Modify: `frontend/src/lib/screens/__tests__/PaymentsScreen.test.ts`

- [x] **Step 1: Write failing tests for date field types, action hierarchy classes, and stronger payment/document CTA wording**
- [x] **Step 2: Run the two screen test files and confirm failures**
- [x] **Step 3: Convert date controls to canonical inputs, apply button hierarchy consistently, and improve action wording/layout**
- [x] **Step 4: Re-run the targeted tests until green**

### Task 5: Full Verification

**Files:**
- Modify: `docs/planning/ui-ux-roadmap-2026-05-01.md` (only if implementation changes the documented first-wave scope)

- [x] **Step 1: Run `npm run test:frontend`**
- [x] **Step 2: Run `npm run check`**
- [x] **Step 3: Run `npm run build`**
- [x] **Step 4: Reload `http://127.0.0.1:1420/` in the in-app browser and visually verify shell, reports, settings, documents and payments**
- [x] **Step 5: Report verified outcomes and any residual gaps with explicit evidence**

---

## Статус реалізації

**Повністю реалізовано** — 2026-05-01

| Задача | Коміт | Статус |
|--------|-------|--------|
| Task 1: Browser Fallback Contract | `a707425` | ✅ |
| Task 2: Shell And Palette Reliability | `a707425` | ✅ |
| Task 3: Reports And Settings As Canonical UI Patterns | `a707425` | ✅ |
| Task 4: Documents And Payments Control Consistency | `a707425` | ✅ |
| Task 5: Full Verification | `4a78c7b`, `61ee8b3` | ✅ |

Деталі того що реалізовано — у `docs/planning/ui-ux-roadmap-2026-05-01.md` у розділі «Статус реалізації станом на 2026-05-01».
