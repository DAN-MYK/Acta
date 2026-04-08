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

-- Індекси для company_id створюються разом з таблицями у міграціях 012, 013, 020.
