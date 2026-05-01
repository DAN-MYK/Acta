# UI/UX Roadmap Execution Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Довести канонічний Svelte/Tauri UI до послідовного, передбачуваного і сценарно-орієнтованого стану для shell, Documents, Payments, Counterparties та Reports.

**Architecture:** Працюємо хвилями від системного фундаменту до продуктових екранів. Спочатку стабілізуємо shell, кнопки, стани і form controls, потім використовуємо `Settings` як референсний стенд, після чого послідовно поліруємо ключові екрани з найбільшим бізнес-впливом. Стан, поведінка і мікрокопі мають консолідуватися навколо наявних Svelte stores, screen-компонентів і shared CSS tokens без розширення backend/API surface, якщо це не критично.

**Tech Stack:** Svelte, TypeScript, Vitest, Tauri invoke API, shared CSS tokens.

---

## Карта файлів

**Shell / app frame**
- `frontend/src/App.svelte` — shell layout, company switcher, command palette, top-level loading/focus behavior.
- `frontend/src/lib/stores/shell.ts` — loading/error state shell і перемикання компанії.
- `frontend/src/lib/stores/palette.ts` — open/close/reset/search semantics command palette.
- `frontend/src/lib/stores/navigation.ts` — top-level screen navigation.
- `frontend/src/lib/stores/__tests__/palette-behavior.test.ts` — поведінкові тести палітри.
- `frontend/src/lib/stores/__tests__/shell-documents.test.ts` — shell/document integration coverage.

**System foundation**
- `frontend/src/lib/styles/tokens.css` — канонічні design tokens.
- `frontend/src/styles.css` — глобальні base styles, focus-visible, базові control states.
- `frontend/src/styles/settings.css` — референсний системний екран.
- `frontend/src/lib/screens/SettingsScreen.svelte` — референс для button/action/form patterns.
- `frontend/src/lib/screens/__tests__/SettingsScreen.test.ts` — regression coverage для settings UX.

**Product screens**
- `frontend/src/lib/screens/DocumentsScreen.svelte`
- `frontend/src/lib/screens/PaymentsScreen.svelte`
- `frontend/src/lib/screens/CounterpartiesScreen.svelte`
- `frontend/src/lib/screens/ReportsScreen.svelte`
- `frontend/src/styles/documents.css`
- `frontend/src/styles/counterparties.css`
- `frontend/src/styles/reports.css`
- `frontend/src/lib/screens/__tests__/DocumentsScreen.test.ts`
- `frontend/src/lib/screens/__tests__/PaymentsScreen.test.ts`
- `frontend/src/lib/screens/__tests__/CounterpartiesScreen.test.ts`
- `frontend/src/lib/screens/__tests__/ReportsScreen.test.ts`

**Supporting stores**
- `frontend/src/lib/stores/documents.ts`
- `frontend/src/lib/stores/payments.ts`
- `frontend/src/lib/stores/counterparties.ts`
- `frontend/src/lib/stores/reports.ts`
- `frontend/src/lib/stores/settings.ts`

## Execution Waves

### Task 1: Shell polish і deterministic command palette

**Files:**
- Modify: `frontend/src/App.svelte`
- Modify: `frontend/src/lib/stores/shell.ts`
- Modify: `frontend/src/lib/stores/palette.ts`
- Test: `frontend/src/lib/stores/__tests__/palette-behavior.test.ts`
- Test: `frontend/src/lib/stores/__tests__/shell-documents.test.ts`

- [x] Зафіксувати контракт shell loading: initial load, company reload, temporary disabling critical actions, visible progress state.
- [x] Довести command palette до deterministic behavior: `Esc` always closes, reopen starts from clean query/results state, focus returns predictably.
- [x] Прибрати “німі” стани при reload компанії та перевірити, що switcher не допускає повторних ризикових дій.
- [x] Оновити Vitest coverage для palette/shell сценаріїв.
- [x] Запустити `npm run test:frontend -- palette-behavior` або еквівалентний таргетований Vitest suite.
- [x] Зробити окремий коміт: `feat: polish shell loading and command palette flow`.

### Task 2: Unified UI foundation для buttons, states і form controls

**Files:**
- Modify: `frontend/src/lib/styles/tokens.css`
- Modify: `frontend/src/styles.css`
- Modify: `frontend/src/lib/screens/SettingsScreen.svelte`
- Modify: `frontend/src/styles/settings.css`
- Test: `frontend/src/lib/screens/__tests__/SettingsScreen.test.ts`

