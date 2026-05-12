-- Обмежуємо BAS-ідентифікатори межами компанії.
--
-- BAS id є зовнішнім ідентифікатором документа. Один і той самий BAS id може
-- траплятися в експорті різних компаній, тому унікальність має бути per company.

DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM counterparties WHERE bas_id IS NOT NULL GROUP BY company_id, bas_id HAVING COUNT(*) > 1
    ) THEN
        RAISE EXCEPTION 'Cannot scope counterparties.bas_id: duplicate company_id/bas_id values exist';
    END IF;

    IF EXISTS (
        SELECT 1 FROM acts WHERE bas_id IS NOT NULL GROUP BY company_id, bas_id HAVING COUNT(*) > 1
    ) THEN
        RAISE EXCEPTION 'Cannot scope acts.bas_id: duplicate company_id/bas_id values exist';
    END IF;

    IF EXISTS (
        SELECT 1 FROM invoices WHERE bas_id IS NOT NULL GROUP BY company_id, bas_id HAVING COUNT(*) > 1
    ) THEN
        RAISE EXCEPTION 'Cannot scope invoices.bas_id: duplicate company_id/bas_id values exist';
    END IF;

    IF EXISTS (
        SELECT 1 FROM contracts WHERE bas_id IS NOT NULL GROUP BY company_id, bas_id HAVING COUNT(*) > 1
    ) THEN
        RAISE EXCEPTION 'Cannot scope contracts.bas_id: duplicate company_id/bas_id values exist';
    END IF;

    IF EXISTS (
        SELECT 1 FROM payments WHERE bas_id IS NOT NULL GROUP BY company_id, bas_id HAVING COUNT(*) > 1
    ) THEN
        RAISE EXCEPTION 'Cannot scope payments.bas_id: duplicate company_id/bas_id values exist';
    END IF;

    IF EXISTS (
        SELECT 1 FROM waybills WHERE bas_id IS NOT NULL GROUP BY company_id, bas_id HAVING COUNT(*) > 1
    ) THEN
        RAISE EXCEPTION 'Cannot scope waybills.bas_id: duplicate company_id/bas_id values exist';
    END IF;
END $$;

ALTER TABLE counterparties DROP CONSTRAINT IF EXISTS counterparties_bas_id_key;
ALTER TABLE acts DROP CONSTRAINT IF EXISTS acts_bas_id_key;
ALTER TABLE invoices DROP CONSTRAINT IF EXISTS invoices_bas_id_key;
ALTER TABLE contracts DROP CONSTRAINT IF EXISTS contracts_bas_id_key;
ALTER TABLE payments DROP CONSTRAINT IF EXISTS payments_bas_id_key;
ALTER TABLE waybills DROP CONSTRAINT IF EXISTS waybills_bas_id_key;

CREATE UNIQUE INDEX IF NOT EXISTS uq_counterparties_company_bas_id
    ON counterparties(company_id, bas_id)
    WHERE bas_id IS NOT NULL;

CREATE UNIQUE INDEX IF NOT EXISTS uq_acts_company_bas_id
    ON acts(company_id, bas_id)
    WHERE bas_id IS NOT NULL;

CREATE UNIQUE INDEX IF NOT EXISTS uq_invoices_company_bas_id
    ON invoices(company_id, bas_id)
    WHERE bas_id IS NOT NULL;

CREATE UNIQUE INDEX IF NOT EXISTS uq_contracts_company_bas_id
    ON contracts(company_id, bas_id)
    WHERE bas_id IS NOT NULL;

CREATE UNIQUE INDEX IF NOT EXISTS uq_payments_company_bas_id
    ON payments(company_id, bas_id)
    WHERE bas_id IS NOT NULL;

CREATE UNIQUE INDEX IF NOT EXISTS uq_waybills_company_bas_id
    ON waybills(company_id, bas_id)
    WHERE bas_id IS NOT NULL;
