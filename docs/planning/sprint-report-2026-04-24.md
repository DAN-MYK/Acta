# Sprint Report

Оновлено: `2026-04-27`
План: [next-sprint-2026-04-24.md](/C:/Users/MykhailoDan/apps/Acta/docs/planning/next-sprint-2026-04-24.md)
Статус: `partial / not fully completed`

## Що це за файл

Це фактичний звіт по спринту, запланованому `2026-04-24`.
Статус нижче звірений безпосередньо з кодом репозиторію станом на `2026-04-27`, а не зі старими самооцінками в checklist.

## Підсумок

Спринт виконано частково.

Найважливіші розбіжності з планом:

- `BAS import MVP` не завершено: у [migrate.rs](/C:/Users/MykhailoDan/apps/Acta/src/bin/migrate.rs:74) досі є пряма заглушка `TODO`
- `Documents flow completion` не завершено: user-facing `doc_new`, `doc_open`, `doc_edit` та bulk-дії досі лишаються `TODO` у [documents.rs](/C:/Users/MykhailoDan/apps/Acta/src/ui/documents.rs:181)
- `Tasks` просунуті частково: `task_save` wired, але `task_more` лишається debug-only callback у [tasks.rs](/C:/Users/MykhailoDan/apps/Acta/src/ui/tasks.rs:217), а `day_events` усе ще заповнюється порожньою моделлю в [tasks.rs](/C:/Users/MykhailoDan/apps/Acta/src/ui/tasks.rs:63)
- `Payments` просунуті частково: import callback-и в UI є, але в поточному коді не знайдено підтвердження окремого `unreconcile` flow чи тестів на нього

## Workstream Status

### 1. BAS Import MVP

Статус: `not done`

#### Підтверджено в коді

- у [migrate.rs](/C:/Users/MykhailoDan/apps/Acta/src/bin/migrate.rs:1) є лише CLI parsing
- у [migrate.rs](/C:/Users/MykhailoDan/apps/Acta/src/bin/migrate.rs:74) логіка імпорту все ще не реалізована

#### Висновок

Workstream не можна вважати завершеним або навіть частково завершеним на рівні користувацького результату.

### 2. Documents Flow Completion

Статус: `not done`

#### Підтверджено в коді

- [documents.rs](/C:/Users/MykhailoDan/apps/Acta/src/ui/documents.rs:181) — `doc_new` лишається `TODO`
- [documents.rs](/C:/Users/MykhailoDan/apps/Acta/src/ui/documents.rs:185) — `doc_open` лишається `TODO`
- [documents.rs](/C:/Users/MykhailoDan/apps/Acta/src/ui/documents.rs:191) — `doc_edit` лишається `TODO`
- [documents.rs](/C:/Users/MykhailoDan/apps/Acta/src/ui/documents.rs:205) — `doc_more_actions` лишається `TODO`
- [documents.rs](/C:/Users/MykhailoDan/apps/Acta/src/ui/documents.rs:210) — bulk-дії лишаються `TODO`

#### Висновок

Documents screen не відповідає плановому definition of done.

### 3. Tasks Flow Completion

Статус: `partial`

#### Підтверджено в коді

- `task_save` справді wired у [tasks.rs](/C:/Users/MykhailoDan/apps/Acta/src/ui/tasks.rs:178)
- `task_more` не відкриває details/edit flow, а лише пише debug log у [tasks.rs](/C:/Users/MykhailoDan/apps/Acta/src/ui/tasks.rs:217)
- `day_events` ініціалізується порожньою моделлю в [tasks.rs](/C:/Users/MykhailoDan/apps/Acta/src/ui/tasks.rs:63)
- у [ui_events.rs](/C:/Users/MykhailoDan/apps/Acta/tests/ui_events.rs:553) є event-contract перевірка `task_more`
- у [ui_events.rs](/C:/Users/MykhailoDan/apps/Acta/tests/ui_events.rs:581) є event-contract перевірка `task_save`

#### Висновок

Callback contract частково wired, але user-facing flow ще не доведений до стану, описаного в плані.

### 4. Payments Import/Reconcile Completion

Статус: `partial`

#### Підтверджено в коді

- імпорт CSV із `storage/import/bank/` справді підключений у [payments.rs](/C:/Users/MykhailoDan/apps/Acta/src/ui/payments.rs:198)
- sync/import callback-и є в [payments.rs](/C:/Users/MykhailoDan/apps/Acta/src/ui/payments.rs:198) і [payments.rs](/C:/Users/MykhailoDan/apps/Acta/src/ui/payments.rs:223)
- ручний template flow для нового платежу є в [payments.rs](/C:/Users/MykhailoDan/apps/Acta/src/ui/payments.rs:248)
- `pay_link` є в [payments.rs](/C:/Users/MykhailoDan/apps/Acta/src/ui/payments.rs:267)
- у наявному файлі не знайдено окремого `pay_unlink` / `unreconcile` callback-у
- у [ui_events.rs](/C:/Users/MykhailoDan/apps/Acta/tests/ui_events.rs:443) і [ui_events.rs](/C:/Users/MykhailoDan/apps/Acta/tests/ui_events.rs:469) є контрактні тести на `pay_import_csv` і `pay_link`

#### Висновок

Payments просунулись далі, ніж було видно з первинного плану, але завершеним цей workstream назвати не можна.

## Що реально можна вважати зробленим

- planning-пакет для спринту був створений
- у `tasks` є робочий `task_save` callback
- у `payments` є реальний import callback flow для банківського CSV із файлової папки
- у `payments` є базовий reconcile callback через `pay_link`
- `ui_events` справді містить callback-contract тести для частини task/payment дій

## Що не можна вважати зробленим

- завершений BAS import MVP
- завершений documents flow
- details/edit flow для `task_more`
- наповнення `day_events`
- повністю завершений payments import/reconcile scope з окремим unreconcile flow
- відсутність критичних user-facing `TODO` у P1 scope

## Рекомендована інтерпретація спринту

Цей спринт варто трактувати не як `completed`, а як `partial implementation sprint`.
Частина wiring і callback-ів була додана, але кілька ключових outcome з плану не досягнуті.

## Наступний крок

Якщо потрібно продовжити цю тему, наступний спринт краще формувати не з нуля, а як remediation-план поверх цього звіту:

- BAS import: або реально довести `migrate.rs`, або явно винести зі sprint scope
- documents: закрити user-facing `TODO` першочергово
- tasks: добити `task_more` і `day_events`
- payments: визначити, чи потрібен окремий `unreconcile` UI flow у цьому епіку
