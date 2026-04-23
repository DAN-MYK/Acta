---
name: Slint UI Components
description: >
  Comprehensive guide for writing .slint files in the Acta project. Use whenever
  creating, editing, or debugging Slint UI — new components, screens, callbacks,
  layouts, data bindings, animations, keyboard handling, or Rust↔Slint wiring.
  Even if the task looks like "just add a label" — invoke this skill to ensure
  design tokens, money contract, and project conventions are followed.
---

# Slint Skill — Acta Project

## 1. File structure & imports

```
ui/
├── design-tokens.slint  ← AppTheme global (colors, fonts, spacing, radii)
├── types.slint          ← All struct/enum definitions
├── components.slint     ← Shared primitives
├── icons.slint          ← Icons global
├── shell.slint          ← Navigation shell
├── app.slint            ← Root AppWindow (wiring only)
└── <screen>.slint       ← Per-screen components
```

Always import only what you use:
```slint
import { AppTheme } from "design-tokens.slint";
import { NavScreen, DocumentStatus } from "types.slint";
import { Card, PrimaryButton, Badge } from "components.slint";
```

## 2. Property kinds

```slint
in property <T> foo;           // Rust → Slint (read-only inside .slint)
out property <T> bar;          // Slint → Rust
in-out property <T> baz;       // bidirectional — REQUIRED for overlay flags
private property <T> internal; // component-internal only
```

**Rule**: Use `in-out` for any boolean that controls modals/overlays — they must be
closeable from BOTH Rust (`set_show_x(false)`) and Slint (cancel button).

## 3. Bindings & two-way sync

```slint
// One-way (reactive)
text: root.counter;

// Two-way sync with <=>
text-input.text <=> root.value;

// Conditional expression (ternary)
color: active ? AppTheme.accent : AppTheme.text-muted;

// Chained conditional
color:
    status == DocumentStatus.Paid    ? AppTheme.success :
    status == DocumentStatus.Overdue ? AppTheme.danger  :
    status == DocumentStatus.Issued  ? AppTheme.info    :
    AppTheme.text-muted;
```

## 4. Layouts

```slint
VerticalLayout {
    padding: AppTheme.sp-4;   // uniform
    padding-left: AppTheme.sp-3;  // per-side override
    spacing: AppTheme.sp-2;
    alignment: start | end | center | space-between | stretch;

    Rectangle { vertical-stretch: 1; }  // spacer
}

HorizontalLayout {
    spacing: AppTheme.sp-2;
    alignment: stretch;

    Text { horizontal-stretch: 1; }  // fill remaining width
}

GridLayout {
    Row { Text {}  TextInput {} }
    Row { Text {}  TextInput {} }
}
```

## 5. Conditional rendering

```slint
// Show element
if condition : Rectangle { ... }

// Show/hide block
if root.show-detail : VerticalLayout { ... }

// Inline if (label vs placeholder)
if root.value == "" : Text {
    text: "Placeholder…";
    color: AppTheme.text-faint;
    accessible-role: none;
}
```

## 6. List rendering (for…in)

```slint
// Simple
for item in root.items : Text { text: item.name; }

// With index
for item[i] in root.items : Rectangle {
    background: Math.mod(i, 2) == 0 ? AppTheme.bg-stripe : transparent;
    // NOTE: use Math.mod(i, 2) — operator % is NOT supported in Slint
}

// Sparse array inline
for row in [
    { k: "Ctrl+1", d: "Головна" },
    { k: "Ctrl+2", d: "Документи" },
] : HorizontalLayout { ... }
```

## 7. Callbacks

```slint
// Declaration
callback save(string, string);  // named params implicit
callback row-selected(int);
callback toggled(bool);

// Handling inside .slint
button.clicked => { root.save(field.text, date.text); }

// Forwarding
child.clicked => { root.clicked(); }

// Passing data back from handler
for item[i] in root.items : Rectangle {
    TouchArea {
        clicked => { root.row-selected(i); }
    }
}
```

## 8. Enums & structs

Declare in `types.slint`, use everywhere:

```slint
// types.slint
export enum DocumentStatus { Draft, Issued, Signed, Paid, Overdue }
export struct Act {
    id: string,
    number: string,
    amount-str: string,    // ← money is ALWAYS string, pre-formatted in Rust
    status: DocumentStatus,
}

// usage
in property <[Act]> acts;
in property <DocumentStatus> status;

// comparison
if root.status == DocumentStatus.Paid : Badge { tone: "success"; }
```

## 9. Money contract — CRITICAL

**Financial amounts are ALWAYS `string` in Slint. Never `float` or `int`.**

- Format in Rust: `rust_decimal::Decimal` → `format!("{:.2}", amount)` → `SharedString`
- `float` is only allowed for normalized chart heights (0.0–1.0 range)

```slint
// CORRECT
in property <string> amount-str;   // "₴ 12 500,00"
Text { text: root.amount-str; font-family: AppTheme.font-mono; }

// WRONG
in property <float> amount;        // never for money
```

