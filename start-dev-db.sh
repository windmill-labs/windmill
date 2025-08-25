docker run --rm -d \
  --name windmill-db-dev \
  -e POSTGRES_PASSWORD=changeme \
  -e POSTGRES_DB=windmill \
  -p 5432:5432 \
  -v windmill_db_data:/var/lib/postgresql/data \
  postgres:16
