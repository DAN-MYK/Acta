# AI Implementation Prompt — UI Stabilization, Architecture Cleanup, and Accessibility Pass

> **Призначення:** цей файл є готовою інструкцією-промптом для іншого ШІ-агента, який має реалізувати весь пул технічних питань, виявлених під час аудиту Rust + Slint проєкту `Acta`.
>
> **Мова роботи, коментарів, повідомлень у коді та звітів:** українська.
>
> **Режим роботи:** працюй як senior Rust/Slint engineer, який не просто описує зміни, а доводить їх до завершеного технічного результату з перевіркою збірки, тестів і архітектурної цілісності.

---

## 1. Контекст проєкту

- Проєкт: `Acta`
- Тип: desktop app для управлінського обліку українського бізнесу
- Стек:
  - Rust
  - Slint
  - PostgreSQL + sqlx
  - tokio
  - rust_decimal
- Поточний стан:
  - новий UI живе в `ui-redesign/`
  - `build.rs` уже компілює `ui-redesign/app.slint`
  - старі UI-тести більше не відповідають поточному Slint contract
  - у UI-contract ще є бізнес-критичні `float` для money-facing даних
  - orchestration частково живе в `src/main.rs`
  - є незавершені placeholder/no-op сценарії

---

## 2. Головна мета

Ти маєш реалізувати повний технічний пул покращень, щоб привести проєкт до більш сучасного і підтримуваного стану для Rust + Slint desktop app.

Після завершення роботи система має відповідати таким умовам:

1. `ui-redesign` є єдиним канонічним UI.
2. UI test suite працює проти нового Slint contract.
3. Бізнес-критичні грошові значення не передаються в Slint як `float`, окрім суто chart/render use cases.
4. `main.rs` не містить зайвої orchestration-логіки.
5. Є один канонічний підхід до state management.
6. Незавершені interactive сценарії не лишаються мовчазними no-op.
7. Після стабілізації redesign виконано окремий accessibility/keyboard navigation pass.
8. Усі зміни задокументовані, перевірені та зведені до консистентного архітектурного канону.

---

## 3. Обов'язкові правила реалізації

### 3.1 Загальні

- Не переписуй проєкт із нуля.
- Працюй ітеративно, але доводь кожен етап до compile-ready стану.
- Не ламай існуючий доменний шар без необхідності.
- Якщо є сумнів між “швидко заліпити” і “узгодити контракт правильно”, обирай правильний контракт.

### 3.2 Rust

- Для грошей використовуй `rust_decimal::Decimal`.
- Не використовуй `f32`/`f64` для фінансової логіки.
- Для помилок використовуй `anyhow::Result`.
- Не використовуй `.unwrap()` у production path.
- Для дат використовуй `chrono::NaiveDate`.
- Для UUID використовуй `uuid::Uuid`.

### 3.3 Slint

- UI-логіка має залишатися в `.slint`.
- Бізнес-логіка має бути в Rust.
- Не форматуйте фінансові display values у Slint, якщо це можна підготувати в Rust.
- По можливості прибирай stringly-typed contract і переходь на enum-based або структурований підхід.

### 3.4 Тести

- Не залишай “зелену збірку” як єдине свідчення коректності.
- Віднови або перепиши UI tests під актуальний contract.
- Додавай unit tests для mapping/presenter/formatting logic.
- Якщо старий тест більше не має сенсу, заміни його новим, а не просто видали без покриття.

---

## 4. Стратегія виконання

Виконуй роботу строго в такій черговості:

1. Канонізувати `ui-redesign`.
2. Відновити UI test safety net.
3. Очистити money contract у Rust/Slint.
4. Консолідувати application state і прибрати orchestration із `main.rs`.
5. Прибрати крихкий `Mutex + unwrap` з UI flow.
6. Декомпозувати `AppWindow` там, де це дає реальну архітектурну користь.
7. Усунути placeholder/no-op interaction patterns.
8. Нормалізувати Rust/Slint contracts.
9. Посилити presenter/mapping layer.
10. Виконати окремий accessibility/keyboard navigation pass.
11. Оновити документацію.

Не перестрибуй одразу до косметики, якщо не закриті compile/test/contract ризики.

---

