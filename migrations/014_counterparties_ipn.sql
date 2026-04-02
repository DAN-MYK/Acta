-- Міграція 014: ІПН / РНОКПП для контрагентів

ALTER TABLE counterparties
    ADD COLUMN ipn VARCHAR(12);

CREATE INDEX idx_counterparties_ipn ON counterparties(ipn);
