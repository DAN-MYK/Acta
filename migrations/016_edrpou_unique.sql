-- Міграція 002: UNIQUE constraint на edrpou
-- ЄДРПОУ унікальний для кожного контрагента (один юрособа — один код)
-- NULL дозволено — фізособи без ЄДРПОУ можуть існувати паралельно

ALTER TABLE counterparties
    ADD CONSTRAINT uq_counterparties_edrpou UNIQUE (edrpou);
