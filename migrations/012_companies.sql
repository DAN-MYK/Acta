-- Міграція: мульти-компанійна архітектура
-- Додає таблицю companies та прив'язує існуючі таблиці до компаній

-- Частина 1: таблиця companies
CREATE TABLE companies (
    id              UUID         PRIMARY KEY DEFAULT gen_random_uuid(),
    name            TEXT         NOT NULL,
    short_name      TEXT,
    edrpou          CHAR(8),
    ipn             CHAR(10),
    iban            VARCHAR(29),
    legal_address   TEXT,
    actual_address  TEXT,
    phone           VARCHAR(50),
    email           VARCHAR(255),
    director_name   TEXT,
    accountant_name TEXT,
    tax_system      VARCHAR(20)
                        CHECK (tax_system IN ('simplified', 'general')),
    is_vat_payer    BOOLEAN      NOT NULL DEFAULT FALSE,
    logo_path       TEXT,
    notes           TEXT,
    is_archived     BOOLEAN      NOT NULL DEFAULT FALSE,
    created_at      TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_companies_edrpou ON companies(edrpou);

-- Частина 2: дефолтна компанія для backfill
INSERT INTO companies (id, name, short_name)
VALUES ('00000000-0000-0000-0000-000000000001', 'Компанія за замовчуванням', 'Default');

-- Частина 3: ALTER існуючих таблиць
ALTER TABLE counterparties
    ADD COLUMN company_id UUID REFERENCES companies(id) ON DELETE RESTRICT;

ALTER TABLE acts
    ADD COLUMN company_id UUID REFERENCES companies(id) ON DELETE RESTRICT;

-- Частина 4: Backfill
UPDATE counterparties SET company_id = '00000000-0000-0000-0000-000000000001';
UPDATE acts           SET company_id = '00000000-0000-0000-0000-000000000001';

-- Частина 5: NOT NULL
ALTER TABLE counterparties ALTER COLUMN company_id SET NOT NULL;
ALTER TABLE acts           ALTER COLUMN company_id SET NOT NULL;

-- tasks теж отримують company_id
ALTER TABLE tasks ADD COLUMN company_id UUID REFERENCES companies(id) ON DELETE RESTRICT;
UPDATE tasks SET company_id = '00000000-0000-0000-0000-000000000001';
ALTER TABLE tasks ALTER COLUMN company_id SET NOT NULL;

-- Частина 6: Індекси
CREATE INDEX idx_counterparties_company ON counterparties(company_id);
CREATE INDEX idx_acts_company           ON acts(company_id);
CREATE INDEX idx_tasks_company          ON tasks(company_id);
