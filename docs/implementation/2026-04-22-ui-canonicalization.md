## 2026-04-22: канонізація `ui-redesign` як єдиного UI

### Рішення

Канонічний UI Acta — це `ui-redesign/`.

- `build.rs` компілює тільки `ui-redesign/app.slint`.
- `src/main.rs` та `tests/ui_events.rs` працюють із generated contract від `ui-redesign`.
- Папка `ui/` не повинна сприйматись як активний runtime-шар.

### Inventory legacy UI artifacts

| Артефакт | Статус | Дія |
|---|---|---|
| `ui/` | legacy reference | `keep temporarily` |
| `ui/README.md` | guardrail | `keep` |
| `UI_ROADMAP.md` старі згадки про `ui/` як current UI | застаріло | `migrate` |
| коментарі/нотатки, де `ui/` названо поточним Slint UI | застаріло | `migrate` |
| runtime build path | уже на `ui-redesign` | `keep` |
| headless UI tests | уже на `AppWindow` із `ui-redesign` | `keep` |

### Що вважаємо завершеним у межах старту плану

1. Зафіксовано, що `ui-redesign` є єдиним джерелом істини для runtime UI.
2. Legacy-папка `ui/` більше не маскується під поточний UI.
3. Є окремий список legacy артефактів із рішенням: `keep temporarily`, `migrate`, `keep`.

### Що ще лишається на наступні кроки

1. Переписати застарілі нотатки та дорожні карти, де `ui/` ще фігурує як current UI.
2. Продовжити Epic 2 і вирівняти UI safety net під актуальний Slint contract.
3. Далі перейти до спрощення orchestration у `src/main.rs`.
