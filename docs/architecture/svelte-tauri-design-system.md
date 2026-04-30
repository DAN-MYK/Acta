# Svelte/Tauri Design System

Оновлено: `2026-04-30`

## Призначення

Цей документ є канонічною design-system опорою для поточного Acta UI. Він замінює старі Slint token/component правила для live runtime.

## Канонічні файли

- Tokens: [frontend/src/lib/styles/tokens.css](/C:/Users/MykhailoDan/apps/Acta/frontend/src/lib/styles/tokens.css)
- Global layout/styles: [frontend/src/styles.css](/C:/Users/MykhailoDan/apps/Acta/frontend/src/styles.css)
- Root shell: [frontend/src/App.svelte](/C:/Users/MykhailoDan/apps/Acta/frontend/src/App.svelte)
- Screens: [frontend/src/lib/screens](/C:/Users/MykhailoDan/apps/Acta/frontend/src/lib/screens)
- Components: [frontend/src/lib/components](/C:/Users/MykhailoDan/apps/Acta/frontend/src/lib/components)
- Icons: [frontend/src/lib/icons](/C:/Users/MykhailoDan/apps/Acta/frontend/src/lib/icons)
- Icon rules: [docs/architecture/icon-style-guide.md](/C:/Users/MykhailoDan/apps/Acta/docs/architecture/icon-style-guide.md)

## Token contract

У Svelte компонентах використовувати CSS custom properties з `tokens.css`:

- spacing: `--space-*`;
- radius: `--radius-*`;
- surfaces: `--bg-*`;
- borders: `--border-*`;
- text: `--text-*`;
- semantic colors: `--success`, `--warning`, `--danger`, `--info`;
- accent: `--accent`, `--accent-soft`, `--accent-text`.

Нові hardcoded кольори допускаються тільки якщо вони стають кандидатом на новий token і документуються в цьому файлі або `tokens.css`.

## Component rules

- Reusable UI живе в `frontend/src/lib/components`.
- Screen-level composition живе в `frontend/src/lib/screens`.
- Store/business orchestration не пишеться всередині reusable component.
- Icons рендеряться через `AppIcon`, а не через inline SVG у screen markup.
- Money display бере `*Str` DTO поля, не рахує суми у Svelte.

## Screen rules

Кожен screen має:

- читати власний store через `$store`;
- показувати loading/error/message, якщо slice має такі states;
- викликати Tauri-backed дії через store methods;
- робити cross-slice navigation тільки там, де це описано в contract docs;
- мати component tests, якщо screen містить drill-in, editor, mutation action або conditional empty state.

## Testing contract

Ризикові screen-рівні покриваються Vitest + jsdom:

- root shell: bootstrap, theme wiring, company switch, command palette, keyboard shortcuts;
- dashboard: sections, empty state, drill-ins;
- documents: list/editor/chain actions;
- payments: KPI rows, editor open, reconciliation actions.
- settings: appearance controls, BAS import flow, company settings save.

Новий screen або суттєва зміна screen поведінки має додати тест у `frontend/src/lib/screens/__tests__`.
