#!/usr/bin/env bash
set -e 

docker run --rm -d \
  --name windmill-db-dev \
  -e POSTGRES_PASSWORD=changeme \
  -e POSTGRES_DB=windmill \
  -p 5432:5432 \
  -v windmill_db_data:/var/lib/postgresql/data \
  postgres:16

echo "PostgreSQL database started successfully!"
echo "Connection string: postgres://postgres:changeme@localhost:5432/windmill"