- [x] Зафіксувати канонічну ієрархію `primary / secondary / ghost / danger` без локальних button-винятків.
- [x] Уніфікувати `loading / disabled / error / success` states для кнопок та базових controls.
- [x] Довести `input / select / textarea / date` до спільної висоти, padding, focus-visible і disabled semantics.
- [x] Використати `Settings` як референсний екран для action rows, section cards і integration states.
- [x] Уточнити долю `density`: або реально підключити вплив на layout, або прибрати selector до готового рішення.
- [x] Запустити `npm run check` і `npm run test:frontend -- SettingsScreen`.
- [x] Зробити окремий коміт: `feat: unify button hierarchy and control states`.

### Task 3: Documents як перший повний сценарний екран

**Files:**
- Modify: `frontend/src/lib/screens/DocumentsScreen.svelte`
- Modify: `frontend/src/styles/documents.css`
- Modify: `frontend/src/lib/stores/documents.ts`
- Test: `frontend/src/lib/screens/__tests__/DocumentsScreen.test.ts`

- [x] Переробити create-strip навколо реального сценарію: контрагент, тип документа, головний CTA.
- [x] Вирівняти editor-header: `Зберегти` як primary, `Наступний статус` як secondary, `Видалити` як danger, `Закрити` як ghost.
- [x] Уніфікувати date control і прибрати неоднозначний формат вводу дати.
- [x] Перетворити ланцюжок документа зі списку зв’язків на status-flow/navigation block.
- [x] Поліпшити item editor: читабельність числових колонок, CTA для додавання позиції, сильніший empty state.
- [x] Запустити `npm run test:frontend -- DocumentsScreen` і `npm run check`.
- [x] Зробити окремий коміт: `feat: redesign documents flow around scenario-first actions`.

### Task 4: Payments навколо імпорту і звірки

**Files:**
- Modify: `frontend/src/lib/screens/PaymentsScreen.svelte`
- Modify: `frontend/src/lib/stores/payments.ts`
- Test: `frontend/src/lib/screens/__tests__/PaymentsScreen.test.ts`

- [x] Перебудувати header навколо трьох головних дій: імпорт, звірка, ручний платіж.
- [x] Візуально відокремити `matched` і `unmatched`, щоб незведені платежі читалися з першого погляду.
- [x] Підсилити CTA `Звести` / `Зняти зведення` і не ховати їх серед другорядних дій.
- [x] Уніфікувати date control і поля editor-а: напрям, сума, контрагент, референс, зв’язок з документом.
- [x] Додати чіткі loading/empty/error states для import/reconciliation flow.
- [x] Запустити `npm run test:frontend -- PaymentsScreen` і `npm run check`.
- [x] Зробити окремий коміт: `feat: refocus payments screen on reconciliation workflow`.

### Task 5: Counterparties як operational/risk card

**Files:**
- Modify: `frontend/src/lib/screens/CounterpartiesScreen.svelte`
- Modify: `frontend/src/styles/counterparties.css`
- Modify: `frontend/src/lib/stores/counterparties.ts`
- Test: `frontend/src/lib/screens/__tests__/CounterpartiesScreen.test.ts`

- [x] Підняти в detail panel уже наявний DTO-контекст: баланс, прострочка, сума прострочки, останній контакт, директор, банк, VAT-статус.
- [x] Перетворити праву панель на сценарний блок: хто це, фінансовий стан, документи, платежі, наступна дія.
- [x] Посилити CTA `Редагувати`, `Створити документ`, `Архівувати`.
- [x] Поліпшити empty state правої панелі, щоб він підказував корисний наступний крок.
- [x] Запустити `npm run test:frontend -- CounterpartiesScreen` і `npm run check`.
- [x] Зробити окремий коміт: `feat: turn counterparty detail into operational risk view`.

### Task 6: Reports readability і KPI context

**Files:**
- Modify: `frontend/src/lib/screens/ReportsScreen.svelte`
- Modify: `frontend/src/styles/reports.css`
- Modify: `frontend/src/lib/stores/reports.ts`
- Test: `frontend/src/lib/screens/__tests__/ReportsScreen.test.ts`

- [x] Переписати мікрокопі українською і зробити фільтри зрозумілими без знання внутрішньої моделі.
- [x] Підсилити KPI-блок залежно від активної вкладки звіту.
- [x] Поліпшити wide-table scanning: sticky header, контраст, типографіка сум, акценти на прострочці, вирівнювання колонок.
- [x] За можливості додати сценарний порядок важливості або принаймні стабільне ранжування проблемних рядків.
- [x] Запустити `npm run test:frontend -- ReportsScreen` і `npm run check`.
- [x] Зробити окремий коміт: `feat: improve reports readability and KPI context`.

