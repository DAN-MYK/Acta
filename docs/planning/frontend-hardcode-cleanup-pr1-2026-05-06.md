# Frontend Hardcode Cleanup PR1 Plan

**Дата:** 2026-05-06

**Мета:** підготувати малоризиковий перший cleanup PR для фронтенду `Acta`, який прибирає найбільш явні захардкоджені дублікати без великої перебудови архітектури.

**Фокус:** `frontend/src/lib/screens/`, `frontend/src/lib/components/`, `frontend/src/lib/config/ui.ts`, `frontend/src/styles.css`

---

## Контекст

Після аудиту фронтенду видно, що основна проблема не в окремих literal strings самих по собі, а в місцях, де hardcode:

- дублює вже наявне canonical джерело
- розмазує один і той самий UI-copy по кількох screens
- змішує screen-level presentation з shared доменною подачею
- ускладнює дрібні правки через розсинхрон між файлами

Цей PR має бути вузьким, швидким і безпечним. Його задача не "винести все", а закрити найочевидніші дублікати.

---

## Scope PR #1

У PR входить:

1. Уніфікація `dirty banner` copy для editor screen.
2. Уніфікація `document kind` labels/options через одне shared джерело.
3. Прибирання кількох очевидних inline style у Svelte markup.
4. Точкове прибирання повторюваних short-copy для payments/calendar там, де дубль уже явний.

У PR не входить:

- великий refactor `ui.ts`
- винесення всієї локальної business-copy
- повний CSS token pass
- зміна доменної логіки в store/backend

---

## Quick Wins На 1 PR

### 1. Dirty Banner Copy

**Ціль:** прибрати дублювання одного й того самого confirm-copy для незбережених змін.

**Файли:**

- `frontend/src/lib/config/ui.ts`
- `frontend/src/lib/screens/TasksScreen.svelte`
- `frontend/src/lib/screens/PaymentsScreen.svelte`
- `frontend/src/lib/screens/CounterpartiesScreen.svelte`
- `frontend/src/lib/screens/DocumentsScreen.svelte`

**Що зробити:**

- додати один shared copy-блок для:
  - `dirtyTitle`
  - `dirtyDescription`
  - `dirtyStay`
  - `dirtyDiscard`
- перевести `TasksScreen` і `PaymentsScreen` на нього
- звести `Counterparties` і `Documents` до того самого джерела, якщо їхній текст збігається семантично

**Очікуваний ефект:**

- один canonical текст замість кількох копій
- менше шансів, що один editor буде поводитися інакше лише через локальний hardcode

### 2. Document Kind Labels And Options

**Ціль:** прибрати локальні label-мапи та filter options, які дублюють `DocumentKind`.

**Файли:**

- `frontend/src/lib/config/ui.ts`
- `frontend/src/lib/screens/DocumentsScreen.svelte`
- `frontend/src/lib/screens/PaymentsScreen.svelte`

**Що зробити:**

- додати shared `DOCUMENT_KIND_FILTER_OPTIONS`
- у `DocumentsScreen` прибрати локальний `kindChips`
- у `PaymentsScreen` прибрати локальний `getDocumentKindLabel()`
- використовувати спільну `DOCUMENT_KIND_META` або helper поверх неї

**Очікуваний ефект:**

- усі labels для `act/invoice/waybill` беруться з одного місця
- додавання нового `DocumentKind` не вимагатиме шукати локальні ручні мапи

### 3. Inline Style Cleanup

**Ціль:** прибрати найочевидніші інлайнові presentation-значення з markup.

**Файли:**

- `frontend/src/lib/screens/SettingsScreen.svelte`
- `frontend/src/lib/screens/CounterpartiesScreen.svelte`

**Що зробити:**

- замінити `style="display: none;"` на utility/class
- замінити дрібні статичні presentation style типу `margin-top: 8px` на клас

**Не чіпати в цьому PR:**

- data-driven inline width, наприклад прогрес/бар у звітах

**Очікуваний ефект:**

- чистіший markup
- менше локального CSS hardcode прямо в template

### 4. Payments Short-Copy Cleanup

**Ціль:** винести лише короткі повторювані тексти, які вже утворюють патерн.

**Файли:**

- `frontend/src/lib/screens/PaymentsScreen.svelte`
- `frontend/src/lib/components/PaymentCalendarPanel.svelte`
- за потреби `frontend/src/lib/config/ui.ts`

**Що зробити:**

