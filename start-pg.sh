#!/usr/bin/env bash
# Запуск PostgreSQL встановленого через Scoop

PG_DATA="$(scoop prefix postgresql)/data"
PG_LOG="$PG_DATA/postgresql.log"

if pg_ctl status -D "$PG_DATA" > /dev/null 2>&1; then
    echo "PostgreSQL вже запущено."
else
    echo "Запускаємо PostgreSQL..."
    pg_ctl start -D "$PG_DATA" -l "$PG_LOG"
fi