## 5. Implementation Backlog

### Epic 1. Канонізувати `ui-redesign` як єдиний UI

#### Task 1.1. Зафіксувати `ui-redesign` як єдине джерело істини

**Ціль:**
- прибрати двозначність між старим `ui/` і новим `ui-redesign/`

**Acceptance Criteria:**
1. У збірці та документації зафіксовано, що `ui-redesign` є канонічним UI.
2. Не залишається активних runtime assumptions про старий UI.
3. Legacy-посилання або ізольовані, або прибрані.

#### Task 1.2. Провести inventory legacy UI-contract

**Ціль:**
- знайти старі generated type assumptions, callback names, test dependencies

**Acceptance Criteria:**
1. Є список legacy UI artifacts.
2. Для кожного artifact визначено: `remove`, `migrate`, `keep temporarily`.

#### Task 1.3. Очистити або ізолювати legacy UI artifacts

**Acceptance Criteria:**
1. Старий `MainWindow` не використовується як очікуваний current UI binding.
2. Legacy UI не створює хибного test/runtime signal.

---

### Epic 2. Відновити UI test safety net

#### Task 2.1. Переписати `tests/ui_events.rs` під новий Slint contract

**Acceptance Criteria:**
1. Тест використовує актуальний generated root component.
2. `cargo test --test ui_events --no-run` проходить.
3. Тест не очікує старого `MainWindow`.

#### Task 2.2. Покрити основні interaction contracts

Мінімально покрити:
- navigation callbacks
- documents callbacks
- tasks callbacks
- settings/theme callbacks
- command palette callbacks
- inbox callbacks

**Acceptance Criteria:**
1. Основні callback-и нового UI wired і перевірені тестами.
2. Regression по wiring ловиться автоматично.

#### Task 2.3. Додати test helpers для Slint UI

**Acceptance Criteria:**
1. Менше boilerplate у UI tests.
2. Легко додавати нові interaction tests.

---

### Epic 3. Очистити фінансовий UI-contract

#### Task 3.1. Провести аудит money fields у `ui-redesign/types.slint`

**Ціль:**
- відокремити money display fields від chart/render numeric fields

**Acceptance Criteria:**
1. Є список усіх money-related `float` полів.
2. Для кожного визначено новий безпечний contract.

#### Task 3.2. Замінити business-facing `float` поля на безпечні типи

**Очікування:**
- display sums, balances, KPI amounts, totals мають іти в Slint як `string` або інший стабільний display-safe формат
- `float` дозволений тільки для chart geometry / normalized render values

**Acceptance Criteria:**
1. Бізнес-критичні суми більше не передаються як `float`.
2. Усі екрани лишаються візуально коректними.

#### Task 3.3. Оновити Rust mapper/formatter layer

**Acceptance Criteria:**
1. Немає blanket conversion `Decimal -> f32` для display values.
2. Є окремі formatter-и для money display.
3. Є окремі normalizer-и для charts.

#### Task 3.4. Додати тести на formatting/mapping грошей

**Acceptance Criteria:**
1. Є unit tests на formatters.
2. Є unit tests на presenter mapping для KPI / documents / payments.

---

### Epic 4. Консолідувати application state

#### Task 4.1. Вибрати канонічний state container

**Варіанти:**
- розвинути `AppCtx`
- або ввести більш чіткий `AppController` / `AppState`

**Acceptance Criteria:**
1. Є одне задокументоване рішення.
2. Немає паралельних state-management підходів.

#### Task 4.2. Перенести orchestration з `main.rs`

**Acceptance Criteria:**
1. `main.rs` виконує тільки bootstrap і старт застосунку.
2. Loading, refresh, wiring coordination не живуть у `main.rs`.

#### Task 4.3. Уніфікувати screen state + refresh methods

**Acceptance Criteria:**
1. Є централізовані refresh methods для screen data.
2. Active company, filters, loaded state читаються через один канонічний API.

---

### Epic 5. Прибрати крихкий shared mutable state з UI flow

#### Task 5.1. Позбутися прямого `lock().unwrap()` у UI callbacks

**Acceptance Criteria:**
1. Немає panic-prone direct lock access у production UI flow.
2. Shared state читається через safe accessor або snapshot pattern.

