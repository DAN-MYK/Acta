-- Видаткові накладні та їх позиції
-- company_id NOT NULL з самого початку (після міграції 012_companies)

-- ENUM статусів накладної — аналогічно act_status
CREATE TYPE invoice_status AS ENUM ('draft', 'issued', 'signed', 'paid');

CREATE TABLE invoices (
    id              UUID            PRIMARY KEY DEFAULT gen_random_uuid(),
    company_id      UUID            NOT NULL REFERENCES companies(id),
    number          VARCHAR(100)    NOT NULL,
    counterparty_id UUID            NOT NULL REFERENCES counterparties(id),
    contract_id     UUID,           -- FK до contracts додамо пізніше
    date            DATE            NOT NULL,
    total_amount    DECIMAL(15,2)   NOT NULL DEFAULT 0,
    vat_amount      DECIMAL(15,2)   NOT NULL DEFAULT 0,
    status          invoice_status  NOT NULL DEFAULT 'draft',
    notes           TEXT,
    pdf_path        TEXT,
    bas_id          VARCHAR(100)    UNIQUE,
    created_at      TIMESTAMPTZ     NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ     NOT NULL DEFAULT NOW()
);

CREATE TABLE invoice_items (
    id          UUID            PRIMARY KEY DEFAULT gen_random_uuid(),
    invoice_id  UUID            NOT NULL REFERENCES invoices(id) ON DELETE CASCADE,
    position    SMALLINT        NOT NULL,   -- порядок позицій у накладній
    description TEXT            NOT NULL,
    unit        VARCHAR(50),
    quantity    DECIMAL(15,4)   NOT NULL DEFAULT 1,
    price       DECIMAL(15,2)   NOT NULL,  -- ціна за одиницю (не unit_price!)
    amount      DECIMAL(15,2)   NOT NULL,  -- quantity * price, денормалізовано
    created_at  TIMESTAMPTZ     NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ     NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_invoices_company    ON invoices(company_id);
CREATE INDEX idx_invoices_counterparty ON invoices(counterparty_id);
CREATE INDEX idx_invoice_items_invoice ON invoice_items(invoice_id);