### Task 7: Cross-screen polish, empty states і accessibility

**Files:**
- Modify: `frontend/src/App.svelte`
- Modify: `frontend/src/styles.css`
- Modify: `frontend/src/lib/styles/tokens.css`
- Modify: `frontend/src/lib/screens/DocumentsScreen.svelte`
- Modify: `frontend/src/lib/screens/PaymentsScreen.svelte`
- Modify: `frontend/src/lib/screens/CounterpartiesScreen.svelte`
- Modify: `frontend/src/lib/screens/ReportsScreen.svelte`
- Test: `frontend/src/lib/screens/__tests__/DocumentsScreen.test.ts`
- Test: `frontend/src/lib/screens/__tests__/PaymentsScreen.test.ts`
- Test: `frontend/src/lib/screens/__tests__/CounterpartiesScreen.test.ts`
- Test: `frontend/src/lib/screens/__tests__/ReportsScreen.test.ts`

- [ ] Звести empty states до єдиного патерну: пояснення, наступний крок, візуальна ієрархія.
- [ ] Уніфікувати destructive confirmations там, де ризик дії справді високий.
- [ ] Добити фінансову типографіку: суми, негативні значення, alignment, status colors.
- [ ] Перевірити focus order, focus trap, hit areas, keyboard flow і видимість focus-visible на критичних сценаріях.
- [ ] Запустити повний фронтенд-чек: `npm run check` і `npm run test:frontend`.
- [ ] Зробити фінальний коміт хвилі: `feat: finalize cross-screen polish and accessibility pass`.

## Порядок виконання і межі PR/комітів

1. Shell and palette.
2. UI foundation and Settings.
3. Documents.
4. Payments.
5. Counterparties.
6. Reports.
7. Cross-screen polish and accessibility.

Кожна хвиля має бути mergeable окремо. Не змішувати системний foundation і продуктову переробку в один великий коміт.

## Definition of Done

- Усі головні екрани користуються однією системою кнопок і control states.
- Async-дії мають видимий loading або disabled state.
- Command palette передбачувано відкривається, закривається і повертає фокус.
- Documents, Payments, Counterparties і Reports читаються як сценарії, а не як набори випадкових блоків.
- Англомовні системні підписи прибрані, якщо немає явно виправданого винятку.
- Клавіатурний користувач проходить критичні top-level сценарії без втрати фокусу.

## Verification Checklist

- `npm run check`
- `npm run test:frontend`
- Таргетовані Vitest suites для змінених stores/screens
- Візуальний smoke pass для:
  - shell/company switcher;
  - command palette;
  - Documents create/edit;
  - Payments reconcile/unreconcile;
  - Counterparty detail panel;
  - Reports wide tables.

## Ризики і guardrails

- Не розширювати backend/API surface без реальної блокуючої потреби.
- Не плодити локальні CSS-винятки, якщо їх можна підняти в shared tokens/base styles.
- Не змішувати behavioral fixes і великий visual redesign в одній хвилі без окремих тестів.
- Якщо екран вимагає нового reusable pattern, спочатку зафіксувати його в system foundation, а вже потім копіювати в екран.

---

## Статус реалізації

**Tasks 1–6 повністю реалізовані** — 2026-05-01. Commit: `dd31c7d` (merged via feat/ui-roadmap-wave-1 + reports session).

| Task | Статус |
|------|--------|
| Task 1: Shell polish і deterministic command palette | ✅ |
| Task 2: Unified UI foundation (buttons, states, form controls) | ✅ |
| Task 3: Documents як сценарний екран | ✅ |
| Task 4: Payments навколо reconciliation workflow | ✅ |
| Task 5: Counterparties як operational/risk card | ✅ |
| Task 6: Reports readability і KPI context | ✅ |
| **Task 7: Cross-screen polish, empty states, accessibility** | **⏭ відкладено** |

**Task 7 залишається відкритим** і містить:
- Уніфікація empty states до єдиного патерну з поясненням і наступним кроком
- Destructive confirmations там де ризик справді високий
- Фінансова типографіка: суми, негативні значення, alignment, status colors
- Accessibility pass: focus order, focus trap, hit areas, keyboard flow, focus-visible
