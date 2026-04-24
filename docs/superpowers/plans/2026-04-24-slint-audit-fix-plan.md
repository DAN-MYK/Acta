# План виправлення проблем Slint UI після аудиту

Дата: 2026-04-24

## Контекст

Цей план покриває findings з аудиту Slint-коду Acta:

1. `ui/documents.slint`: вибір одного документа підсвічує всі рядки.
2. `ui/documents.slint`: пагінація може віддавати сторінки за межами діапазону.
3. `ui/app.slint`: частина callbacks з `Documents` не форвардиться в `AppWindow`.
4. `ui/app.slint`: `Dashboard` має властивості, які `AppWindow` не передає.
5. `ui/documents.slint`: document tabs мають неповний data contract.
6. `ui/settings.slint`: preview-картки тем мають локальний hardcode і дублювання.

Поточний технічний блокер перевірки: `cargo check` доходить до Rust і падає в
`src/bootstrap.rs:422` через передачу `ModelRc<PaletteItemData>` у
`upgrade_in_event_loop`. Slint-генерація при цьому проходить.

## Принципи виправлення

- Зберегти Money Contract: фінансові значення в Slint тільки як pre-formatted `string`.
- Не переносити бізнес-логіку в Slint; Slint має відповідати за стан екрана, callbacks і layout.
- Для станів вкладок/фільтрів поступово рухатися від string values до enums у `types.slint`.
- Не залишати UI-дії без Rust callback або явного TODO logging у wiring.
- Виправляти малими кроками: P1 поведінка, потім data contracts, потім cleanup/refactor.

## Етап 0. Зафіксувати базовий стан

- [ ] Запустити `git status --short` і не перезаписувати сторонні локальні зміни.
- [ ] Запустити `cargo check` і зафіксувати актуальний Rust-блокер.
- [ ] Перед змінами переглянути:
  - `ui/app.slint`
  - `ui/documents.slint`
  - `ui/dashboard.slint`
  - `ui/settings.slint`
  - `ui/types.slint`
  - Rust UI wiring для документів і dashboard.

## Етап 1. P1: виправити вибір документів

### Проблема

У `ui/documents.slint` кожен рядок отримує:

```slint
selected: root.selected-ids.length > 0;
```

Через це вибір одного документа візуально позначає всі рядки.

### Рекомендоване рішення

- [ ] Додати в `DocumentItem` поле `selected: bool`.
- [ ] Формувати `selected` у Rust на основі реального selection state.
- [ ] У `DocRow` передавати:

```slint
selected: doc.selected;
```

- [ ] Переконатися, що checkbox state і row background беруть один і той самий selected source.
- [ ] Оновити Rust mapping/generated assignments для `DocumentItem`.

### Альтернатива

Якщо зміна Rust model небажана, можна зробити перевірку належності `doc.id` до
`selected-ids` у Slint. Але для стабільності й простоти контракту краще тримати
обчислення selection у Rust.

### Перевірка

- [ ] Додати або оновити UI contract test: вибір одного документа не позначає інші.
- [ ] Перевірити bulk bar: він показується тільки коли є selected rows.

## Етап 2. P1: захистити пагінацію

### Проблема

Кнопки previous/next викликають:

```slint
root.page-changed(root.page - 1);
root.page-changed(root.page + 1);
```

без guard-умов.

### Рішення

- [ ] Додати локальні properties у `Documents`:

```slint
property <bool> can-prev-page: root.page > 1;
property <bool> can-next-page: root.page < root.total-pages;
```

- [ ] Для previous:
  - opacity нижча, якщо `!can-prev-page`;
  - hover background тільки якщо `can-prev-page`;
  - callback викликається тільки якщо `can-prev-page`.
- [ ] Для next:
  - opacity нижча, якщо `!can-next-page`;
  - hover background тільки якщо `can-next-page`;
  - callback викликається тільки якщо `can-next-page`.
- [ ] Врахувати `total-pages <= 1`: обидві кнопки мають бути disabled.

### Перевірка

- [ ] UI не викликає `page-changed(0)`.
- [ ] UI не викликає `page-changed(total-pages + 1)`.
- [ ] На першій/останній сторінці відповідна кнопка візуально disabled.

