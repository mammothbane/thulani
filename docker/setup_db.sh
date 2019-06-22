#!/usr/bin/env bash

set -e

if [ -z "$THULANI_PGPASS" ]; then
    echo "THULANI_PGPASS unset" >&2
    exit 1
fi

psql --command "CREATE USER thulani WITH PASSWORD '$THULANI_PGPASS'" 
createdb -O thulani memes
psql --command "CREATE EXTENSION IF NOT EXISTS pgcrypto" memes

diesel migration run --database-url "postgres:///memes" 
