# Accessibility / Keyboard Navigation — Acta

Оновлено: `2026-04-30`
Статус: базові вимоги виконані у Svelte UI

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
| Перший фокус при завантаженні | ✅ | sidebar nav отримує фокус |
| Фокус після modal close | ✅ | повертається на елемент що відкрив |
| Фокус після screen switch | ✅ | sidebar залишається accessible |
| Focus ring visible | ✅ | глобальні focus styles у CSS |
| Keyboard trap | ✅ | немає відомих traps |

## Кольори та контраст

| Елемент | Контраст | Норма WCAG AA |
|---------|----------|---------------|
| Text main on bg | ✅ | ≥ 4.5:1 |
| Text muted on bg | ✅ | 4.93:1 / 4.64:1 — #696A71 |
| Success/WARNING/Danger | ✅ | Колір + іконка (не тільки колір) |
| StatusDot + Badge | ✅ | Подвійне кодування (колір + текст) |

## Screen reader (HTML/Svelte)

- `aria-label` на icon buttons — обов'язково
- `role` на кастомних інтерактивних елементах
- Form fields потребують `<label>` або `aria-label`

## Keyboard shortcuts

| Shortcut | Дія |
|----------|-----|
| Ctrl+1..7 | навігація між screens |
| Ctrl+K | Command Palette |
| Ctrl+/ | shortcuts cheatsheet |
| Escape | закрити modal / palette |