## Етап 3. P2: дофорвардити Documents callbacks у AppWindow

### Проблема

`Documents` оголошує callbacks:

- `selection-cleared`
- `more-actions(string)`
- `bulk-send`
- `bulk-archive`
- `bulk-delete`

але `AppWindow` не форвардить їх у Rust.

### Рішення

- [ ] У `ui/app.slint` додати root callbacks:

```slint
callback doc-selection-cleared;
callback doc-more-actions(string);
callback doc-bulk-send;
callback doc-bulk-archive;
callback doc-bulk-delete;
```

- [ ] У блоці `Documents { ... }` додати forwarding:

```slint
selection-cleared => { root.doc-selection-cleared(); }
more-actions(id) => { root.doc-more-actions(id); }
bulk-send => { root.doc-bulk-send(); }
bulk-archive => { root.doc-bulk-archive(); }
bulk-delete => { root.doc-bulk-delete(); }
```

- [ ] Вирішити долю наявного `doc-delete(string)`:
  - або підключити до конкретної дії видалення одного документа;
  - або замінити на `doc-bulk-delete`, якщо single delete більше не використовується.
- [ ] У Rust wiring додати handlers для нових callbacks.
- [ ] Якщо поведінка ще не реалізована, використовувати `tracing::info!("TODO: ...")`, а не мовчазний no-op.

### Перевірка

- [ ] UI callback tests перевіряють clear selection, more actions і bulk actions.
- [ ] Натискання кнопок не губиться між Slint і Rust.

## Етап 4. P2: вирівняти Dashboard data contract

### Проблема

`Dashboard` має властивості:

- `accounts-total`
- `open-task-count`
- `delta-revenue-str`
- `delta-expenses-str`
- `delta-net-str`
- `ytd-total-str`
- `ytd-revenue-str`
- `ytd-expenses-str`
- `spark-revenue`
- `spark-expenses`

але `AppWindow` передає лише частину dashboard даних.

### Рішення

- [ ] Перевірити `DashboardViewData` у `ui/types.slint`.
- [ ] Додати відсутні поля в `DashboardViewData`, якщо їх там немає.
- [ ] Оновити Rust struct/mapping для `DashboardViewData`.
- [ ] У `ui/app.slint` передати всі поля в `Dashboard { ... }`.
- [ ] Якщо частина значень ще не має реального джерела, формувати чесні placeholder/default значення в Rust.

### Перевірка

- [ ] Dashboard UI не залежить від implicit Slint defaults для видимих даних.
- [ ] Contract test перевіряє, що apply-функція заповнює всі поля, які UI очікує.
- [ ] Money values лишаються `string`; sparkline values лишаються normalized `[float]`.

## Етап 5. P2: вирівняти Documents tabs contract

### Проблема

`Documents` має props:

- `invoice-docs`
- `act-docs`
- `waybill-docs`

але `AppWindow` передає тільки `all-docs`. Також:

```slint
property <[DocumentItem]> visible-rows: all-docs;
```

не залежить від активної вкладки.

### Рішення

- [ ] Додати у `DocumentsViewData` окремі списки:
  - `invoice-items: [DocumentItem]`
  - `act-items: [DocumentItem]`
  - `waybill-items: [DocumentItem]`
- [ ] У Rust формувати ці списки разом із `items`.
- [ ] У `ui/app.slint` передати:

```slint
invoice-docs: root.documents.invoice-items;
act-docs: root.documents.act-items;
waybill-docs: root.documents.waybill-items;
```

- [ ] У `ui/documents.slint` зробити `visible-rows` залежним від `active-tab`:

```slint
property <[DocumentItem]> visible-rows:
    active-tab == "invoice" ? invoice-docs :
    active-tab == "act"     ? act-docs     :
    active-tab == "waybill" ? waybill-docs :
    all-docs;
```

- [ ] Перевірити, що лічильники вкладок і список показують один і той самий source.

### Подальший рефакторинг

- [ ] Замінити string tabs на enum `DocumentTab`.
- [ ] Перевести `tab-changed(string)` на enum callback або зробити тимчасовий adapter у `app.slint`.

