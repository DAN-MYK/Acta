# Платежі: поділ на вкладки "Банк" і "Платіжний календар"

**Дата:** 2026-05-07  
**Статус:** Затверджено

---

## Мета

Розділити монолітний екран "Платежі" на два логічних блоки через горизонтальні вкладки: "Банк" (операційна звірка банківських транзакцій) і "Платіжний календар" (планування). Кожна вкладка має власні KPI-картки.

---

## Архітектура

### Файли

| Файл | Зміна |
|------|-------|
| `frontend/src/lib/screens/PaymentsScreen.svelte` | Рефакторинг: лишається оболонкою з tab switcher, заголовком панелі, editor sheet та backdrop. Зі `<style>` лишаються тільки стилі tab switcher та editor sheet/backdrop. |
| `frontend/src/lib/components/BankTabContent.svelte` | Новий компонент: весь поточний вміст PaymentsScreen (KPI банку, тулбар, import/match preview, manual picker, split draft, дві групи платежів). Усі scoped CSS стилі банківського вмісту (`.payments-toolbar`, `.payments-group`, `.payment-row`, `.payment-state`, `.task-kpi-card`, `.payment-import-preview-*` тощо) переїжджають у `<style>` цього компонента. |
| `frontend/src/lib/components/PaymentCalendarPanel.svelte` | Додати KPI-картки зверху (3 штуки), стилізовані як `task-kpi-card`. Стиль `task-kpi-card` дублюється у `<style>` цього компонента (не виноситься глобально — обсяг малий, scoped достатньо). Поточні `calendar-summary-card` замінюються. |

### Стан табу

- Локальна змінна `activeTab: 'bank' | 'calendar'` в `PaymentsScreen.svelte`
- Не потрапляє в `paymentsStore` — суто UI стан
- Початкове значення: `'bank'`

### Data flow

- Обидва компоненти (`BankTabContent`, `PaymentCalendarPanel`) читають `paymentsStore` напряму — без нових props
- `importButton` bind, `focusImportButton()` і всі bank-only handler-функції та реактивні змінні (`closeEditor`, `onPaymentFieldChange`, `onManualSearchInput`, `onSplitAllocationInput`, `requestCloseEditor`, `confirmDiscardChanges`, `cancelDiscardChanges`, `runHeaderReconciliation`, `isPaymentBusy`, `openManualPickerForCurrentPreview`, `getFlowCopy`, а також `$: items`, `$: unmatchedPayments`, `$: matchedPayments`, `$: busyImport*`, `$: busySync`, `$: busySave`, `$: manualPickerCanConfirm`, `$: manualPickerDisabledReason`, `$: flowCopy`, `$: previewCopy`) переїжджають у `BankTabContent.svelte`
- `pendingDirtyClose` лишається в `PaymentsScreen.svelte` разом з editor sheet, бо dirty banner — частина editor overlay
- Editor sheet і backdrop лишаються в `PaymentsScreen.svelte` — відображаються поверх обох табів через `{#if $payments.editor}`

---

## UI

### Tab switcher

Розміщується між `panel-header` і вмістом активного табу. Два пункти: "Банк" і "Платіжний календар".

- Активна вкладка: нижня border-line акцентного кольору + `color: var(--acta-color-accent-text)`
- Неактивна вкладка: `color: var(--acta-color-text-muted)`
- Клас `.payments-tabs` для контейнера, `.payments-tab` для кнопки, `.payments-tab.active` для активної
- Доступність: `role="tablist"` на контейнері, `role="tab"` + `aria-selected={activeTab === 'bank'}` на кожній кнопці

### KPI-картки вкладки "Банк"

4 картки (незмінний вміст, переносяться в `BankTabContent`):
1. Надходження (`kpi.incomingStr`)
2. Витрати (`kpi.outgoingStr`)
3. Баланс (`kpi.netStr`)
4. Не зведено (`kpi.unmatchedCount`) — `task-kpi-card-alert`

### KPI-картки вкладки "Платіжний календар"

3 картки у стилі `task-kpi-card`, замінюють поточні `calendar-summary-card` у `PaymentCalendarPanel`:
1. Планових платежів у місяці (`scheduleCount`)
2. Дедлайнів задач у місяці (`taskCount`)
3. Показано подій (`visibleEventCount`) — залежить від активного фільтру

---

## Тести

Файл: `frontend/src/lib/screens/__tests__/PaymentsScreen.test.ts`

- `data-testid="payments-screen"` лишається на root `<section>` — наявні тести не ламаються
- Додати тест: початковий рендер — активна вкладка "Банк", `data-testid="payments-unmatched-group"` присутній у DOM, `data-testid="payments-calendar"` відсутній
- Додати тест: клік на вкладку "Платіжний календар" приховує `data-testid="payments-unmatched-group"` і показує `data-testid="payments-calendar"`
- Додати тест: клік на вкладку "Банк" показує `data-testid="payments-unmatched-group"` і приховує `data-testid="payments-calendar"`
- `BankTabContent` покривається через `PaymentsScreen.test.ts`, окремий тест-файл не потрібен

---

## Що не змінюється

- `paymentsStore` — без змін
- Вся логіка звірки, імпорту, match preview, split draft — без змін, просто переїжджає у `BankTabContent`
- `PaymentCalendarPanel` логіка — без змін, тільки стиль KPI-карток
- Навігація в `App.svelte` — без змін, "Платежі" лишається одним пунктом меню
