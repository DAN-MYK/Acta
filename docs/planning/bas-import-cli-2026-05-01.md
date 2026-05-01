# Статус BAS import CLI

> **Archived execution note:** operational-перевірку завершено `2026-05-01`. Файл збережено як короткий запис виконаного smoke/proof, а не як активний план.

Оновлено: `2026-05-01`

## Підсумок

- `BAS import CLI` завершений і перевірений у двох режимах: `dry-run` та `write-mode`.
- Команда `cargo run --bin migrate -- --input ./bas-export/` успішно проходить повний flow запису на окремій безпечній тестовій PostgreSQL БД.
- Повторний `dry-run` після запису показує очікувану ідемпотентну поведінку:
  - контрагенти, договори, акти й накладні переходять у `оновити`;
  - платежі з тим самим `bank_ref` переходять у `пропустити`.

## Що перевірено

- discovery і маршрутизація всіх підтриманих артефактів (`counterparties`, `contracts`, `acts`, `invoices`, `payments`);
- dry-run preview на тестовому каталозі `./bas-export/`;
- write-mode import без `--dry-run` на окремій БД `acta_bas_smoke_run_20260501`;
- фактичний запис даних через SQL-перевірку по `bas_id` / `bank_ref`;
- повторний dry-run після імпорту для перевірки ідемпотентності.

## Результат smoke

- створено 1 контрагента з `bas_id = cp-smoke-001`;
- створено 1 договір з `bas_id = ctr-smoke-001`;
- створено 1 акт з `bas_id = act-smoke-001` і сумою `3000.00`;
- створено 1 накладну з `bas_id = inv-smoke-001` і сумою `1500.00`;
- створено 1 платіж з `bank_ref = PAY-SMOKE-001` і сумою `3000.00`.

## Статус

- `BAS import CLI`: `Готово`
- відкритий operational-blocker по write-mode smoke: `закрито`
