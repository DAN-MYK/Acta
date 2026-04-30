# Dashboard Migration Contract — 2026-04-30

## Рішення

Tauri dashboard у поточній міграції вважається **redesign-first перенесенням**, а не strict parity-копією Slint dashboard.

Мета цього рішення:

- не блокувати cutover на повному відтворенні старого dashboard 1:1;
- закріпити новий Tauri dashboard як корисний, backend-backed головний екран;
- уникнути плутанини між статусами “dashboard працює” і “dashboard повністю повторює Slint”.

## Що це означає

### Вважаємо перенесеним у Tauri

- backend-backed завантаження dashboard;
- KPI-блок у новому форматі;
- cashflow summary у новому форматі;
- recent acts;
- upcoming payments;
- urgent/focus tasks;
- переходи з dashboard у documents / payments / tasks;
- відкриття recent act у documents;
- відкриття upcoming payment у конкретний payment editor;
- відкриття urgent task у task editor.

### Вважаємо зміненим відносно Slint

- набір KPI не є 1:1 копією Slint;
- cashflow існує як новий data slice, а не як старий chart-first flow;
- tasks відображаються як список фокусних задач, а не як старий sidebar/task-control flow;
- recent acts не є заміною повного журналу операцій.

### Вважаємо свідомо виключеним із поточного Tauri dashboard contract

- перемикач `Огляд / Вхідні`;
- `Вхідні`;
- блок `Рахунки`;
- journal / таблиця операцій;
- `Усі типи` / `Всі операції`;
- task toggle/new task безпосередньо з dashboard;
- YTD / delta / sparklines;
- старий правий sidebar layout Slint dashboard.

## Наслідки для міграції

Tauri dashboard можна вважати **реалізованим як робочий screen**, але **не можна називати strict parity dashboard migration**.

Тобто:

- finding про placeholder dashboard вважається закритим;
- dashboard feature вважається реалізованою;
- parity зі Slint dashboard вважається частковим і навмисно зміненим.

## Якщо пізніше знадобиться strict parity

Тоді наступними окремими slices мають бути:

1. journal / таблиця операцій;
2. inbox flow;
3. accounts block;
4. dashboard-level task actions;
5. додаткові KPI/chart presentation details.

До цього моменту команда має трактувати поточний dashboard як **новий Tauri контракт**, а не як неповну копію Slint UI.

## Правило для документації

У roadmap, audit та contract matrix потрібно використовувати формулювання:

- `dashboard implemented`
- `dashboard parity partial by design`
- `redesign-first, not strict Slint parity`

Короткий запис для roadmap:

> Dashboard: реалізовано у Tauri як redesign-first screen; strict parity зі Slint не є поточною ціллю.
