# Accessibility / Keyboard Navigation — Acta

> Перевірено: 2026-04-22
> Статус: базовий pass виконано, залишаються TODO

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

## Відомі проблеми (TODO)

1. ❌ Sidebar nav items — немає `accessible-label`
2. ❌ Search input — немає `accessible-label`
3. ❌ Table rows — не оголошуються як `row` з колонками
4. ❌ Focus не повертається після Закриття Command Palette
5. ⚠️ `Text muted` може не проходити WCAG AA contrast
6. ❌ Немає skip-link для keyboard users

## Пріоритети виправлень

### P0 (критичні)
- focus ring завжди visible (зараз ✅)
- Escape закриває modals (зараз ✅ в Shell)
- Немає keyboard traps (зараз ✅)

### P1 (важливі)
- Add `accessible-label` до icon buttons
- Focus return після modal/palette close
- Table announcements для screen readers

### P2 (поліпшення)
- Skip navigation link
- Focus trap всередині діалогів
- Keyboard shortcuts cheatsheet (Cmd+/ → показати)