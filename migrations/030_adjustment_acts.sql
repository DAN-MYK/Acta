CREATE TYPE adjustment_act_status AS ENUM ('draft', 'issued', 'signed', 'applied');

CREATE TABLE adjustment_acts (
    id                UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    company_id        UUID NOT NULL REFERENCES companies(id),
    original_act_id   UUID NOT NULL REFERENCES acts(id),
    counterparty_id   UUID NOT NULL REFERENCES counterparties(id),
    number            VARCHAR(50) NOT NULL,
    date              DATE NOT NULL,
    direction         VARCHAR(8) NOT NULL DEFAULT 'outgoing'
                        CHECK (direction IN ('outgoing', 'incoming')),
    total_amount      DECIMAL(15,2) NOT NULL,
    status            adjustment_act_status NOT NULL DEFAULT 'draft',
    notes             TEXT,
    bas_id            VARCHAR(100) UNIQUE,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(company_id, number)
);

CREATE TABLE adjustment_act_items (
    id                UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    adjustment_act_id UUID NOT NULL REFERENCES adjustment_acts(id) ON DELETE CASCADE,
    description       TEXT NOT NULL,
    quantity          DECIMAL(15,4) NOT NULL,
    unit_price        DECIMAL(15,2) NOT NULL,
    total_price       DECIMAL(15,2) NOT NULL,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at        TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_adjustment_acts_company      ON adjustment_acts(company_id);
CREATE INDEX idx_adjustment_acts_original     ON adjustment_acts(original_act_id);
CREATE INDEX idx_adjustment_acts_counterparty ON adjustment_acts(counterparty_id);
CREATE INDEX idx_adjustment_act_items_parent  ON adjustment_act_items(adjustment_act_id);

ALTER TABLE acts ADD COLUMN is_adjusted BOOLEAN NOT NULL DEFAULT FALSE;
