# Банківський імпорт — виконаний план (2026-05-04)

## Статус: ЗАВЕРШЕНО

---

## Що зроблено (commits в HEAD)

### Backend (Rust) — закомічено

| Файл | Зміни |
|------|-------|
| `src/import/bank_common.rs` | Новий модуль: спільні header-aliases, `parse_decimal`, `parse_date`, `HeaderLayout`, `ParsedBankRow`, `normalize_iban`, `amount_and_direction_from_strings` |
| `src/import/bank_xlsx.rs` | Новий парсер XLSX/XLS через calamine; auto-detect header row; Excel serial dates; debit/credit columns; 6 unit tests |
| `src/import/bank_csv.rs` | Рефакторинг → тонка обгортка через `bank_common`; додані: PrivatBankCsvParser, MonobankCsvParser, RaiffeisenCsvParser, OtpBankCsvParser, PumbCsvParser |
| `src/import/bas_payments.rs` | `newest_statement_path` (CSV + XLSX); `parse_payments_statement_file`; `import_payments_from_statement`; `parser_for_path` з routing по 7 банках |
| `src/tauri_api/payments.rs` | Нові DTOs: `PaymentImportPreviewDto`, `PaymentImportPreviewRowDto`, `PaymentImportCommitRequest`; `payments_import_preview`; `payments_import_commit` |
| `src-tauri/src/commands/payments.rs` | Tauri commands: `payments_import_pick_and_preview` (file picker → dry-run); `payments_import_preview`; `payments_import_commit` |
| `src-tauri/src/lib.rs` | Реєстрація нових команд |

### Frontend (TypeScript/Svelte) — незакомічено

| Файл | Зміни |
|------|-------|
| `frontend/src/lib/types.ts` | 3 нових інтерфейси: `PaymentImportPreviewRowDto`, `PaymentImportPreviewDto`, `PaymentImportCommitRequest` |
| `frontend/src/lib/api.ts` | 3 нових функції: `paymentsImportPickAndPreview`, `paymentsImportPreview`, `paymentsImportCommit` |
| `frontend/src/lib/stores/payments.ts` | `importPreview` стан; 4 нові дії: `pickAndPreviewImport`, `commitImportPreview`, `cancelImportPreview`, `refreshImportPreview` |
| `frontend/src/lib/browser-fixtures.ts` | Мок для `payment_match_manual_candidates` (був відсутній!); моки для `payments_import_pick_and_preview`, `payments_import_preview`, `payments_import_commit` |
| `frontend/src/lib/screens/PaymentsScreen.svelte` | Кнопка file picker як primary CTA; секція import preview; loading banner для `import-pick`, `import-commit`, `confirm-split`; CSS |
| `frontend/src/lib/screens/__tests__/PaymentsScreen.test.ts` | Виправлено mock: `importPreview`, `pickAndPreviewImport`, `commitImportPreview`, `cancelImportPreview`; 2 нових тести для import preview секції |
| `frontend/src/lib/stores/__tests__/counterparties-payments-settings.test.ts` | `makePaymentCalendarMonth` helper + `payments_calendar_load` mock у кожному тесті |

---

## Виправлені неточності (2026-05-04)

1. **`getFlowTitle`/`getFlowDescription`** — відсутній кейс `confirm-split` (розподіл платежу не мав loading banner)
2. **`PaymentsScreen.test.ts`** — mock store не мав `importPreview` у стані та методів `pickAndPreviewImport`, `commitImportPreview`, `cancelImportPreview`
3. **`PaymentsScreen.test.ts`** — відсутні тести для import preview секції

---

## Стан тестів після виправлень

- **vitest**: 148 passed (20 test files) ✓
- **svelte-check**: 0 errors ✓
- **cargo build --tests**: Finished ✓

---

## Що залишилось

### Обов'язково перед merge

- [ ] Закомітити 7 frontend файлів (uncommitted working tree changes)
- [ ] Запустити `cargo sqlx prepare` якщо є нові sqlx::query! макроси (зараз нових немає — перевірити)

### Опціонально / future work

- [ ] Тест з реальним XLSX файлом із bank statement (наразі тільки synthetic data)
- [ ] Оновити Obsidian vault: `Integrations/Bank Integrations.md` (MCP restricted to project dir)
- [ ] `payments_import_commit` — можна показати diff між preview і реальним commit якщо файл змінився між pick і confirm (Race condition edge case)
- [ ] Ощадбанк XLS підтримка — перевірити реальний формат (зараз test-only, не мали прикладів)

---

## UX Flow (реалізований)

```
Натиснути "Імпортувати виписку"
  ↓
Нативний file picker (CSV / XLSX / XLS)
  ↓
Dry-run parse → PaymentImportPreviewDto
  ↓
Секція preview: банк, кількість, таблиця рядків (max 25)
  ↓ [Підтвердити] або [Скасувати]
  ↓
Реальний INSERT у БД → refresh списку
```

Legacy flow "Імпорт з storage/import/bank" — залишено як ghost button (backward compat).
