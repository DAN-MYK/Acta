-- Міграція 006: Додати category_id до acts та invoices

ALTER TABLE acts
    ADD COLUMN IF NOT EXISTS category_id UUID REFERENCES categories(id) ON DELETE SET NULL;

ALTER TABLE invoices
    ADD COLUMN IF NOT EXISTS category_id UUID REFERENCES categories(id) ON DELETE SET NULL;

CREATE INDEX IF NOT EXISTS idx_acts_category    ON acts(category_id);
CREATE INDEX IF NOT EXISTS idx_invoices_category ON invoices(category_id);
