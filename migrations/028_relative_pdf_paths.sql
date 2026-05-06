-- Нормалізуємо керовані PDF-шляхи до відносного формату від storage/documents.
-- Після цього в БД зберігається лише existing_pdf/.../working.pdf, без абсолютного префікса машини.

UPDATE invoices
SET pdf_path = regexp_replace(
    replace(pdf_path, '\', '/'),
    '^.*?(existing_pdf/.*)$',
    '\1'
)
WHERE pdf_path IS NOT NULL
  AND replace(pdf_path, '\', '/') ~ '(^|/)existing_pdf/';

UPDATE waybills
SET pdf_path = regexp_replace(
    replace(pdf_path, '\', '/'),
    '^.*?(existing_pdf/.*)$',
    '\1'
)
WHERE pdf_path IS NOT NULL
  AND replace(pdf_path, '\', '/') ~ '(^|/)existing_pdf/';
