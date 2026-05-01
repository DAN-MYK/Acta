# Skeleton Loaders — Design Spec

**Date:** 2026-05-01  
**Status:** Approved

## Overview

Add shimmer-style skeleton loaders to all data-heavy screens in Acta to eliminate "content pop-in" and communicate loading state clearly to users.

## Animation Style

**Shimmer sweep** — a gradient highlight moves left-to-right across placeholder blocks.

```css
@keyframes shimmer {
  0%   { background-position: -600px 0; }
  100% { background-position: 600px 0; }
}

.sk {
  border-radius: 5px;
  background: linear-gradient(
    90deg,
    var(--bg-subtle) 25%,
    var(--bg) 50%,
    var(--bg-subtle) 75%
  );
  background-size: 1200px 100%;
  animation: shimmer 1.5s infinite linear;
}
```

Added to `frontend/src/lib/styles/tokens.css` so both components share it.

## Components

### `SkeletonRow.svelte`

Renders N placeholder list rows. Mirrors the layout of document/payment/counterparty/task rows: icon block + two text lines + amount block + badge block.

```svelte
<!-- Props -->
let count: number = 5;
let variant: 'default' | 'compact' = 'default';
```

- `default` — icon (34×34) + 2 lines + amount + badge. Covers Documents, Payments, Counterparties.
- `compact` — no icon, 2 lines + small badge. Covers Tasks.

Line widths vary per row (40%, 55%, 65%…) to look natural, not grid-like.

### `SkeletonCard.svelte`

Renders N placeholder KPI cards in a 2-column grid. Each card: label line + large value block + subtitle line.

```svelte
<!-- Props -->
let count: number = 4;
```

Covers Dashboard (4 KPI cards) and Documents focus cards (2 cards).

## Store Changes

### `initialLoading` vs `loading`

Stores already use `loading: boolean` for all async operations — save, open, reconcile, export. Reusing that flag for skeleton display would cause skeleton flashes on every user action.

Solution: add a separate `initialLoading: boolean` field that is `true` only until the first successful data fetch completes. It starts `true` and is set to `false` once — it never resets to `true` again.

```ts
type DocumentsState = {
  initialLoading: boolean;  // ← new: true until first fetch, then permanently false
  loading: boolean;         // existing: save/open/reconcile/export operations
  items: DocumentItemDto[];
  // ...rest unchanged
};

// Initial state
const initial: DocumentsState = {
  initialLoading: true,
  loading: false,
  // ...
};
```

Pattern in the load function:

```ts
async function load() {
  store.update(s => ({ ...s, loading: true }));
  const items = await invoke('list_documents');
  store.update(s => ({
    ...s,
    items,
    loading: false,
    initialLoading: false,   // set once, never reset
  }));
}
```

Screen template uses `initialLoading`, not `loading`:

```svelte
{#if $documents.initialLoading}
  <SkeletonCard count={2} />
  <SkeletonRow count={5} />
{:else}
  <!-- existing content; $documents.loading still drives button/spinner states -->
{/if}
```

## Screen Integration

Skeleton replaces only the data-driven areas. Persistent chrome — action bars, filter strips, tab navigation — stays visible so users can interact while data loads.

### Screen Matrix

| Screen | Stays visible (chrome) | Gets skeleton |
|---|---|---|
| **DocumentsScreen** | panel-header, search, create strip | focus cards (2), document list (5 rows) |
| **PaymentsScreen** | panel-header, create-strip-card (import/sync/manual buttons), KPI row | payment list rows (6) when list exists |
| **CounterpartiesScreen** | panel-header, search | counterparty list (6 rows) |
| **TasksScreen** | panel-header, filter tabs | task list (5 rows, compact variant) |
| **DashboardScreen** | panel-header (with refresh button) | KPI cards (4), cashflow table rows (4), recent docs (3), upcoming payments (3) |
| **ReportsScreen** | panel-header, tab bar, filter controls | data table rows (6) |

Payments note: `create-strip-card` contains the primary action buttons (Import CSV, Sync bank, Manual entry) — these are always interactive and must never be hidden behind a skeleton.

Dashboard note: `$dashboard.screen` is null until load completes. Each `{#each ... ?? []}` renders nothing now. Skeleton wraps each `{#each}` section individually, not the whole panel, so the panel-header and card titles stay visible.

## File Changes

```
frontend/src/lib/components/
  SkeletonRow.svelte              ← new
  SkeletonCard.svelte             ← new

frontend/src/lib/styles/tokens.css
  + @keyframes shimmer
  + .sk base class

frontend/src/lib/stores/
  documents.ts        ← add initialLoading: boolean
  payments.ts         ← add initialLoading: boolean
  counterparties.ts   ← add initialLoading: boolean
  dashboard.ts        ← add initialLoading: boolean
  tasks.ts            ← add initialLoading: boolean
  reports.ts          ← add initialLoading: boolean

frontend/src/lib/screens/
  DocumentsScreen.svelte      ← integrate skeleton
  PaymentsScreen.svelte       ← integrate skeleton (list only)
  CounterpartiesScreen.svelte ← integrate skeleton
  TasksScreen.svelte          ← integrate skeleton (compact)
  DashboardScreen.svelte      ← integrate skeleton (per-section)
  ReportsScreen.svelte        ← integrate skeleton (table only)
```

## Tests

```ts
// SkeletonRow.test.ts
// - renders correct number of rows via count prop
// - default variant includes icon block
// - compact variant omits icon block

// SkeletonCard.test.ts
// - renders correct number of cards via count prop

// DocumentsScreen.test.ts (extend existing)
// - when initialLoading=true: SkeletonRow and SkeletonCard are visible, list is not
// - when initialLoading=false: list is visible, skeletons are not
// - when loading=true (save op): skeleton is NOT shown, existing content stays
```

## Out of Scope

- Skeleton for modal/dialog content (separate initiative)
- Skeleton for inline editing states
- Dark mode skeleton color tuning (tokens already handle this via CSS vars)
