# Fix Document Chain — Code Review Issues Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [x]`) syntax for tracking.

**Goal:** Виправити всі 7 проблем знайдених при code review документного ланцюжка в `src/ui/documents.rs`.

**Architecture:** Всі зміни — в одному файлі `src/ui/documents.rs`. Критичні виправлення (cycle guard, error propagation), видалення мертвого коду, мінімальний рефакторинг дублювання, unit тести для чистих функцій.

**Tech Stack:** Rust, anyhow, std::collections::HashSet, #[cfg(test)]

---

## Файли що змінюються

- Modify: `src/ui/documents.rs` — всі 7 виправлень + нові unit тести

---

### Task 1: Виправити propagation помилки в `find_document_by_parent_ref` (Critical #2)

**Files:**
- Modify: `src/ui/documents.rs:410,434,463`

Три виклики `.await.unwrap_or_default()` ковтають DB помилки без логу. Замінити на `.await?`.

- [x] **Step 1: Замінити unwrap_or_default → ? (три місця)**

У `src/ui/documents.rs` знайти і замінити:

```rust
// Рядок ~410
for row in db::acts::list(pool, company_id, None).await.unwrap_or_default() {
// замінити на:
for row in db::acts::list(pool, company_id, None).await? {

// Рядок ~434
for row in db::invoices::list(pool, company_id, None).await.unwrap_or_default() {
// замінити на:
for row in db::invoices::list(pool, company_id, None).await? {

// Рядок ~463
for row in db::waybills::list(pool, company_id, None).await.unwrap_or_default() {
// замінити на:
for row in db::waybills::list(pool, company_id, None).await? {
```

- [x] **Step 2: Перевірити компіляцію**

```bash
cargo build --tests 2>&1 | tail -5
```
Expected: `Finished` без помилок.

- [x] **Step 3: Commit**

```bash
git add src/ui/documents.rs
git commit -m "fix: propagate DB errors in find_document_by_parent_ref instead of swallowing"
```

---

### Task 2: Додати cycle detection в `load_document_chain` (Critical #1)

**Files:**
- Modify: `src/ui/documents.rs:878`

Цикл `while let Some(parent_ref) = ...` не захищений від циклічних посилань у БД.

- [x] **Step 1: Додати `use std::collections::HashSet;` у верх файлу (якщо відсутній)**

Перевірити imports у верхній частині файлу. Якщо `HashSet` не імпортовано — додати:

```rust
use std::collections::HashSet;
```

- [x] **Step 2: Замінити while-цикл у `load_document_chain` на варіант з visited set**

Знайти блок:
```rust
    let mut current = source.clone();
    while let Some(parent_ref) = split_visible_notes_and_chain_parent(current.notes.as_deref()).1 {
        let parent_doc_ref = parse_document_ref(&parent_ref)
            .ok_or_else(|| anyhow!("Некоректний parent link у документі {}", current.number))?;
        let parent = load_document_snapshot(pool, company_id, parent_doc_ref).await?;
        match parent.kind.as_str() {
            "invoice" if invoice.is_none() => invoice = Some(parent.clone()),
            "act" if act.is_none() => act = Some(parent.clone()),
            "waybill" if waybill.is_none() => waybill = Some(parent.clone()),
            _ => {}
        }
        current = parent;
    }
```

Замінити на:
```rust
    let mut current = source.clone();
    let mut visited: HashSet<String> = HashSet::new();
    visited.insert(current.ref_id.clone());
    while let Some(parent_ref) = split_visible_notes_and_chain_parent(current.notes.as_deref()).1 {
        let parent_doc_ref = parse_document_ref(&parent_ref)
            .ok_or_else(|| anyhow!("Некоректний parent link у документі {}", current.number))?;
        let parent = load_document_snapshot(pool, company_id, parent_doc_ref).await?;
        if visited.contains(&parent.ref_id) {
            tracing::warn!("chain: виявлено цикл у ланцюжку документів при обробці {}", current.number);
            break;
        }
        visited.insert(parent.ref_id.clone());
        match parent.kind.as_str() {
            "invoice" if invoice.is_none() => invoice = Some(parent.clone()),
            "act" if act.is_none() => act = Some(parent.clone()),
            "waybill" if waybill.is_none() => waybill = Some(parent.clone()),
            _ => {}
        }
        current = parent;
    }
```

- [x] **Step 3: Перевірити компіляцію**

