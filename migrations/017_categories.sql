-- Міграція 004: Категорії доходів та витрат
--
-- Довідник статей для класифікації актів, накладних та платежів.
-- Підтримує ієрархію через parent_id.
-- Per-company: кожна компанія має свій набір категорій.

CREATE TABLE IF NOT EXISTS categories (
    id          UUID         PRIMARY KEY DEFAULT gen_random_uuid(),
    name        VARCHAR(255) NOT NULL,
    kind        VARCHAR(10)  NOT NULL CHECK (kind IN ('income', 'expense')),
    parent_id   UUID         REFERENCES categories(id) ON DELETE SET NULL,
    company_id  UUID         NOT NULL REFERENCES companies(id) ON DELETE CASCADE,
    is_archived BOOLEAN      NOT NULL DEFAULT FALSE,
    created_at  TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_categories_company  ON categories(company_id);
CREATE INDEX IF NOT EXISTS idx_categories_kind     ON categories(company_id, kind);
CREATE INDEX IF NOT EXISTS idx_categories_parent   ON categories(parent_id);
