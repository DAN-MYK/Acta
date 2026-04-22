# Accessibility / Keyboard Navigation — Acta

> Перевірено: 2026-04-22
> Оновлено: 2026-04-22
> Статус: P1 виконано — accessible-label/role додані; залишаються P2

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
| Перший фокус при завантаженні | ❌ TODO | Sidebar → перший nav item має отримати focus |
| Фокус після modal close | ❌ TODO | Повертати фокус на trigger елемент |
| Фокус після screen switch | ✅ | Sidebar залишається accessible |
| Focus ring visible | ✅ | `AppTheme.focus-ring` використовується |
| Keyboard trap | ✅ | Немає відомих traps |

## Кольори та контраст

| Елемент | Контраст | Норма WCAG AA |
|---------|----------|---------------|
| Text main on bg | ✅ | ≥ 4.5:1 |
| Text muted on bg | ⚠️ | Може бути < 4.5:1 — перевірити |
| Success/WARNING/Danger | ✅ | Колір + іконка (не тільки колір) |
| StatusDot + Badge | ✅ | Подвійне кодування (колір + текст) |

## Screen reader (декларація для майбутнього)

- `aria-label` у Slint не підтримується напряму — використати `accessible-role` та `accessible-label`
- Icon buttons мають мати text fallback
- Form fields потребують `accessible-label`

## Відомі проблеми

1. ✅ Sidebar nav items — `accessible-label` / `accessible-role: button` додано
2. ✅ Search input — `accessible-label` додано (всі 7 списків + CommandPalette)
3. ✅ Table rows — `accessible-role: list-item` + `accessible-label` додано (всі 6 списків)
4. ✅ Focus після закриття Command Palette — вже реалізовано `fs-global.focus()` у app.slint
5. ⚠️ `Text muted` може не проходити WCAG AA contrast — потребує перевірки
6. ❌ Немає skip-link для keyboard users

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
- Skip navigation link
- Focus trap всередині діалогів
- Keyboard shortcuts cheatsheet (Cmd+/ → показати)
- Перевірити `Text muted` на WCAG AA контраст (≥ 4.5:1)