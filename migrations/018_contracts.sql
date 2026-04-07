-- Міграція 005: Договори
--
-- Договори між компанією та контрагентом.
-- Прив'язуються до актів та накладних через contract_id.

DO $$ BEGIN
    CREATE TYPE contract_status AS ENUM ('active', 'expired', 'terminated');
EXCEPTION WHEN duplicate_object THEN
    NULL;
END $$;

CREATE TABLE IF NOT EXISTS contracts (
    id              UUID            PRIMARY KEY DEFAULT gen_random_uuid(),
    company_id      UUID            NOT NULL REFERENCES companies(id) ON DELETE CASCADE,
    counterparty_id UUID            NOT NULL REFERENCES counterparties(id) ON DELETE RESTRICT,
    number          VARCHAR(100)    NOT NULL,
    subject         TEXT,
    date            DATE            NOT NULL,
    expires_at      DATE,
    amount          DECIMAL(15,2),
    status          contract_status NOT NULL DEFAULT 'active',
    notes           TEXT,
    bas_id          VARCHAR(100)    UNIQUE,
    created_at      TIMESTAMPTZ     NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ     NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_contracts_company       ON contracts(company_id);
CREATE INDEX IF NOT EXISTS idx_contracts_counterparty  ON contracts(counterparty_id);
CREATE INDEX IF NOT EXISTS idx_contracts_status        ON contracts(status);

-- Додаємо FK до acts та invoices (поля contract_id вже існують без FK)
ALTER TABLE acts
    ADD CONSTRAINT fk_acts_contract
    FOREIGN KEY (contract_id) REFERENCES contracts(id) ON DELETE SET NULL;

ALTER TABLE invoices
    ADD CONSTRAINT fk_invoices_contract
    FOREIGN KEY (contract_id) REFERENCES contracts(id) ON DELETE SET NULL;
