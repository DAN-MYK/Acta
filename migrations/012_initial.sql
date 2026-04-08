-- Міграція 001: Контрагенти
-- Базова таблиця — від неї залежать усі документи

CREATE TABLE counterparties (
    id          UUID         PRIMARY KEY DEFAULT gen_random_uuid(),
    company_id  UUID         NOT NULL REFERENCES companies(id) ON DELETE RESTRICT,
    name        VARCHAR(500) NOT NULL,
    edrpou      CHAR(8),                        -- ЄДРПОУ (8 цифр)
    iban        VARCHAR(29),                    -- UA + 27 цифр
    address     TEXT,
    phone       VARCHAR(50),
    email       VARCHAR(255),
    notes       TEXT,
    is_archived BOOLEAN      NOT NULL DEFAULT FALSE,
    bas_id      VARCHAR(100) UNIQUE,            -- оригінальний ID з BAS
    created_at  TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_counterparties_name    ON counterparties(name);
CREATE INDEX idx_counterparties_edrpou  ON counterparties(edrpou);
CREATE INDEX idx_counterparties_company ON counterparties(company_id);
