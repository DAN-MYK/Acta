-- Видаткові накладні (товарні накладні)
-- Статуси: draft → issued → signed → delivered

CREATE TYPE waybill_status AS ENUM ('draft', 'issued', 'signed', 'delivered');

CREATE TABLE waybills (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    company_id      UUID NOT NULL REFERENCES companies(id) ON DELETE CASCADE,
    number          VARCHAR(100) NOT NULL,
    counterparty_id UUID NOT NULL REFERENCES counterparties(id),
    contract_id     UUID REFERENCES contracts(id),
    category_id     UUID REFERENCES categories(id),
    direction       VARCHAR(20) NOT NULL DEFAULT 'outgoing',
    date            DATE NOT NULL,
    total_amount    DECIMAL(15,2) NOT NULL DEFAULT 0,
    status          waybill_status NOT NULL DEFAULT 'draft',
    notes           TEXT,
    pdf_path        VARCHAR(500),
    bas_id          VARCHAR(100) UNIQUE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE waybill_items (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    waybill_id  UUID NOT NULL REFERENCES waybills(id) ON DELETE CASCADE,
    position    SMALLINT NOT NULL DEFAULT 1,
    description TEXT NOT NULL,
    unit        VARCHAR(50),
    quantity    DECIMAL(15,4) NOT NULL DEFAULT 1,
    price       DECIMAL(15,2) NOT NULL DEFAULT 0,
    amount      DECIMAL(15,2) NOT NULL DEFAULT 0,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_waybills_company_id      ON waybills(company_id);
CREATE INDEX idx_waybills_counterparty_id ON waybills(counterparty_id);
CREATE INDEX idx_waybills_date            ON waybills(date);
CREATE INDEX idx_waybill_items_waybill_id ON waybill_items(waybill_id);
