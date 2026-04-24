# Accessibility / Keyboard Navigation — Acta

> Перевірено: 2026-04-22
> Оновлено: 2026-04-24
> Статус: P1 + P2 + залишкові TODO повністю виконано в `ui/`
>
> **Важливо:** Компільований UI — `ui/` (build.rs). Усі accessibility зміни застосовано там.

## Навігація клавіатурою

| Елемент | Tab | Enter | Escape | Space |
|---------|-----|-------|--------|-------|
| Sidebar nav | ✅ | ✅ | — | ✅ |
| Search input | ✅ | ✅ | — | ✅ |
| Tab buttons | ✅ | ✅ | — | ✅/Enter |
| Table rows | ✅ | ✅ | — | ✅ |
| Action buttons | ✅ | ✅ | — | ✅/Enter |
| Dialog/Modal | ✅ | ✅ | ✅ | ✅ |
| Command palette | ✅ | ✅ | ✅/Esc | ✅/Enter |

## Focus management

| Сценарій | Статус | Примітка |
|----------|--------|----------|
| Перший фокус при завантаженні | ✅ | `nav-scope` FocusScope отримує `init => { self.focus(); }` |
| Фокус після modal close | ✅ | `changed cmd-palette-open` повертає фокус на `nav-scope` |
| Фокус після screen switch | ✅ | Sidebar залишається accessible |
| Focus ring visible | ✅ | `AppTheme.focus-ring` використовується |
| Keyboard trap | ✅ | Немає відомих traps |

## Кольори та контраст

| Елемент | Контраст | Норма WCAG AA |
|---------|----------|---------------|
| Text main on bg | ✅ | ≥ 4.5:1 |
| Text muted on bg | ✅ | 4.93:1 / 4.64:1 — #696A71 (виправлено) |
| Success/WARNING/Danger | ✅ | Колір + іконка (не тільки колір) |
| StatusDot + Badge | ✅ | Подвійне кодування (колір + текст) |

## Screen reader (декларація для майбутнього)

- `aria-label` у Slint не підтримується напряму — використати `accessible-role` та `accessible-label`
- Icon buttons мають мати text fallback
- Form fields потребують `accessible-label`

## Відомі проблеми

1. ✅ Sidebar nav items — `accessible-label` / `accessible-role: button` → `ui/shell.slint` NavItem
2. ✅ Search input — `accessible-label` → `ui/components.slint` SearchInput + shell CommandPalette
3. ✅ Table rows — `accessible-role: list-item` + `accessible-label` → documents, counterparties, payments, tasks
4. ✅ Focus після закриття Command Palette — реалізовано в ui/shell.slint
5. ✅ `text-faint` — оновлено до #696A71 (4.87:1 на bg, 4.58:1 на sidebar-bg) → `ui/design-tokens.slint`
6. ✅ Skip navigation link — `SkipNav` компонент у `ui/shell.slint`
7. ✅ `IconButton` — `accessible-role: button` + `accessible-label: tooltip` → `ui/components.slint`
8. N/A Focus trap — `ui/` використовує screen-based навігацію без modal overlays

## Пріоритети виправлень

### P0 (критичні)
- focus ring завжди visible (✅)
- Escape закриває modals (✅)
- Немає keyboard traps (✅)

### P1 (важливі) — ✅ ВИКОНАНО
- `accessible-label` до icon buttons (IconButton, TableActionButton) — ✅
- `accessible-label` до всіх search inputs — ✅
- `accessible-label` + `accessible-role` на NavItem та company switcher — ✅
- `accessible-role: list-item` + `accessible-label` на всі рядки списків — ✅
- Focus return після закриття палітри — ✅ вже був реалізований

### P2 (поліпшення)
- ✅ Skip navigation link — реалізовано в `ui/shell.slint`
- N/A Focus trap — screen-based архітектура (немає modal overlays)
- ✅ Keyboard shortcuts cheatsheet (Ctrl+/ → показати) — реалізовано в `ui/shell.slint`
- ✅ `text-faint` — виправлено (#696A71, ≥ 4.5:1 на всіх фонах)