### Перевірка

- [ ] Вкладка "Усі" показує всі документи.
- [ ] Вкладка "Рахунки" показує тільки рахунки.
- [ ] Вкладка "Акти" показує тільки акти.
- [ ] Вкладка "Видаткові накладні" показує тільки накладні.
- [ ] Counts вкладок збігаються з кількістю рядків.

## Етап 6. P3: прибрати hardcode у theme previews

### Проблема

У `ui/settings.slint` preview-картки світлої й темної теми мають локальні hex-кольори
і майже однакову структуру.

### Мінімальне рішення

- [ ] Створити локальний компонент `ThemePreviewCard`.
- [ ] Передавати в нього:
  - `label`
  - `active`
  - `preview-bg`
  - `preview-sidebar`
  - `preview-surface`
  - `preview-line-a`
  - `preview-line-b`
  - `clicked`
- [ ] Замінити два дубльовані блоки одним компонентом з різними параметрами.

### Краще рішення

- [ ] Винести preview palette у `design-tokens.slint` або окремий `ThemePreviewTokens`.
- [ ] Залишити в `settings.slint` тільки композицію UI, без raw hex.

### Перевірка

- [ ] Preview світлої теми виглядає світлою навіть у dark mode.
- [ ] Preview темної теми виглядає темною навіть у light mode.
- [ ] Зміна теми все ще викликає `dark-mode-toggled(bool)`.

## Етап 7. Cleanup: мертвий код і дублювання

- [ ] `BulkBar` у `ui/components.slint`:
  - або використати в `ui/documents.slint`;
  - або видалити, якщо shared component не потрібен.
- [ ] `MonoNumber`:
  - прибрати `float value`, `show-sign`, `decimals`, якщо компонент показує тільки `formatted`;
  - залишити тільки string-based API для Money Contract.
- [ ] Placeholder icons в `ui/app.slint`:
  - додати реальні `Bell.svg`, `Sun.svg`, `Moon.svg`, `Star.svg`, `ChevronDown.svg`;
  - підключити їх у `ui/icons.slint`;
  - замінити тимчасові `Icons.search`, `Icons.uah`, `Icons.sort`.
- [ ] Hardcoded saved filters у `ui/shell.slint`:
  - додати struct `SavedViewData`;
  - додати список у `ShellChrome` або окремий property;
  - рендерити saved filters через `for`.
- [ ] Hover state:
  - замінити прості `property <bool> hovered` на `TouchArea.has-hover`, де це не погіршує читабельність.

## Етап 8. String state -> enums

### Кандидати на enums

- [ ] `DocumentTab { All, Invoice, Act, Waybill }`
- [ ] `TaskFilter { Open, Done, All }`
- [ ] `CounterpartyTab { Docs, Payments, Contracts, Details, Activity }`
- [ ] `SettingsSection { Appearance, Company, Numbering, Integrations, Team, Backup }`
- [ ] `ChainDocType { Invoice, Act, Waybill }`
- [ ] `ChainStatus { Draft, Issued, Signed, Paid, Overdue, Partial, Missing }`

### Порядок переходу

1. Додати enum у `types.slint`.
2. Перевести внутрішній UI state на enum.
3. Тимчасово лишити Rust callbacks string-based, якщо Rust ще не готовий.
4. Після оновлення Rust wiring перевести callbacks на enum.
5. Видалити string adapters.

## Етап 9. Фінальна перевірка

- [ ] `cargo check`
- [ ] `cargo test`
- [ ] UI contract tests для:
  - single document selection;
  - pagination boundaries;
  - document callbacks forwarding;
  - dashboard data propagation;
  - document tab counts and visible rows.
- [ ] Якщо змінювали Rust SQL/data layer, перевірити релевантні sqlx кроки.
- [ ] Візуально переглянути Documents, Dashboard, Settings.

## Рекомендований порядок комітів

1. `fix: repair document selection and pagination`
2. `fix: complete documents and dashboard ui contracts`
3. `refactor: reduce slint hardcode and dead shared code`
4. `refactor: introduce typed slint navigation state`

Такий порядок відділяє поведінкові баги від рефакторингу і робить ревʼю значно простішим.