- зібрати повторювані short-copy для:
  - loading state
  - empty state
  - preview short labels
  - filter-empty state
- не переносити великі пояснювальні абзаци, якщо вони живуть лише в одному місці

**Очікуваний ефект:**

- менше шуму в screen/components
- без переходу до великої copy-системи

---

## Друга Хвиля Cleanup

Це окремі наступні задачі, не для першого PR.

### 1. Tasks Presentation Helper

**Файли:**

- `frontend/src/lib/screens/TasksScreen.svelte`
- новий helper у `frontend/src/lib/`

**Що винести:**

- `tab -> visible statuses`
- `priority -> sort weight`
- `priority -> tone`
- формат поточного дня

**Чому окремо:**

це вже не просто тексти, а presentation-логіка задач

### 2. Counterparty Scenario Helper

**Файли:**

- `frontend/src/lib/screens/CounterpartiesScreen.svelte`
- новий helper у `frontend/src/lib/`

**Що винести:**

- `getScenarioTitle`
- `getScenarioDescription`
- `getRiskLabel`

**Чому окремо:**

це змішаний блок business rules + product copy; його краще чіпати окремо і свідомо

### 3. Payments Matching Presentation Cleanup

**Файли:**

- `frontend/src/lib/screens/PaymentsScreen.svelte`
- `frontend/src/lib/stores/payments.ts`
- можливо окремий formatter/helper

**Що винести:**

- candidate hints
- decision-specific preview text assembly
- дрібні `kind/status/decision` presentation rules

### 4. Розрізати `ui.ts`

**Файли:**

- `frontend/src/lib/config/ui.ts`

**Напрямок:**

- `documents-ui.ts`
- `payments-ui.ts`
- `reports-ui.ts`
- `shared-formatters.ts`

**Чому окремо:**

зараз це корисне, але вже перевантажене shared-сховище

### 5. Малий CSS Token Pass

**Файли:**

- `frontend/src/styles.css`
- `frontend/src/styles/*.css`

**Що винести:**

- overlay background
- overlay blur
- common chip/control heights

**Що не робити:**

- не токенізувати кожен `8px`, `12px`, `16px`

---

## Що Не Чіпати

Нижче те, що в цьому cleanup краще залишити інлайн або не роздувати без потреби.

### 1. Одиничні Тексти

Не виносити:

- тексти, які живуть лише в одному screen
- локальні `aria-label`, якщо вони не дублюються
- разові button labels або placeholder-и

### 2. Уже Типізовані String Unions

Не ускладнювати:

- `ScreenId`
- `ReportsTab`
- `ReportsScope`
- інші вже типізовані enum-подібні значення

Поки вони не мають дубльованої presentation-логіки, додаткові константи поверх них не потрібні.

### 3. Технічні Декоративні Константи

Не виносити:

- skeleton widths
- локальні animation timings, якщо вони не повторюються масово
- дрібні layout-значення, які читаються краще прямо в компоненті

### 4. Великий I18n Або Copy Layer

Не запускати:

- велику i18n-систему
- глобальний registry всіх UI-текстів
- винесення "про всяк випадок"

---

## Файли Для PR #1

### Основні

- `frontend/src/lib/config/ui.ts`
- `frontend/src/lib/screens/DocumentsScreen.svelte`
- `frontend/src/lib/screens/TasksScreen.svelte`
- `frontend/src/lib/screens/PaymentsScreen.svelte`
- `frontend/src/lib/screens/CounterpartiesScreen.svelte`
- `frontend/src/lib/screens/SettingsScreen.svelte`

### Опційно

- `frontend/src/lib/components/PaymentCalendarPanel.svelte`

---

## Перевірки Після Змін

### Автоматичні

1. `npm run check`
2. `npm run test:frontend`

### Ручна Швидка Перевірка

1. Відкрити `Documents`, `Tasks`, `Payments`, `Counterparties`
2. Відкрити editor і спробувати закрити його з dirty state
3. Переконатися, що labels для типів документів однакові:
   - у фільтрах
   - у селектах
   - у payment matching UI
4. Перевірити, що після прибирання inline style нічого не зсунулося в layout

---

## Орієнтовна Назва PR

`refactor(frontend): unify editor copy and document kind ui metadata`

---

## Критерій Успіху

PR вважається вдалим, якщо після нього:

- однаковий UX-copy не дублюється по кількох screens
- labels для документів читаються з одного джерела
- markup стає чистішим без великого CSS-refactor
- ми не додаємо новий шар абстракції там, де він не дає реальної користі
