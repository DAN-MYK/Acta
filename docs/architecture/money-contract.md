# Money Contract

Оновлено: `2026-04-30`

Канонічне правило для `Acta`: бізнесові грошові значення не передаються як `float`.

## Backend

- Для всіх фінансових сум у Rust використовується `rust_decimal::Decimal`.
- У БД суми зберігаються як `DECIMAL(15,2)`.
- Кількості зберігаються як `DECIMAL(15,4)`.
- `f32` / `f64` не використовуються для грошей, балансів, цін або кількостей.

## Tauri DTO

Frontend-facing грошові поля передаються вже відформатованими рядками:

- `amountStr`;
- `balanceStr`;
- `incomeStr`;
- `expenseStr`;
- `netStr`;
- `overdueAmountStr`;
- інші поля, які користувач читає як гривневі значення.

TypeScript DTO у [frontend/src/lib/types.ts](/C:/Users/MykhailoDan/apps/Acta/frontend/src/lib/types.ts) має зберігати такі значення як `string`.

## Frontend

Svelte screens не рахують гроші через `number`. Вони:

- показують `*Str` поля з backend DTO;
- передають draft input назад як `string`;
- не виконують фінансове округлення у компоненті;
- не перетворюють display amount у `number`, якщо це не суто UI-only поле без бізнесового сенсу.

## Chart/display винятки

Нормалізовані значення для візуальної геометрії можуть бути `number`, якщо:

- це не сума;
- діапазон документовано як render-only, наприклад `0.0..1.0`;
- поруч існує окреме `*Str` поле для користувацького money display.

Поточний Svelte/Tauri contract не переносить старі Slint `ChartBar.rev-h` / `exp-h` як live правило; це лишається archived reference.
