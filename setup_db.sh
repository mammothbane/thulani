#!/usr/bin/env bash

set -euo pipefail

export PGPASSWORD=clickheretodie

psql -h db -U thulani -w -c "CREATE EXTENSION IF NOT EXISTS pgcrypto" memes
diesel migration run --database-url "postgres://thulani:clickheretodie@db/memes"
