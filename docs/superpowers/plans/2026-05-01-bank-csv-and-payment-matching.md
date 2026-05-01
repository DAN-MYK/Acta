# План: Bank CSV + Payment Matching

Дата: 2026-05-01
Статус: в роботі

## Контекст

Мета цього потоку робіт:
- імпорт CSV банківських виписок;
- preview автозіставлення платежів;
- persisted reconcile/unreconcile через link-таблиці;
- UI flow для exact / ambiguous / none;
- stable tests для критичних payment-matching сценаріїв.

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

### Task 7. Повний manual reconcile flow
Статус: `completed`

Зроблено:
- додано окремий Tauri API для manual candidate search по відкритих актах і накладних;
- payments store отримав `manualPicker` state, search/update/select/confirm actions і окремий loading flow;
- `PaymentsScreen.svelte` тепер відкриває ручний picker із `none` та `ambiguous` preview сценаріїв;
- frontend tests покривають manual picker search + confirm flow;
- Tauri command surface скомпільовано з новою командою.

### Task 8. Partial / split reconcile
Статус: `completed`

Зроблено:
- persisted reconcile вже підтримує кілька link-записів на один платіж і валідовує over-allocation;
- `payments` store отримав `splitDraft` state із залишком платежу, editable allocation amounts і confirm flow;
- `PaymentsScreen.svelte` показує split draft, remaining amount і керує persisted confirm із кількох allocation-ів;
- додано backend integration test на split/reject over-allocation;
- додано frontend regression test на багатокроковий `confirmSplitDraft()`.

Залишкові обмеження:
- split preview ще не будує amount-aware recommendation автоматично, тож partial/split flow починається з manual picker;
- persisted confirm виконується послідовними reconcile-викликами, без окремого batch command.

## Поточний стан

Готово:
- persisted reconcile backend flow;
- preview/apply-auto API;
- Svelte preview/manual confirm flow;
- partial/split draft у store та UI;
- backend і frontend regression coverage для критичних matching сценаріїв.

Залишилось:
- додати amount-aware preview heuristics для partial/split recommendation;
- розширити component-level та integration coverage для split UX і error handling.

## Наступна таска

### Task 9. Amount-aware split preview + hardening
Статус: `next`

Ціль:
- навчити matcher повертати recommendation для partial/split cases, де один exact candidate не закриває весь платіж;
- додати ширші component/integration checks на split UX і error handling.

Очікуваний обсяг:
- amount-aware backend preview logic для partial/split;
- UI підказки для рекомендованого розподілу суми;
- додаткові frontend component tests;
- ширші backend/integration перевірки для split matching.
