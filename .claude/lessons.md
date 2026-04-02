# Lessons — Уроки з помилок Claude

> Щоразу коли Claude зробив помилку і ти її виправив —
> додавай правило сюди щоб вона не повторилась.

## Формат запису
```
## [дата] Назва помилки
**Що сталось:** ...
**Правило:** ЗАВЖДИ/НІКОЛИ ...
```

---

## [2026-04-01] tokio::join! для паралельних async запитів
**Що сталось:** При відкритті форми акту послідовно виконувались два незалежних запити (counterparties_for_select та generate_next_number) через окремі `.await`.
**Правило:** ЗАВЖДИ використовувати `tokio::join!(fut1, fut2)` коли два async запити незалежні — виконуються паралельно і разом швидші. Sequential `.await` виправданий лише якщо результат першого потрібен для другого.

## [2026-04-01] Паттерн зчитування ModelRc через row_count/row_data
**Що сталось:** Спроба викликати `.row_count()` та `.row_data()` на `ModelRc<SharedString>` призводила до помилки компіляції "method not found".
**Правило:** ЗАВЖДИ імпортувати `use slint::Model;` для використання методів `row_count()` і `row_data()` на `ModelRc<T>`. Паттерн зчитування у Vec:
```rust
use slint::Model;
let v: Vec<SharedString> = (0..model.row_count())
    .filter_map(|i| model.row_data(i))
    .collect();
```

## [2026-04-01] Парсинг дати з UI рядка %d.%m.%Y
**Що сталось:** UI передає дату як рядок у форматі "ДД.ММ.РРРР". Потрібно надійно перетворити в `chrono::NaiveDate` та обробити невалідний ввід.
**Правило:** ЗАВЖДИ парсити дату з UI через `NaiveDate::parse_from_str(&s, "%d.%m.%Y")` і обробляти `Err` — повертати ранній вихід з `tracing::error!`, ніколи не `.unwrap()`. Формат `%d.%m.%Y` = "01.04.2026".

## [2026-04-01] cargo sqlx prepare після кожної нової sqlx::query!
**Що сталось:** Додавання нових `sqlx::query!` макросів (generate_next_number, counterparties_for_select) викликало помилку компіляції "set DATABASE_URL to use query macros online, or run cargo sqlx prepare".
**Правило:** ЗАВЖДИ запускати `DATABASE_URL=... cargo sqlx prepare` після додавання або зміни будь-якого `sqlx::query!` / `sqlx::query_as!` макросу. Нові `.sqlx/*.json` файли треба комітити разом з кодом.

## [2026-04-01] Паттерн оновлення позицій: DELETE + INSERT в транзакції
**Що сталось:** При реалізації update_with_items постало питання як оновити позиції — порівнювати старі з новими (diff) чи замінювати повністю.
**Правило:** ЗАВЖДИ використовувати паттерн "replace all" для дочірніх позицій при оновленні батьківського документу: `DELETE FROM act_items WHERE act_id = $1` → `INSERT` нових позицій в одній транзакції. Diff між старими і новими складніший, схильний до помилок і не дає переваг для документів управлінського обліку.

## [2026-04-01] Slint: Math.mod замість % і відсутній StandardListViewItem в std-widgets
**Що сталось:** Використання `i % 2` призводило до помилки "Unexpected '%'". Спроба `import { StandardListViewItem }` з `std-widgets.slint` — помилка "No exported type".
**Правило:** ЗАВЖДИ використовувати `Math.mod(i, 2)` для остачі в Slint (оператор `%` не підтримується). `StandardListViewItem` є глобальним типом Slint — НЕ імпортувати явно. `ListView` — навпаки, потребує `import { ListView } from "std-widgets.slint"`.

## [2026-04-02] query_as! macro vs runtime-style при зміні схеми БД
**Що сталось:** Після додавання `company_id` до таблиць, `sqlx::query_as!` макроси з новими параметрами не компілювались — "No connection could be made" бо `.sqlx` кеш застарів і БД недоступна.
**Правило:** Коли змінюється схема БД (нові стовпці, параметри запиту) і БД тимчасово недоступна — використовувати runtime-style `sqlx::query_as::<_, T>()` замість `query_as!`. Це рівноцінно за функціональністю але не потребує `.sqlx` кешу. Міграцію до `query_as!` зробити пізніше після `cargo sqlx prepare`.

## [2026-04-02] in-out vs in property у Slint для overlay flags
**Що сталось:** Спроба встановити `root.show-company-picker = false` всередині `.slint` файлу при `in property <bool>` дала помилку "Assignment on a input property".
**Правило:** ЗАВЖДИ використовувати `in-out property` для прапорців що керують overlay/modal в Slint — їх потрібно закривати як з Rust (`set_show_X(false)`), так і з Slint (кнопка Скасувати/×). `in property` — тільки read-only для Slint.

## [2026-04-02] Theme props у Slint — перевіряй theme.slint перед використанням
**Що сталось:** Використання `Theme.hover-bg` і `Theme.primary-light` дало помилки "does not have a property". Ці властивості відсутні в theme.slint.
**Правило:** ЗАВЖДИ читати `ui/theme.slint` перед написанням нових Slint компонентів щоб знати доступні кольори. Наявні: bg, surface, border, sidebar-bg, primary, text-main, text-sub, text-muted, success, warning, row-alt. Для "hover" використовувати `Theme.row-alt`, для "primary light" — inline hex (#e8f0fe).

## [2026-04-02] Arc<Mutex<Uuid>> для активної компанії в callbacks
**Що сталось:** Потрібно передавати змінний UUID активної компанії у десятки callbacks. AppState складний для Clone через PgPool.
**Правило:** Для спільного мутабельного стану між callbacks — `Arc<Mutex<T>>`. Паттерн:
```rust
let active_company_id_X = active_company_id.clone();
ui.on_callback(move |...| {
    let cid = *active_company_id_X.lock().unwrap();
    // використовуй cid
});
```

## [2026-04-02] self.enabled у TouchArea в Slint 1.9
**Що сталось:** `mouse-cursor: enabled ? pointer : default` всередині `TouchArea { enabled: ...; }` дало помилку "Unknown unqualified identifier 'enabled'" при повній збірці після `cargo clean`.
**Правило:** ЗАВЖДИ писати `self.enabled` для звернення до власних properties всередині елемента в Slint 1.9+. Спрощена форма `enabled` (без `self.`) не є unqualified посиланням — треба явний `self.`.

## [2026-04-02] function pointer cast не підходить для async fn у тестах
**Що сталось:** `let _ = fn_name as fn(&PgPool, Uuid) -> _;` в тестах db/invoices.rs давав "non-primitive cast" бо async функції повертають `impl Future`, а не конкретний тип.
**Правило:** У тестах перевірки що функція компілюється — писати просто `let _ = fn_name;` без cast. Для `db/acts.rs` cast раніше "працював" через кеш, але нестабільний. Використовуй `let _ = function_name;` без приведення типу.

## [2026-04-01] rsplit_once('-') для парсингу порядкових номерів
**Що сталось:** При генерації наступного номеру акту (generate_next_number) спокусливо використати `MAX(number)` як рядок. Але лексикографічний MAX дає "АКТ-2026-9" > "АКТ-2026-10" — неправильний результат.
**Правило:** НІКОЛИ не покладатись на лексикографічний MAX для рядків з числовим суфіксом. ЗАВЖДИ парсити числову частину: `s.rsplit_once('-').and_then(|(_, n)| n.parse::<u32>().ok())`, знаходити числовий максимум, і форматувати з нулями: `format!("{:03}", max + 1)`.