#### Task 5.2. Мінімізувати `Arc<Mutex<...>>`

**Acceptance Criteria:**
1. Shared mutable state використовується тільки там, де справді потрібен.
2. Лишні mutex-based точки координації прибрані.

---

### Epic 6. Декомпозувати `AppWindow`

#### Task 6.1. Зменшити surface area top-level root component

**Acceptance Criteria:**
1. `AppWindow` не перетворюється на god component.
2. Глобальний shell state відокремлено від screen-specific state.

#### Task 6.2. Виділити feature-specific contracts/view models

**Acceptance Criteria:**
1. Dashboard / Documents / Counterparties / Payments / Tasks / Settings мають логічно згрупований contract.
2. Менше flat property clutter.

---

### Epic 7. Прибрати placeholder/no-op поведінку

#### Task 7.1. Зробити inventory усіх no-op callback-ів і placeholder-ів

**Acceptance Criteria:**
1. Є список незавершених interactive сценаріїв.
2. Для кожного вибрано статус: `implemented`, `disabled`, `hidden`, `stub with explicit marker`.

#### Task 7.2. Замінити мовчазні no-op на явний UX

**Acceptance Criteria:**
1. Користувач не може безслідно натиснути на “мертву” дію.
2. Незавершені сценарії або вимкнені, або явно позначені.

#### Task 7.3. Прибрати placeholder icons / fallback visual hacks

**Acceptance Criteria:**
1. Placeholder visual semantics не вводять в оману.
2. Якщо іконка тимчасова, це або виправлено, або елемент приховано.

---

### Epic 8. Нормалізувати Rust/Slint contracts

#### Task 8.1. Прибрати stringly-typed state там, де можливі enum-и

**Acceptance Criteria:**
1. Статуси, типи, фільтри та інші структуровані поля не живуть як магічні рядки без потреби.
2. Зменшено кількість fragile string comparisons.

#### Task 8.2. Уніфікувати naming callback-ів і property

**Acceptance Criteria:**
1. Naming консистентний між Rust і Slint.
2. Legacy/new naming style не змішані хаотично.

---

### Epic 9. Посилити presenter/mapping layer

#### Task 9.1. Розділити loading, mapping, apply-to-ui

**Acceptance Criteria:**
1. У feature modules немає хаотичного змішування fetch + transform + UI mutation.
2. Pure mapping code можна тестувати окремо.

#### Task 9.2. Додати unit tests на presentation semantics

Мінімально покрити:
- status mapping
- derived display fields
- sorting/filtering/paging
- empty states

**Acceptance Criteria:**
1. Presenter layer має своє test coverage.
2. Contract changes ловляться без ручного UI smoke only.

---

### Epic 10. Accessibility та keyboard navigation pass

> **Цей блок обов'язковий після стабілізації `ui-redesign`.**

#### Task 10.1. Провести accessibility audit нового UI

**Acceptance Criteria:**
1. Перевірені всі ключові екрани `ui-redesign`.
2. Задокументовані проблеми з focus, tab order, key actions, focus visibility.

#### Task 10.2. Налаштувати keyboard navigation

Перевірити і виправити:
- tab order
- Enter / Space / Escape behavior
- keyboard usability форм
- keyboard usability command palette
- keyboard usability навігації між екранами

**Acceptance Criteria:**
1. Всі ключові interactive елементи reachable з клавіатури.
2. Немає keyboard traps.
3. Escape/Enter/Space працюють передбачувано.

#### Task 10.3. Перевірити focus/hover/active states

**Acceptance Criteria:**
1. Focus states видимі і зрозумілі.
2. Після screen switch / modal close фокус не губиться.
3. Keyboard user не лишається без візуального контексту.

#### Task 10.4. Додати keyboard/accessibility regression tests, де це можливо

**Acceptance Criteria:**
1. Критичні keyboard flows покриті тестами.
2. Accessibility regressions не проходять непоміченими.

---

### Epic 11. Оновити документацію

#### Task 11.1. Зафіксувати новий архітектурний канон

Описати:
- який UI канонічний
- як працює state management
- як передаються дані Rust -> Slint
- як працює money contract
- як писати нові UI tests

