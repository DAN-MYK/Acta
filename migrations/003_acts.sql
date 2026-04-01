-- Міграція 003: Акти виконаних робіт
--
-- act_status — нативний PostgreSQL ENUM, відображається на ActStatus у Rust.
-- Порядок статусів: draft → issued → signed → paid (лише вперед).

-- DO $$ BEGIN ... EXCEPTION WHEN duplicate_object —
-- захист від помилки якщо тип вже існує (повторний запуск міграції або dev-середовище).
DO $$ BEGIN
    CREATE TYPE act_status AS ENUM ('draft', 'issued', 'signed', 'paid');
EXCEPTION WHEN duplicate_object THEN
    NULL;  -- тип вже є — ігноруємо, продовжуємо
END $$;

CREATE TABLE acts (
    id               UUID           PRIMARY KEY DEFAULT gen_random_uuid(),
    number           VARCHAR(50)    NOT NULL,
    counterparty_id  UUID           NOT NULL REFERENCES counterparties(id),
    -- contract_id поки без FK — таблиця contracts буде в майбутній міграції
    contract_id      UUID,
    date             DATE           NOT NULL,
    total_amount     DECIMAL(15,2)  NOT NULL DEFAULT 0,
    status           act_status     NOT NULL DEFAULT 'draft',
    notes            TEXT,
    bas_id           VARCHAR(100)   UNIQUE,
    created_at       TIMESTAMPTZ    NOT NULL DEFAULT NOW(),
    updated_at       TIMESTAMPTZ    NOT NULL DEFAULT NOW()
);

-- Позиції акту: видаляються каскадно разом з актом
CREATE TABLE act_items (
    id           UUID           PRIMARY KEY DEFAULT gen_random_uuid(),
    act_id       UUID           NOT NULL REFERENCES acts(id) ON DELETE CASCADE,
    description  VARCHAR(1000)  NOT NULL,
    quantity     DECIMAL(15,4)  NOT NULL,       -- 4 знаки: для годин, кг тощо
    unit         VARCHAR(20)    NOT NULL DEFAULT 'шт',
    unit_price   DECIMAL(15,2)  NOT NULL,
    amount       DECIMAL(15,2)  NOT NULL,       -- quantity * unit_price, зберігається денормалізовано
    created_at   TIMESTAMPTZ    NOT NULL DEFAULT NOW(),
    updated_at   TIMESTAMPTZ    NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_acts_counterparty ON acts(counterparty_id);
CREATE INDEX idx_acts_status       ON acts(status);
CREATE INDEX idx_acts_date         ON acts(date DESC);
CREATE INDEX idx_act_items_act     ON act_items(act_id);
