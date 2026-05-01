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

Added to `frontend/src/styles/tokens.css` so both components share it.

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

Covers Dashboard (4 KPI cards) and Documents/Reports focus cards (2 cards).

## Store Changes

Each store gains a `loading: boolean` field, initialised to `true`.

```ts
type DocumentsState = {
  items: DocumentItemDto[];
  loading: boolean;   // ← new, starts true
  // ...rest unchanged
};
```

Pattern for all load functions:

```ts
async function load() {
  store.update(s => ({ ...s, loading: true }));
  const items = await invoke('list_documents');
  store.update(s => ({ ...s, items, loading: false }));
}
```

Starting `loading: true` means the skeleton appears immediately on screen mount — no empty flash before the first fetch.

## Screen Integration

Each screen wraps its main content in an `{#if loading}` branch:

```svelte
{#if $documents.loading}
  <SkeletonCard count={2} />
  <SkeletonRow count={5} />
{:else}
  <!-- existing content -->
{/if}
```

### Screen Matrix

| Screen | SkeletonCard | SkeletonRow | count |
|---|---|---|---|
| DocumentsScreen | ✓ (focus cards) | ✓ | Card: 2, Row: 5 |
| PaymentsScreen | ✗ | ✓ | Row: 6 |
| CounterpartiesScreen | ✗ | ✓ | Row: 6 |
| TasksScreen | ✗ | ✓ compact | Row: 5 |
| DashboardScreen | ✓ | ✗ | Card: 4 |
| ReportsScreen | ✓ | ✓ | Card: 2, Row: 6 |

## File Changes

```
frontend/src/lib/components/
  SkeletonRow.svelte          ← new
  SkeletonCard.svelte         ← new

frontend/src/styles/tokens.css
  + @keyframes shimmer
  + .sk base class

frontend/src/lib/stores/
  documents.ts                ← add loading: boolean
  payments.ts                 ← add loading: boolean
  counterparties.ts           ← add loading: boolean
  dashboard.ts                ← add loading: boolean
  tasks.ts                    ← add loading: boolean
  reports.ts                  ← add loading: boolean

frontend/src/lib/screens/
  DocumentsScreen.svelte      ← integrate skeleton
  PaymentsScreen.svelte       ← integrate skeleton
  CounterpartiesScreen.svelte ← integrate skeleton
  TasksScreen.svelte          ← integrate skeleton
  DashboardScreen.svelte      ← integrate skeleton
  ReportsScreen.svelte        ← integrate skeleton
```

## Tests

```ts
// SkeletonRow.test.ts
// - renders correct number of rows via count prop
// - default variant includes icon block
// - compact variant omits icon block

// SkeletonCard.test.ts
// - renders correct number of cards via count prop

// DocumentsScreen.test.ts (existing)
// - when loading=true: SkeletonRow and SkeletonCard are visible, list is not
// - when loading=false: list is visible, skeletons are not
```

## Out of Scope

- Skeleton for modal/dialog content (separate initiative)
- Skeleton for inline editing states
- Dark mode skeleton color tuning (tokens already handle this via CSS vars)
