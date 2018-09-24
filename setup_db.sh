#!/usr/bin/env bash

set -e

export PGPASSWORD=clickheretodie

psql -h db -U thulani -w -c "CREATE EXTENSION IF NOT EXISTS pgcrypto" memes
diesel migration run --database-url "postgres://thulani:clickheretodie@db/memes"
