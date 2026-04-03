ALTER TABLE acts
    ADD COLUMN IF NOT EXISTS direction VARCHAR(8) NOT NULL DEFAULT 'outgoing'
    CHECK (direction IN ('outgoing', 'incoming'));

ALTER TABLE invoices
    ADD COLUMN IF NOT EXISTS direction VARCHAR(8) NOT NULL DEFAULT 'outgoing'
    CHECK (direction IN ('outgoing', 'incoming'));

CREATE INDEX IF NOT EXISTS idx_acts_direction
    ON acts(direction);

CREATE INDEX IF NOT EXISTS idx_invoices_direction
    ON invoices(direction);
