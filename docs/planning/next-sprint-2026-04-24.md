# Next Sprint Plan

Оновлено: `2026-04-27`
Горизонт: `1-2 тижні`
Статус: `historical plan snapshot`

> Це плановий документ, сформований `2026-04-24`.
> Він фіксує задуманий scope спринту і не є джерелом правди про фактичне виконання.
> Актуальний підсумок див. у [sprint-report-2026-04-24.md](/C:/Users/MykhailoDan/apps/Acta/docs/planning/sprint-report-2026-04-24.md).

## Мета спринту

Закрити найбільш помітні user-facing дірки між уже наявним backend/UI і реально завершеним flow, не розпорошуючись на нові великі модулі на кшталт календаря, OCR чи повного блоку звітів.

## Sprint Outcome

Наприкінці спринту користувач мав би отримати:

- робочий CLI-потік імпорту даних з BAS замість заглушки
- завершені базові дії на екрані документів без критичних no-op callback-ів
- завершені основні flow для задач
- доведений до usable state flow імпорту банківських CSV і reconcile/unreconcile платежів

## Scope In

### 1. BAS Import MVP

Ціль: довести `src/bin/migrate.rs` від парсингу аргументів до реального імпорту.

#### Що входить

- читання вхідної директорії
- виявлення підтримуваних файлів експорту BAS
- базовий orchestration layer для імпорту
- dry-run режим без запису в БД
- logging прогресу імпорту
- базові інтеграційні або unit-тести на parsing/orchestration

#### Мінімальний definition of done

- `cargo run --bin migrate -- --input <dir>` виконує не лише parse args
- `--dry-run` проходить весь pipeline без запису в БД
- для unsupported/пошкоджених файлів є зрозуміле повідомлення
- є тести на happy path і щонайменше 2 failure cases

#### Ймовірні точки змін

- [migrate.rs](/C:/Users/MykhailoDan/apps/Acta/src/bin/migrate.rs)
- `src/import/`
- `tests/`

#### Ризики

- формат експорту BAS може бути неуніфікований
- може знадобитися окреме рішення для mapping документів у доменну модель `Acta`

### 2. Documents Flow Completion

Ціль: прибрати найбільш болючі stub callbacks на documents screen.

#### Що входить

- реалізувати `doc_new`
- реалізувати `doc_open`
- реалізувати `doc_edit`
- визначити мінімально корисну поведінку для `doc_more_actions`
- або реалізувати, або тимчасово свідомо прибрати з UI bulk-дії, які ще не підтримані
- реалізувати або чітко зафіксувати defer для `doc_chain_load` / `doc_chain_create`

#### Мінімальний definition of done

- на documents screen немає критичних головних дій, які лише логують `TODO`
- новий документ можна створити через доступний flow
- існуючий документ можна відкрити й відредагувати або через detail/edit screen, або через один канонічний edit flow
- якщо частина bulk-функцій не готова, вони не виглядають як робочі, але порожні

#### Ймовірні точки змін

- [documents.rs](/C:/Users/MykhailoDan/apps/Acta/src/ui/documents.rs)
- [bootstrap.rs](/C:/Users/MykhailoDan/apps/Acta/src/bootstrap.rs)
- `ui/documents.slint`
- пов'язані db/ui form модулі для актів, накладних, waybills

#### Ризики

- documents screen може тягнути кілька типів документів, тому edit/open flow треба уніфікувати
- chain flow може виявитися окремою mini-feature і не влізти повністю у спринт

### 3. Tasks Flow Completion

Ціль: закрити P1/P2 user-facing TODO для задач, які вже винесені в окрему нотатку.

#### Що входить

- реальний `new-task` flow
- `task-more` як details/edit flow
- реальне наповнення `day-events`
- увімкнення пошуку/фільтрації через `TaskListState.query`
- базове UI test coverage для головних task interactions

#### Мінімальний definition of done

- користувач може створити задачу не через stub, а через робочий UI flow
- `task-more` веде до корисної дії, а не в тупик
- day view показує реальні події або дедлайни
- пошук реально впливає на список задач
- UI/event тести покривають create/filter/open-flow

#### Ймовірні точки змін

- `src/ui/tasks.rs`
- `src/bootstrap.rs`
- `src/app_ctx.rs`
- `ui/tasks.slint`
- [ui-safety-net.md](/C:/Users/MykhailoDan/apps/Acta/docs/testing/ui-safety-net.md)
- `tests/ui_events.rs`

#### Ризики

