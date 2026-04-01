-- Міграція 010: необхідне розширення для gen_random_uuid()

CREATE EXTENSION IF NOT EXISTS pgcrypto;