```bash
cargo build --tests 2>&1 | tail -5
```
Expected: `Finished` без помилок.

- [x] **Step 4: Commit**

```bash
git add src/ui/documents.rs
git commit -m "fix: add cycle detection to load_document_chain parent traversal"
```

---

### Task 3: Видалити мертвий код `load_document_chain_legacy` та `prefill_items_from_source` (Important #3, #4)

**Files:**
- Modify: `src/ui/documents.rs:771–865, 934–986`

Обидві функції `pub` але ніде не викликаються. `load_document_chain_legacy` використовує неправильний алгоритм (пошук за counterparty_name).

- [x] **Step 1: Видалити `load_document_chain_legacy`**

Видалити весь блок від рядка `/// Завантажує ланцюг пов'язаних документів` (doc comment) до кінця функції `load_document_chain_legacy` включно — рядки ~769–865.

- [x] **Step 2: Видалити `prefill_items_from_source`**

Видалити весь блок від рядка `/// Prefills items for new document` (doc comment) до кінця функції `prefill_items_from_source` — рядки ~934–986 (після зсуву від попереднього видалення).

- [x] **Step 3: Перевірити компіляцію та що нема посилань**

```bash
cargo build --tests 2>&1 | grep -E "error|warning.*unused|load_document_chain_legacy|prefill_items_from_source"
```
Expected: жодних рядків.

- [x] **Step 4: Commit**

```bash
git add src/ui/documents.rs
git commit -m "refactor: remove dead pub functions load_document_chain_legacy and prefill_items_from_source"
```

---

### Task 4: Усунути подвійний виклик parse у `create_chain_draft_from_source` (Important #5)

**Files:**
- Modify: `src/ui/documents.rs` (~рядок 1008 до видалень)

`DocumentRef` є `Copy`. Замінити `load_chain_from_id(pool, company_id, source_id)` на `load_document_chain(pool, company_id, source_ref)` — уникає повторного парсингу рядка.

- [x] **Step 1: Замінити виклик у `create_chain_draft_from_source`**

Знайти рядок:
```rust
    let chain_steps = load_chain_from_id(pool, company_id, source_id).await?;
```

Замінити на:
```rust
    let chain_steps = load_document_chain(pool, company_id, source_ref).await?;
```

(`source_ref` вже розпарсований вище у функції і є `Copy`)

- [x] **Step 2: Перевірити компіляцію**

```bash
cargo build --tests 2>&1 | tail -5
```
Expected: `Finished` без помилок.

- [x] **Step 3: Commit**

```bash
git add src/ui/documents.rs
git commit -m "refactor: use already-parsed source_ref in create_chain_draft_from_source"
```

---

### Task 5: Зробити `load_counterparty_name` приватною (Minor #8)

**Files:**
- Modify: `src/ui/documents.rs:495`

Функція стала `pub` без зовнішніх викликів.

- [x] **Step 1: Прибрати `pub`**

Знайти:
```rust
pub async fn load_counterparty_name(pool: &PgPool, company_id: Uuid, counterparty_id: Uuid) -> Result<String> {
```
Замінити на:
```rust
async fn load_counterparty_name(pool: &PgPool, company_id: Uuid, counterparty_id: Uuid) -> Result<String> {
```

- [x] **Step 2: Перевірити що немає зовнішніх використань**

```bash
grep -rn "load_counterparty_name" src/ --include="*.rs"
```
Expected: лише рядки всередині `src/ui/documents.rs`.

- [x] **Step 3: Перевірити компіляцію**

```bash
cargo build --tests 2>&1 | tail -5
```

- [x] **Step 4: Commit**

```bash
git add src/ui/documents.rs
git commit -m "refactor: make load_counterparty_name private (no external callers)"
```

---

### Task 6: Додати unit тести для чистих функцій chain логіки (Important #7)

**Files:**
- Modify: `src/ui/documents.rs` — блок `#[cfg(test)] mod tests` (~рядок 1806)

Функції `split_visible_notes_and_chain_parent`, `compose_notes_with_chain_parent`, `normalize_chain_kind`, `can_create_chain_target`, `chain_kind_rank` не мають тестів.

- [x] **Step 1: Дописати тести у існуючий `mod tests`**

Знайти кінець блоку `#[cfg(test)] mod tests` (закриваюча `}`) і перед нею вставити:

