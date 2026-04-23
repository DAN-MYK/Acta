# Money Contract

Канонічне правило для `Acta`: бізнесові грошові значення не передаються в Slint як `float`.

## Що вважається money-facing

- суми документів
- KPI totals
- balances
- overdue / outstanding amounts
- payment amounts
- будь-які display fields, які користувач читає як гривневі значення

Усі такі значення мають приходити в Slint уже підготовленими як `string`.
Форматування виконується в [src/ui/helpers.rs](/C:/Users/MykhailoDan/apps/Acta/src/ui/helpers.rs).

## Де `float` дозволений

- [ui-redesign/types.slint](/C:/Users/MykhailoDan/apps/Acta/ui-redesign/types.slint) у `ChartBar.rev-h` / `ChartBar.exp-h`
- sparkline arrays у [ui-redesign/dashboard.slint](/C:/Users/MykhailoDan/apps/Acta/ui-redesign/dashboard.slint), бо це normalized render data

Це render-only значення в діапазоні `0.0..1.0`, а не бізнесові суми.

## Rust side

- `format_money()`, `format_money_round()`, `format_money_ua()` відповідають за money display
- `normalize_chart_value()` та `max_chart_value()` відповідають тільки за chart normalization

Змішувати ці два шляхи не можна: formatter-и для display, normalizer-и тільки для geometry/render.