## 10. Design tokens (AppTheme)

All visual values come from `AppTheme` global — never hardcode colors or sizes.

### Colors
```
bg, bg-elevated, bg-subtle, bg-hover, bg-sidebar, bg-stripe
border, border-strong
text, text-muted, text-faint
accent, accent-hover, accent-soft, accent-text
success, success-soft
warning, warning-soft
danger, danger-soft
info, info-soft
```

### Typography
```
font-sans: "Geist"            ← UI text
font-serif: "Source Serif 4"  ← headings
font-mono: "JetBrains Mono"   ← numbers, codes

font-xs: 10px  font-sm: 11px  font-body: 13px
font-md: 15px  font-lg: 20px  font-xl: 26px
```

### Spacing
```
sp-1: 4px  sp-2: 8px  sp-3: 12px  sp-4: 16px
sp-5: 20px sp-6: 24px sp-7: 32px  sp-8: 40px
```

### Radii
```
radius-sm: 4px  radius-md: 6px  radius-lg: 8px
radius-xl: 10px radius-2xl: 14px
```

## 11. Component library (components.slint)

Use these — don't reinvent:

| Component | Props | Use for |
|-----------|-------|---------|
| `Card` | `padded: bool` | Elevated surface, hairline border |
| `PrimaryButton` | `text`, `enabled`, `clicked` | Main CTA |
| `SecondaryButton` | `text`, `small`, `clicked` | Secondary action |
| `GhostButton` | `text`, `small`, `clicked` | Tertiary / inline |
| `IconButton` | `icon`, `active`, `tooltip`, `clicked` | Icon-only action |
| `SearchInput` | `placeholder`, `value`, `search-icon`, `edited` | Search fields |
| `Badge` | `text`, `tone` | Status labels |
| `StatusDot` | `tone` | 6px status indicator |
| `HDivider` / `VDivider` | — | 1px hairline separator |
| `Checkbox` | `checked`, `indeterminate`, `check-icon`, `toggled` | Selection |
| `TableHeaderCell` | `label`, `align` | Table column headers |
| `MonoNumber` | `value`, `tone`, `formatted` | Tabular numbers |
| `HeadingLg/Md/Sm` | `text` | Semantic headings |
| `SectionLabel` | `label` | Uppercase section labels |
| `Avatar` | `initials`, `tone`, `size` | User/company avatars |
| `FilterPill` | `label`, `value`, `close-icon`, `removed` | Active filter chips |
| `BulkBar` | `selected-count`, actions | Bulk selection bar |
| `KbdHint` | `key` | Keyboard shortcut hint chip |
| `DocChain` | `steps`, `create-next` | Invoice→Act→Waybill chain |
| `SparkLine` | `data` (float[] 0-1), `line-color` | Inline trend chart |
| `BarChart` | `bars: [ChartBar]` | Dual revenue/expense bars |
| `SimpleProgressBar` | `value` (0-1), `fill-color` | Progress indicator |

Badge / StatusDot tones: `"muted"` `"info"` `"success"` `"warning"` `"danger"` `"accent"`

## 12. Hover state pattern

Use `has-hover` from `TouchArea` instead of a separate `hovered` property where possible:

```slint
// Simple: direct has-hover
Rectangle {
    background: touch.has-hover ? AppTheme.bg-hover : transparent;
    touch := TouchArea { mouse-cursor: pointer; clicked => { ... } }
}

// When hover affects multiple children — use private property
component NavItem inherits Rectangle {
    private property <bool> hovered: false;

    background: hovered ? AppTheme.bg-hover : transparent;

    Image { colorize: root.hovered ? AppTheme.text : AppTheme.text-muted; }

    TouchArea {
        mouse-cursor: pointer;
        pointer-event(e) => {
            if (e.kind == PointerEventKind.move) {
                root.hovered = self.has-hover;
            }
        }
        clicked => { root.clicked(); }
    }
}
```

**Rule**: ALWAYS write `self.enabled` (not bare `enabled`) inside TouchArea when referencing own properties.

## 13. Keyboard & focus handling

```slint
// FocusScope captures key events
scope := FocusScope {
    accessible-label: "Навігація";
    init => { self.focus(); }  // grab focus on mount

    key-pressed(e) => {
        if e.text == Key.Escape {
            root.closed();
            return accept;          // consume event
        }
        if (e.text == "k" && e.modifiers.control) {
            root.open-cmd-palette();
            return accept;
        }
        reject  // pass to next handler
    }
}

// Key constants: Key.Escape, Key.Return, Key.Tab, Key.Space
// Modifier: e.modifiers.control / .shift / .alt / .meta
```

## 14. Images & icons