```rust
    #[test]
    fn split_notes_extracts_parent_ref() {
        let (visible, parent) = super::split_visible_notes_and_chain_parent(
            Some("Видно користувачу\n\n[chain-parent:act:some-uuid]"),
        );
        assert_eq!(visible, "Видно користувачу");
        assert_eq!(parent.as_deref(), Some("act:some-uuid"));
    }

    #[test]
    fn split_notes_no_parent_returns_none() {
        let (visible, parent) = super::split_visible_notes_and_chain_parent(Some("Просто нотатка"));
        assert_eq!(visible, "Просто нотатка");
        assert!(parent.is_none());
    }

    #[test]
    fn split_notes_empty_input_returns_empty() {
        let (visible, parent) = super::split_visible_notes_and_chain_parent(None);
        assert_eq!(visible, "");
        assert!(parent.is_none());
    }

    #[test]
    fn split_notes_only_parent_gives_empty_visible() {
        let (visible, parent) =
            super::split_visible_notes_and_chain_parent(Some("[chain-parent:inv:uuid123]"));
        assert_eq!(visible, "");
        assert_eq!(parent.as_deref(), Some("inv:uuid123"));
    }

    #[test]
    fn compose_roundtrip_preserves_both_parts() {
        let composed =
            super::compose_notes_with_chain_parent("Примітка", Some("inv:some-uuid"));
        let (visible, parent) =
            super::split_visible_notes_and_chain_parent(composed.as_deref());
        assert_eq!(visible, "Примітка");
        assert_eq!(parent.as_deref(), Some("inv:some-uuid"));
    }

    #[test]
    fn compose_empty_visible_no_parent_returns_none() {
        assert!(super::compose_notes_with_chain_parent("", None).is_none());
        assert!(super::compose_notes_with_chain_parent("   ", None).is_none());
    }

    #[test]
    fn normalize_chain_kind_maps_aliases() {
        assert_eq!(super::normalize_chain_kind("act"), Some("act"));
        assert_eq!(super::normalize_chain_kind("invoice"), Some("invoice"));
        assert_eq!(super::normalize_chain_kind("inv"), Some("invoice"));
        assert_eq!(super::normalize_chain_kind("waybill"), Some("waybill"));
        assert_eq!(super::normalize_chain_kind("wbl"), Some("waybill"));
        assert_eq!(super::normalize_chain_kind("unknown"), None);
    }

    #[test]
    fn chain_kind_rank_orders_correctly() {
        let inv = super::chain_kind_rank("invoice").unwrap();
        let act = super::chain_kind_rank("act").unwrap();
        let wbl = super::chain_kind_rank("waybill").unwrap();
        assert!(inv < act, "invoice має бути першим у ланцюжку");
        assert!(act < wbl, "act має бути перед waybill");
        assert!(super::chain_kind_rank("bad").is_none());
    }

    #[test]
    fn can_create_chain_target_enforces_order() {
        assert!(super::can_create_chain_target("invoice", "act"));
        assert!(super::can_create_chain_target("invoice", "waybill"));
        assert!(super::can_create_chain_target("act", "waybill"));
        assert!(!super::can_create_chain_target("act", "invoice"));
        assert!(!super::can_create_chain_target("waybill", "act"));
        assert!(!super::can_create_chain_target("act", "act"));
    }
```

- [x] **Step 2: Запустити тести**

```bash
cargo test --lib 2>&1 | grep -E "test.*ok|test.*FAILED|error"
```
Expected: всі нові тести `ok`, без `FAILED`.

- [x] **Step 3: Commit**

```bash
git add src/ui/documents.rs
git commit -m "test: add unit tests for chain pure functions (split/compose notes, normalize kind, rank, can_create)"
```

---

## Self-Review

- [x] Critical #1 (cycle detection) — Task 2
- [x] Critical #2 (error swallowing) — Task 1
- [x] Important #3 (remove legacy) — Task 3
- [x] Important #4 (remove prefill) — Task 3
- [x] Important #5 (duplicate DB call) — Task 4
- [x] Important #6 (string comparison) — не потребує змін: `normalize_chain_kind` гарантує lowercase на обох сторонах; додаткова explicit нормалізація на `step.doc_type` ускладнила б код без реальної вигоди поки нема enum
- [x] Important #7 (unit tests) — Task 6
- [x] Minor #8 (pub visibility) — Task 5
