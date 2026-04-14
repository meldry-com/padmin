#!/bin/bash
set -e

# Create the pasion database (palpo database is auto-created via POSTGRES_USER)
psql -v ON_ERROR_STOP=1 --username "$POSTGRES_USER" <<-EOSQL
    CREATE DATABASE pasion;
EOSQL