**Acceptance Criteria:**
1. Архітектурне рішення задокументоване.
2. Майбутній розробник може продовжити роботу без reverse engineering.

#### Task 11.2. Оновити інструкції/нотатки, якщо змінені правила

**Acceptance Criteria:**
1. Документація в репозиторії узгоджена з фактичною архітектурою.
2. Немає застарілих canonical instructions про UI/state.

---

## 6. Пріоритети

### P0
- Epic 1
- Epic 2
- Epic 3

### P1
- Epic 4
- Epic 5
- Epic 7

### P2
- Epic 6
- Epic 8
- Epic 9

### P3
- Epic 10
- Epic 11

---

## 7. Рекомендований порядок по спринтах

### Sprint 1
- Epic 1
- Epic 2

### Sprint 2
- Epic 3
- частина Epic 4

### Sprint 3
- завершення Epic 4
- Epic 5
- Epic 7

### Sprint 4
- Epic 6
- Epic 8
- Epic 9

### Sprint 5
- Epic 10
- Epic 11
- stabilization pass

---

## 8. Обов'язковий workflow для ШІ

Працюй за таким алгоритмом:

1. Спочатку зчитай поточний стан файлів, які збираєшся змінювати.
2. Не роби великий blind rewrite без локального контексту.
3. Перед серйозним refactor зроби короткий execution plan.
4. Внось зміни невеликими логічними порціями.
5. Після кожного значущого етапу запускай релевантну перевірку:
   - `cargo check`
   - `cargo test --test ... --no-run`
   - `cargo test`
6. Якщо зламався старий тест, виріши:
   - це legitimate regression
   - або test obsolete і його треба адаптувати
7. Не залишай роботу в напівстані “код написаний, але не зібраний”.

---

## 9. Команди перевірки

Використовуй принаймні ці команди:

```bash
cargo check
cargo test --test ui_events --no-run
cargo test --test unit_business_logic --no-run
cargo test
```

Якщо змінюється SQL:

```bash
cargo sqlx prepare
```

Якщо змінюються міграції:

```bash
sqlx migrate run
```

---

## 10. Definition of Done

Роботу можна вважати завершеною лише якщо одночасно виконані всі пункти:

1. `cargo check` проходить.
2. `cargo test` проходить або є чітко пояснені тимчасові винятки, узгоджені з реальним станом проєкту.
3. UI tests працюють проти `ui-redesign`.
4. Бізнес-критичні money values більше не течуть у Slint як `float`.
5. `main.rs` виконує роль bootstrap, а не application coordinator-god-file.
6. Є один канонічний state-management path.
7. Немає критичних мовчазних no-op interaction points.
8. Виконано accessibility/keyboard navigation pass.
9. Документація приведена у відповідність до нового стану.

---

## 11. Формат фінального звіту від ШІ

Після завершення роботи ШІ має повернути звіт у такій формі:

### 1. Що зроблено
- короткий список завершених Epic/Task

### 2. Які файли змінено
- список ключових файлів

### 3. Що перевірено
- які команди запускались
- що пройшло
- що не пройшло, якщо таке є

### 4. Які ризики лишилися
- тільки реальні незакриті ризики

### 5. Що рекомендується наступним кроком
- 1-3 конкретні технічні дії

---

## 12. Готовий короткий prompt для запуску іншого ШІ

Скопіюй і використай цей prompt як стартову інструкцію:

```text
Ти senior Rust + Slint engineer. Працюєш у репозиторії Acta. Реалізуй повний backlog із файлу docs/superpowers/plans/2026-04-21-ai-implementation-backlog-prompt.md.

Обов'язково:
1. Працюй українською.
2. Не обмежуйся аналізом — доводь зміни до compile/test-ready стану.
3. Канонічний UI — ui-redesign.
4. Віднови UI tests під новий Slint contract.
5. Прибери money-facing float contract із UI всюди, де це не chart-only.
6. Винеси orchestration із main.rs у канонічний state/controller layer.
7. Прибери критичні no-op interaction points.
8. Після стабілізації redesign виконай accessibility/keyboard navigation pass.
9. Після кожного великого етапу перевіряй cargo check / cargo test.

Почни з inventory поточного стану і працюй строго в порядку Epic 1 -> Epic 11.
```

