DO $$ BEGIN
    CREATE TYPE task_status AS ENUM ('open', 'in_progress', 'done', 'cancelled');
EXCEPTION WHEN duplicate_object THEN NULL;
END $$;

DO $$ BEGIN
    CREATE TYPE task_priority AS ENUM ('low', 'normal', 'high', 'critical');
EXCEPTION WHEN duplicate_object THEN NULL;
END $$;

CREATE TABLE tasks (
    id              UUID            PRIMARY KEY DEFAULT gen_random_uuid(),
    company_id      UUID            NOT NULL REFERENCES companies(id) ON DELETE RESTRICT,
    title           TEXT            NOT NULL,
    description     TEXT,
    status          task_status     NOT NULL DEFAULT 'open',
    priority        task_priority   NOT NULL DEFAULT 'normal',
    due_date        TIMESTAMPTZ,
    reminder_at     TIMESTAMPTZ,
    counterparty_id UUID            REFERENCES counterparties(id) ON DELETE CASCADE,
    act_id          UUID            REFERENCES acts(id) ON DELETE CASCADE,
    CONSTRAINT only_one_parent CHECK (
        (counterparty_id IS NOT NULL)::int +
        (act_id IS NOT NULL)::int <= 1
    ),
    created_at      TIMESTAMPTZ     NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ     NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_tasks_company      ON tasks(company_id);
CREATE INDEX idx_tasks_status       ON tasks(status);
CREATE INDEX idx_tasks_priority     ON tasks(priority, due_date);
CREATE INDEX idx_tasks_reminder     ON tasks(reminder_at) WHERE reminder_at IS NOT NULL;
CREATE INDEX idx_tasks_counterparty ON tasks(counterparty_id);
CREATE INDEX idx_tasks_act          ON tasks(act_id);
