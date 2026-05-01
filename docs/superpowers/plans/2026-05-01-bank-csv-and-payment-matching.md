# План: Bank CSV + Payment Matching

Дата: 2026-05-01
Статус: в роботі

## Контекст

Мета цього потоку робіт:
- імпорт CSV банківських виписок;
- preview автозіставлення платежів;
- persisted reconcile/unreconcile через link-таблиці;
- UI flow для exact / ambiguous / none;
- стабільні тести для критичних сценаріїв.

## Що вже виконано

### Task 3. Persisted reconcile flow
Статус: `completed`

Зроблено:
- reconcile/unreconcile переведено з простого toggle у реальні зв'язки `payment_acts` / `payment_invoices`;
- `is_reconciled` використовується як derived state;
- inbox flow більше не обходить persisted reconcile логіку;
- backend-тести на persisted links і derived state оновлено.

### Task 4. Preview і auto-apply API
Статус: `completed`

Зроблено:
- додано `payment_match_preview`;
- додано `payment_match_apply_auto`;
- matcher повертає scored candidates і recommendation для exact match;
- додано backend support для безпечного `unreconcile_all`.

### Task 5. UI preview flow
Статус: `completed`

Зроблено:
- payments store відкриває preview перед reconcile;
- exact-match проходить через явне підтвердження автозіставлення;
- ambiguous / none мають окремі стани й повідомлення;
- preview UI винесений у `PaymentsScreen.svelte`.

### Task 6. Persisted manual confirm для ambiguous
Статус: `completed`

Зроблено:
- додано persisted manual confirm для вибраного кандидата;
- UI отримав окрему дію `Підтвердити вибраний варіант`;
- frontend tests покривають `exact`, `ambiguous`, `none` і preview failure path.

### Task 6a. Hardening failure paths
Статус: `completed`

Зроблено:
- додано regression test для `payment_reconcile -> ok: false`;
- додано regression test для `payment_reconcile -> throw`;
- зафіксовано контракт: preview і selected candidate не губляться після невдалого manual confirm.

## Поточний стан

Готово:
- persisted reconcile backend flow;
- preview/apply-auto API;
- Svelte preview/manual confirm flow;
- frontend regression coverage для success, error-result і exception path.

Залишилось:
- завершити наступний UI/backend етап для повноцінного ручного reconcile поза межами лише preview candidates;
- далі перейти до інтеграційних та component-level перевірок.

## Наступна таска

### Task 7. Повний manual reconcile flow
Статус: `next`

Ціль:
- дати користувачу не лише підтверджувати запропоновані candidates, а й вручну обирати документ з повнішого списку;
- підтягнути окремий picker/search flow для актів і накладних;
- підготувати основу для partial / split reconcile, якщо сума платежу не збігається 1:1.

Очікуваний обсяг:
- Tauri API для candidate search/manual picker;
- store actions для відкриття picker і підтвердження manual link;
- UI для вибору документа поза preview recommendation;
- тести для manual picker flow.