- треба не порушити канонічний state path через `AppCtx`
- локальний UI state у Slint треба втримати окремо від shared runtime state

### 4. Payments Import/Reconcile Completion

Ціль: довести модуль платежів від "майже готово" до цілісного MVP flow.

#### Що входить

- окремий користувацький flow імпорту CSV
- wiring callback-ів для імпорту
- `unreconcile-payment`
- модальне або еквівалентне UI-рішення для reconcile
- базова валідація duplicate import / partial reconcile

#### Мінімальний definition of done

- користувач може імпортувати CSV із UI або явно підтриманого flow
- користувач може зіставити платіж з документом
- користувач може зняти зіставлення
- дублікати по `bank_ref` не ламають flow і дають передбачуваний результат

#### Ймовірні точки змін

- `src/import/bank_csv.rs`
- `src/db/payments.rs`
- `src/models/payment.rs`
- `src/ui/payments.rs`
- `ui/payments/`
- `tests/`

#### Ризики

- reconcile UX може вимагати окремого компонента в Slint
- різні банки можуть мати неоднорідні формати CSV

## Scope Out

У цей спринт не втягуємо:

- календар як окремий повний модуль
- OCR документів
- повний блок звітів
- редагування існуючих PDF через `lopdf`
- широку архітектурну перебудову поза потребами конкретних flow

## Пріоритет і порядок виконання

1. BAS Import MVP
2. Documents Flow Completion
3. Tasks Flow Completion
4. Payments Import/Reconcile Completion

## Чому саме такий порядок

- BAS import має найявнішу backend-заглушку і блокує інтеграційний сценарій
- documents screen має видимі no-op дії, які впливають на щоденне використання
- tasks already-almost-there і добре добиваються в одному спринті
- payments мають більший UI/UX хвіст, тому краще брати після закриття двох коротших потоків

## Розбиття на дні

### День 1-2

- уточнити формат sprint scope
- закрити design/contract рішення по BAS import
- почати реалізацію orchestration для `migrate`

### День 3-4

- завершити BAS import MVP
- додати тести
- перевірити dry-run та error handling

### День 5-6

- реалізувати documents `new/open/edit`
- прибрати або доробити misleading bulk actions
- вирішити долю `doc_chain_*` у межах MVP

### День 7-8

- довести tasks `new-task`, `task-more`, `day-events`
- додати пошук/фільтрацію
- оновити `tests/ui_events.rs`

### День 9-10

- доробити payments import flow
- reconcile/unreconcile
- тести на duplicate import і базовий partial reconcile

## Технічні правила спринту

- для кожної зміни спочатку фіксуємо контракт тестом там, де це можливо
- не створюємо другий state container поза `AppCtx`
- для грошей лишається canonical money contract: у Rust `Decimal`, у Slint pre-formatted `string`
- нові `TODO` допустимі лише як явно відкладені, некритичні й documented follow-up

## Acceptance Criteria

- у P1 scope немає нових user-facing кнопок, що ведуть лише до `TODO`
- `cargo test` або щонайменше релевантний піднабір тестів проходить
- sprint scope зафіксований у документації й не суперечить поточній архітектурі
- для кожного завершеного workstream є коротка нотатка, що саме вважати done

## Фактичний статус

Фактичне виконання цього плану виявилося частковим.
Зокрема, частина user-facing `TODO` у `documents` і заглушка в `src/bin/migrate.rs` на момент повторної звірки `2026-04-27` усе ще лишаються в коді.
Деталі й точні розбіжності див. у [sprint-report-2026-04-24.md](/C:/Users/MykhailoDan/apps/Acta/docs/planning/sprint-report-2026-04-24.md).

## Post-Sprint Candidates

- календар
- P&L / top counterparties / Excel export
- OCR
- PDF editing через `lopdf`

## Джерела

- [AGENTS.md](/C:/Users/MykhailoDan/apps/Acta/AGENTS.md)
- [Feature List.md](<C:/Users/MykhailoDan/OneDrive - UDPR/Obsidian/Mykhailo_Dan/development/Acta/Features/Feature List.md:1>)
- [Payments.md](<C:/Users/MykhailoDan/OneDrive - UDPR/Obsidian/Mykhailo_Dan/development/Acta/Features/Payments.md:1>)
- [Todo Feature.md](<C:/Users/MykhailoDan/OneDrive - UDPR/Obsidian/Mykhailo_Dan/development/Acta/Features/Todo Feature.md:1>)
- [app-state.md](/C:/Users/MykhailoDan/apps/Acta/docs/architecture/app-state.md)
