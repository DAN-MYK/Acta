-- Міграція 024: Шаблони документів
--
-- Зберігає metadata про .typ шаблони (шлях до файлу, назва, тип, чи дефолтний).
-- Самі .typ файли лежать в директорії templates/ проекту.

CREATE TABLE document_templates (
    id              UUID         PRIMARY KEY DEFAULT gen_random_uuid(),
    company_id      UUID         NOT NULL REFERENCES companies(id),
    name            VARCHAR(255) NOT NULL,
    description     TEXT,
    template_type   VARCHAR(20)  NOT NULL CHECK (template_type IN ('act', 'invoice')),
    template_path   VARCHAR(500) NOT NULL,
    is_default      BOOLEAN      NOT NULL DEFAULT FALSE,
    created_at      TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_doc_templates_company ON document_templates(company_id);
CREATE UNIQUE INDEX idx_doc_templates_default
    ON document_templates(company_id, template_type)
    WHERE is_default = TRUE;
