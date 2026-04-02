-- Міграція 015: платежі + календар платежів
-- Додає таблиці payments, payment_acts, payment_invoices, payment_schedule
-- та поле expected_payment_date для acts/invoices.

ALTER TABLE acts
    ADD COLUMN IF NOT EXISTS expected_payment_date DATE;

ALTER TABLE invoices
    ADD COLUMN IF NOT EXISTS expected_payment_date DATE;

CREATE INDEX IF NOT EXISTS idx_acts_expected_payment
    ON acts(expected_payment_date);

CREATE INDEX IF NOT EXISTS idx_invoices_expected_payment
    ON invoices(expected_payment_date);

DO $$ BEGIN
    CREATE TYPE payment_direction AS ENUM ('income', 'expense');
EXCEPTION WHEN duplicate_object THEN
    NULL;
END $$;

DO $$ BEGIN
    CREATE TYPE schedule_recurrence AS ENUM ('none', 'weekly', 'monthly', 'quarterly', 'yearly');
EXCEPTION WHEN duplicate_object THEN
    NULL;
END $$;

CREATE TABLE IF NOT EXISTS payments (
    id              UUID              PRIMARY KEY DEFAULT gen_random_uuid(),
    company_id      UUID              NOT NULL REFERENCES companies(id) ON DELETE RESTRICT,
    date            DATE              NOT NULL,
    amount          DECIMAL(15,2)     NOT NULL,
    direction       payment_direction NOT NULL,
    counterparty_id UUID              REFERENCES counterparties(id) ON DELETE SET NULL,
    bank_name       VARCHAR(100),
    bank_ref        VARCHAR(255),
    description     TEXT,
    is_reconciled   BOOLEAN           NOT NULL DEFAULT FALSE,
    bas_id          VARCHAR(100)      UNIQUE,
    created_at      TIMESTAMPTZ       NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ       NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_payments_company
    ON payments(company_id);
CREATE INDEX IF NOT EXISTS idx_payments_date
    ON payments(date);
CREATE INDEX IF NOT EXISTS idx_payments_counterparty
    ON payments(counterparty_id);
CREATE INDEX IF NOT EXISTS idx_payments_is_reconciled
    ON payments(is_reconciled);

CREATE TABLE IF NOT EXISTS payment_acts (
    payment_id UUID          NOT NULL REFERENCES payments(id) ON DELETE CASCADE,
    act_id     UUID          NOT NULL REFERENCES acts(id) ON DELETE CASCADE,
    amount     DECIMAL(15,2) NOT NULL,
    PRIMARY KEY (payment_id, act_id)
);

CREATE TABLE IF NOT EXISTS payment_invoices (
    payment_id UUID          NOT NULL REFERENCES payments(id) ON DELETE CASCADE,
    invoice_id UUID          NOT NULL REFERENCES invoices(id) ON DELETE CASCADE,
    amount     DECIMAL(15,2) NOT NULL,
    PRIMARY KEY (payment_id, invoice_id)
);

CREATE TABLE IF NOT EXISTS payment_schedule (
    id              UUID                PRIMARY KEY DEFAULT gen_random_uuid(),
    company_id      UUID                NOT NULL REFERENCES companies(id) ON DELETE RESTRICT,
    title           TEXT                NOT NULL,
    amount          DECIMAL(15,2),
    direction       payment_direction   NOT NULL,
    scheduled_date  DATE                NOT NULL,
    recurrence      schedule_recurrence NOT NULL DEFAULT 'none',
    recurrence_end  DATE,
    counterparty_id UUID                REFERENCES counterparties(id) ON DELETE SET NULL,
    notes           TEXT,
    is_completed    BOOLEAN             NOT NULL DEFAULT FALSE,
    created_at      TIMESTAMPTZ         NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ         NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_payment_schedule_company
    ON payment_schedule(company_id);
CREATE INDEX IF NOT EXISTS idx_payment_schedule_date
    ON payment_schedule(scheduled_date);
CREATE INDEX IF NOT EXISTS idx_payment_schedule_recurrence
    ON payment_schedule(recurrence)
    WHERE recurrence != 'none';

