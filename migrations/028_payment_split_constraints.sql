-- Міграція 028: інваріанти split-розподілу платежу
-- Гарантує, що сума частин (payment_acts + payment_invoices) ≤ payments.amount
-- та що жодна частина не має від'ємну/нульову суму. Працює як другий рівень
-- захисту після Rust-валідації у reconcile_split_scoped().

ALTER TABLE payment_acts
    ADD CONSTRAINT payment_acts_amount_positive CHECK (amount > 0) NOT VALID;

ALTER TABLE payment_invoices
    ADD CONSTRAINT payment_invoices_amount_positive CHECK (amount > 0) NOT VALID;

ALTER TABLE payment_acts VALIDATE CONSTRAINT payment_acts_amount_positive;
ALTER TABLE payment_invoices VALIDATE CONSTRAINT payment_invoices_amount_positive;

CREATE OR REPLACE FUNCTION ensure_payment_allocation_within_total()
RETURNS TRIGGER AS $$
DECLARE
    v_payment_amount DECIMAL(15,2);
    v_acts_total     DECIMAL(15,2);
    v_invoices_total DECIMAL(15,2);
BEGIN
    SELECT amount INTO v_payment_amount
    FROM payments
    WHERE id = NEW.payment_id;

    IF v_payment_amount IS NULL THEN
        -- Платіж не знайдено — FK constraint впорається з цим окремо.
        RETURN NEW;
    END IF;

    SELECT COALESCE(SUM(amount), 0) INTO v_acts_total
    FROM payment_acts
    WHERE payment_id = NEW.payment_id;

    SELECT COALESCE(SUM(amount), 0) INTO v_invoices_total
    FROM payment_invoices
    WHERE payment_id = NEW.payment_id;

    IF (v_acts_total + v_invoices_total) > v_payment_amount THEN
        RAISE EXCEPTION 'Сума частин розподілу (%, %) перевищує суму платежу %',
            v_acts_total, v_invoices_total, v_payment_amount
            USING ERRCODE = 'check_violation';
    END IF;

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS payment_acts_check_total ON payment_acts;
CREATE CONSTRAINT TRIGGER payment_acts_check_total
    AFTER INSERT OR UPDATE ON payment_acts
    DEFERRABLE INITIALLY DEFERRED
    FOR EACH ROW EXECUTE FUNCTION ensure_payment_allocation_within_total();

DROP TRIGGER IF EXISTS payment_invoices_check_total ON payment_invoices;
CREATE CONSTRAINT TRIGGER payment_invoices_check_total
    AFTER INSERT OR UPDATE ON payment_invoices
    DEFERRABLE INITIALLY DEFERRED
    FOR EACH ROW EXECUTE FUNCTION ensure_payment_allocation_within_total();
