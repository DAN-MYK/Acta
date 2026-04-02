-- Міграція 016: демо-дані для тестування функціоналу
-- Створює 30 тестових компаній і пов'язані дані:
-- counterparties, acts, act_items, invoices, invoice_items,
-- payments, payment_acts, payment_invoices, payment_schedule, tasks.

WITH new_companies AS (
    INSERT INTO companies (
        name, short_name, edrpou, ipn, iban, legal_address,
        phone, email, director_name, accountant_name,
        tax_system, is_vat_payer, notes
    )
    SELECT
        format('ТОВ "Тестова компанія %s"', gs.i),
        format('ТК-%s', lpad(gs.i::text, 2, '0')),
        lpad((90000000 + gs.i)::text, 8, '0'),
        lpad((3000000000 + gs.i)::text, 10, '0'),
        'UA' || lpad(gs.i::text, 27, '0'),
        format('м. Київ, вул. Тестова, %s', gs.i),
        format('+38050111%s', lpad(gs.i::text, 4, '0')),
        format('company%1$s@example.test', gs.i),
        format('Директор %s', gs.i),
        format('Бухгалтер %s', gs.i),
        CASE WHEN gs.i % 2 = 0 THEN 'general' ELSE 'simplified' END,
        (gs.i % 3 = 0),
        format('Демо-компанія #%s для тестування Acta', gs.i)
    FROM generate_series(1, 30) AS gs(i)
    RETURNING id, name
),
company_rows AS (
    SELECT
        c.id AS company_id,
        c.name AS company_name,
        row_number() OVER (ORDER BY c.name) AS rn
    FROM new_companies c
),
new_counterparties AS (
    INSERT INTO counterparties (
        company_id, name, edrpou, ipn, iban, address, phone, email, notes, bas_id
    )
    SELECT
        cr.company_id,
        format('ФОП Контрагент %s', cr.rn),
        lpad((80000000 + cr.rn)::text, 8, '0'),
        lpad((2000000000 + cr.rn)::text, 10, '0'),
        'UA' || lpad((1000 + cr.rn)::text, 27, '0'),
        format('м. Львів, просп. Прикладний, %s', cr.rn),
        format('+38067123%s', lpad(cr.rn::text, 4, '0')),
        format('counterparty%1$s@example.test', cr.rn),
        format('Тестовий контрагент #%s', cr.rn),
        format('DEMO-CP-%s', lpad(cr.rn::text, 3, '0'))
    FROM company_rows cr
    RETURNING id, company_id, name
),
counterparty_rows AS (
    SELECT
        cp.id AS counterparty_id,
        cp.company_id,
        cp.name AS counterparty_name,
        row_number() OVER (ORDER BY cp.name) AS rn
    FROM new_counterparties cp
),
new_acts AS (
    INSERT INTO acts (
        company_id, number, counterparty_id, date, total_amount, status, notes, bas_id, expected_payment_date
    )
    SELECT
        cpr.company_id,
        format('АКТ-2026-%s', lpad(cpr.rn::text, 3, '0')),
        cpr.counterparty_id,
        DATE '2026-01-01' + ((cpr.rn * 2)::int),
        round(((1000 + cpr.rn * 137)::numeric), 2),
        CASE cpr.rn % 4
            WHEN 0 THEN 'draft'::act_status
            WHEN 1 THEN 'issued'::act_status
            WHEN 2 THEN 'signed'::act_status
            ELSE 'paid'::act_status
        END,
        format('Тестовий акт для %s', cpr.counterparty_name),
        format('DEMO-ACT-%s', lpad(cpr.rn::text, 3, '0')),
        DATE '2026-02-01' + (cpr.rn::int)
    FROM counterparty_rows cpr
    RETURNING id, company_id, number, total_amount
),
act_rows AS (
    SELECT
        a.id AS act_id,
        a.company_id,
        a.number AS act_number,
        a.total_amount,
        row_number() OVER (ORDER BY a.number) AS rn
    FROM new_acts a
),
new_act_items AS (
    INSERT INTO act_items (
        act_id, description, quantity, unit, unit_price, amount
    )
    SELECT
        ar.act_id,
        format('Послуга за актом %s', ar.act_number),
        round((1 + (ar.rn % 5) * 0.5)::numeric, 4),
        'посл',
        round((ar.total_amount / GREATEST((1 + (ar.rn % 5) * 0.5)::numeric, 0.0001)), 2),
        ar.total_amount
    FROM act_rows ar
    RETURNING id
),
new_invoices AS (
    INSERT INTO invoices (
        company_id, number, counterparty_id, date, total_amount, vat_amount, status, notes, bas_id, expected_payment_date
    )
    SELECT
        cpr.company_id,
        format('НАК-2026-%s', lpad(cpr.rn::text, 3, '0')),
        cpr.counterparty_id,
        DATE '2026-01-02' + ((cpr.rn * 2)::int),
        round(((1500 + cpr.rn * 111)::numeric), 2),
        round(((1500 + cpr.rn * 111)::numeric) * 0.2, 2),
        CASE cpr.rn % 4
            WHEN 0 THEN 'draft'::invoice_status
            WHEN 1 THEN 'issued'::invoice_status
            WHEN 2 THEN 'signed'::invoice_status
            ELSE 'paid'::invoice_status
        END,
        format('Тестова видаткова накладна для %s', cpr.counterparty_name),
        format('DEMO-INV-%s', lpad(cpr.rn::text, 3, '0')),
        DATE '2026-02-05' + (cpr.rn::int)
    FROM counterparty_rows cpr
    RETURNING id, company_id, number, total_amount
),
invoice_rows AS (
    SELECT
        i.id AS invoice_id,
        i.company_id,
        i.number AS invoice_number,
        i.total_amount,
        row_number() OVER (ORDER BY i.number) AS rn
    FROM new_invoices i
),
new_invoice_items AS (
    INSERT INTO invoice_items (
        invoice_id, position, description, unit, quantity, price, amount
    )
    SELECT
        ir.invoice_id,
        1,
        format('Товар за накладною %s', ir.invoice_number),
        'шт',
        round((2 + (ir.rn % 4))::numeric, 4),
        round((ir.total_amount / GREATEST((2 + (ir.rn % 4))::numeric, 0.0001)), 2),
        ir.total_amount
    FROM invoice_rows ir
    RETURNING id
),
new_payments AS (
    INSERT INTO payments (
        company_id, date, amount, direction, counterparty_id, bank_name, bank_ref, description, is_reconciled, bas_id
    )
    SELECT
        cpr.company_id,
        DATE '2026-01-15' + (cpr.rn::int),
        round(((1200 + cpr.rn * 95)::numeric), 2),
        CASE WHEN cpr.rn % 5 = 0 THEN 'expense'::payment_direction ELSE 'income'::payment_direction END,
        cpr.counterparty_id,
        CASE cpr.rn % 3
            WHEN 0 THEN 'ПриватБанк'
            WHEN 1 THEN 'Ощадбанк'
            ELSE 'Укргазбанк'
        END,
        format('REF-DEMO-%s', lpad(cpr.rn::text, 4, '0')),
        format('Демо-платіж #%s', cpr.rn),
        (cpr.rn % 2 = 0),
        format('DEMO-PAY-%s', lpad(cpr.rn::text, 3, '0'))
    FROM counterparty_rows cpr
    RETURNING id, company_id, amount
),
payment_rows AS (
    SELECT
        p.id AS payment_id,
        p.company_id,
        p.amount,
        row_number() OVER (ORDER BY p.id) AS rn
    FROM new_payments p
),
new_payment_acts AS (
    INSERT INTO payment_acts (payment_id, act_id, amount)
    SELECT
        pr.payment_id,
        ar.act_id,
        round((pr.amount * 0.5), 2)
    FROM payment_rows pr
    JOIN act_rows ar
        ON ar.company_id = pr.company_id
),
new_payment_invoices AS (
    INSERT INTO payment_invoices (payment_id, invoice_id, amount)
    SELECT
        pr.payment_id,
        ir.invoice_id,
        round((pr.amount * 0.5), 2)
    FROM payment_rows pr
    JOIN invoice_rows ir
        ON ir.company_id = pr.company_id
),
new_payment_schedule AS (
    INSERT INTO payment_schedule (
        company_id, title, amount, direction, scheduled_date, recurrence,
        recurrence_end, counterparty_id, notes, is_completed
    )
    SELECT
        cpr.company_id,
        format('Запланований платіж %s', cpr.rn),
        round(((1000 + cpr.rn * 73)::numeric), 2),
        CASE WHEN cpr.rn % 4 = 0 THEN 'expense'::payment_direction ELSE 'income'::payment_direction END,
        DATE '2026-03-01' + (cpr.rn::int),
        CASE cpr.rn % 4
            WHEN 0 THEN 'monthly'::schedule_recurrence
            WHEN 1 THEN 'none'::schedule_recurrence
            WHEN 2 THEN 'weekly'::schedule_recurrence
            ELSE 'quarterly'::schedule_recurrence
        END,
        DATE '2026-12-31',
        cpr.counterparty_id,
        format('Рядок календаря платежів #%s', cpr.rn),
        FALSE
    FROM counterparty_rows cpr
    RETURNING id
)
INSERT INTO tasks (
    company_id, title, description, status, priority, due_date, reminder_at, counterparty_id, act_id
)
SELECT
    cpr.company_id,
    format('Тестове завдання #%s', cpr.rn),
    format('Перевірити документи та платіж по контрагенту %s', cpr.counterparty_name),
    CASE cpr.rn % 4
        WHEN 0 THEN 'open'::task_status
        WHEN 1 THEN 'in_progress'::task_status
        WHEN 2 THEN 'done'::task_status
        ELSE 'cancelled'::task_status
    END,
    CASE cpr.rn % 4
        WHEN 0 THEN 'normal'::task_priority
        WHEN 1 THEN 'high'::task_priority
        WHEN 2 THEN 'critical'::task_priority
        ELSE 'low'::task_priority
    END,
    NOW() + (cpr.rn || ' days')::interval,
    NOW() + (cpr.rn || ' days')::interval - INTERVAL '2 hours',
    CASE WHEN cpr.rn % 2 = 0 THEN cpr.counterparty_id ELSE NULL END,
    CASE WHEN cpr.rn % 2 = 1 THEN ar.act_id ELSE NULL END
FROM counterparty_rows cpr
LEFT JOIN act_rows ar
    ON ar.company_id = cpr.company_id;
