-- Міграція 025: Прив'язка шаблонів до документів
--
-- Дозволяє обрати дефолтний шаблон при створенні акту або накладної.

ALTER TABLE acts ADD COLUMN template_id UUID REFERENCES document_templates(id);
ALTER TABLE invoices ADD COLUMN template_id UUID REFERENCES document_templates(id);