```slint
// Load at compile time
export global Icons {
    out property <image> home: @image-url("assets/icons/Home.svg");
}

// Display with colorize (SVG recolor)
Image {
    source: Icons.home;
    width: 16px;
    height: 16px;
    colorize: AppTheme.text-muted;  // tints SVG
    image-fit: contain;
}

// Dynamic image from Rust (e.g. chart render via plotters)
in property <image> chart-image;
Image { source: root.chart-image; width: 100%; height: 400px; }
```

## 15. Shadows — DESIGN RULE

```slint
// ONLY on floating surfaces (modals, popovers, command palette)
Rectangle {
    drop-shadow-blur: 24px;
    drop-shadow-color: #1e1e1e.with-alpha(0.08);
}

// Content cards — NO shadow, hairline border only
Rectangle {
    border-width: 1px;
    border-color: AppTheme.border;
    // no drop-shadow-*
}
```

## 16. Accessibility

```slint
// Semantic roles
accessible-role: button;     // clickable elements
accessible-role: text;       // informational text
accessible-role: none;       // decorative (placeholder overlays, icons)

// Labels for screen readers
accessible-label: "Компанія: \{root.company-name}";

// Focus scope label
FocusScope {
    accessible-role: none;
    accessible-label: "Список документів — стрілки для навігації";
}
```

## 17. Animations

```slint
// Property animation
property <length> panel-x: 0px;
animate panel-x { duration: 200ms; easing: ease-in-out; }

// Color transition
background: hovered ? AppTheme.bg-hover : transparent;
animate background { duration: 120ms; }
```

## 18. Rust ↔ Slint wiring

### Types mapping

| Slint | Rust |
|-------|------|
| `string` | `SharedString` / `.into()` |
| `int` | `i32` |
| `float` | `f32` |
| `bool` | `bool` |
| `[T]` | `ModelRc<T>` via `VecModel` |
| `image` | `slint::Image` |
| `struct Foo` | generated `Foo` struct |
| `enum Bar` | generated `Bar` enum |

### Reading model from Rust

```rust
use slint::Model;

let model: ModelRc<SharedString> = ui.get_items();
let vec: Vec<SharedString> = (0..model.row_count())
    .filter_map(|i| model.row_data(i))
    .collect();
// MUST import slint::Model for row_count()/row_data()
```

### Setting list data

```rust
use slint::{VecModel, ModelRc};
use std::rc::Rc;

let items = vec![Act { id: "1".into(), number: "АКТ-001".into(), ... }];
let model: ModelRc<Act> = Rc::new(VecModel::from(items)).into();
ui.set_acts(model);
```

### Event loop patterns

```rust
// Main thread (before ui.run())
let data = prepare_data().await;
apply_to_ui(&ui, data);  // sync, direct set_*

// Background thread → UI (after ui.run())
let ui_handle = ui.as_weak();
tokio::spawn(async move {
    let data = load_data().await;
    let _ = ui_handle.upgrade_in_event_loop(move |ui| {
        apply_to_ui(&ui, data);
    });
});

// NEVER use upgrade_in_event_loop before ui.run() — causes empty first frame
```

### Callback wiring

```rust
// Simple callback
ui.on_doc_new(|| {
    println!("New document requested");
});

// With params
ui.on_row_selected(|row_idx| {
    // handle selection
});

// Async callback pattern (most common)
let pool = pool.clone();
let ui_handle = ui.as_weak();
ui.on_doc_open(move |id| {
    let pool = pool.clone();
    let ui_handle = ui_handle.clone();
    tokio::spawn(async move {
        let doc = db::get_doc(&pool, &id.to_string()).await;
        let _ = ui_handle.upgrade_in_event_loop(move |ui| {
            ui.set_current_doc(doc.into());
        });
    });
});
```

## 19. Common gotchas

| Problem | Fix |
|---------|-----|
| `i % 2` gives error | Use `Math.mod(i, 2)` |
| `enabled` unknown in TouchArea | Write `self.enabled` |
| `StandardListViewItem` not found | It's a global type — don't import it |
| `ListView` not found | `import { ListView } from "std-widgets.slint"` |
| `row_count()` method not found | Add `use slint::Model;` |
| Overlay closes from Rust but not from Slint button | Make property `in-out`, not `in` |
| Window opens blank | Don't use `upgrade_in_event_loop` before `ui.run()` — use `apply_*` directly |
| Linter "assignment on input property" | Change `in property` → `in-out property` |

## 20. Workflow when adding a new screen

1. Create `ui/<screen>.slint` — declare data properties using ViewData struct from `types.slint`
2. Add struct to `types.slint` if needed
3. Export component, import in `app.slint`
4. Wire `in property <ScreenViewData> screen-name;` in AppWindow
5. Add `if current-screen == NavScreen.X : ScreenComponent { data: root.screen-name; ... }` in app.slint
6. In Rust: add `prepare_screen_data()` → `apply_screen_to_ui()` pair
7. Register all callbacks in `src/ui/<screen>.rs`
8. Call `cargo sqlx prepare` if new DB queries added
