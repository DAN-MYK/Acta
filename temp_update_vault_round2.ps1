$projectStructurePath = "C:\Users\MykhailoDan\OneDrive - UDPR\Obsidian\Mykhailo_Dan\development\Acta\Architecture\Project Structure.md"
$calendarPath = "C:\Users\MykhailoDan\OneDrive - UDPR\Obsidian\Mykhailo_Dan\development\Acta\Features\Payment Calendar.md"
$todoPath = "C:\Users\MykhailoDan\OneDrive - UDPR\Obsidian\Mykhailo_Dan\development\Acta\Features\Todo Feature.md"
$figmaSpecPath = "C:\Users\MykhailoDan\OneDrive - UDPR\Obsidian\Mykhailo_Dan\development\Acta\UI\Figma UI Spec.md"

$project = Get-Content -Raw -Encoding UTF8 -LiteralPath $projectStructurePath
$project = $project.Replace("│   ├── 006_invoices.sql         ← ПРОПУЩЕНО (invoices реалізовано як 013) 🔲", "│   ├── 006_invoices.sql         ← ПРОПУЩЕНО (invoices реалізовано як 013) 🔲")
$project = $project.Replace("│   ├── 007_payments.sql         ← payments + payment_acts + payment_invoices 🔲", "│   ├── 007_payments.sql         ← історичний план; фактична схема платежів у 015 🔲")
$project = $project.Replace("│   ├── 009_tasks.sql                 ← task_status, task_priority ENUMи + таблиця tasks 🔲", "│   ├── 009_tasks.sql                 ← task_status, task_priority ENUMи + таблиця tasks ✅")
$project = $project.Replace("│   ├── 010_expected_payment_date.sql ← ALTER acts/invoices ADD expected_payment_date 🔲", "│   ├── 010_expected_payment_date.sql ← історичний план; фактично реалізовано в 015 🔲")
$project = $project.Replace("│   ├── 011_payment_schedule.sql      ← schedule_recurrence ENUM + payment_schedule 🔲", "│   ├── 011_payment_schedule.sql      ← історичний план; фактично реалізовано в 015 🔲")
$project = $project.Replace("notify-rust = ""4""", "notify-rust = { path = ""vendor/notify-rust"" }")
$project = $project.Replace("│   │   └── payment_list.slint   ← Зіставлення + фільтр банків (Figma 19:102)", "│   │   └── payment_list.slint   ← Зіставлення + фільтр банків (planned, ще не створено)")
$project = $project.Replace("│   └── notifications.rs         ← reminder_loop() для desktop-сповіщень 🔲", "│   └── notifications.rs         ← reminder_loop() для desktop-сповіщень ✅")
Set-Content -LiteralPath $projectStructurePath -Encoding UTF8 -Value $project

$calendar = Get-Content -Raw -Encoding UTF8 -LiteralPath $calendarPath
$calendar = $calendar.Replace("├── 010_expected_payment_date.sql  ← ALTER TABLE acts/invoices ADD expected_payment_date", "├── 010_expected_payment_date.sql  ← історичний план; фактично увійшло в 015")
$calendar = $calendar.Replace("└── 011_payment_schedule.sql       ← Таблиця planned payments + schedule_recurrence ENUM", "├── 011_payment_schedule.sql       ← історичний план; фактично увійшло в 015`r`n└── 015_payments_and_schedule.sql  ← expected_payment_date + payment_schedule + payments")
Set-Content -LiteralPath $calendarPath -Encoding UTF8 -Value $calendar

$todo = Get-Content -Raw -Encoding UTF8 -LiteralPath $todoPath
$todo = $todo.Replace("-- invoice_id   UUID            REFERENCES invoices(id)       ON DELETE CASCADE,  ← додати після 006_invoices.sql", "-- invoice_id   UUID            REFERENCES invoices(id)       ON DELETE CASCADE,  ← додати після розширення задач на накладні (інвойси вже в 013)")
$todo = $todo.Replace("notify-rust = ""4""", "notify-rust = { path = ""vendor/notify-rust"" }")
Set-Content -LiteralPath $todoPath -Encoding UTF8 -Value $todo

$figma = Get-Content -Raw -Encoding UTF8 -LiteralPath $figmaSpecPath
$figma = $figma.Replace("| `19:102` | Платежі | `ui/payments/payment_list.slint` | 🔲 |", "| `19:102` | Платежі | `ui/payments/payment_list.slint` | 🔲 planned; у репозиторії поки placeholder у `ui/main.slint` |")
Set-Content -LiteralPath $figmaSpecPath -Encoding UTF8 -Value $figma
