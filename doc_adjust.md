# Windmill Local Setup & Troubleshooting Adjustments

## Overview
This document outlines the steps taken to successfully set up and run Windmill locally using Docker Compose, including all adjustments and fixes made to resolve common issues.

---

## 1. Fresh Start & Cleanup
- Removed all existing Windmill files and Docker images to start from a clean state.
- Cloned the latest Windmill repository from GitHub.

## 2. Environment Configuration
- Restored the default `.env` file from the official Windmill repository.
- Set `DATABASE_URL` to:
  ```
  postgres://postgres:changeme@db:5432/windmill?sslmode=disable
  ```
- Added a comment to `.env` to clarify: if connecting from the host, use port `55432` instead of `5432`.

## 3. Docker Compose Adjustments
- Ensured all named volume references in `docker-compose.yml` matched the `volumes:` section (e.g., `db_data`, not `db-data`).
- Changed the database service port mapping to avoid conflicts with a local Postgres instance:
  ```yaml
  ports:
    - "55432:5432"
  ```
- Added a healthcheck to the `db` service to ensure dependent services only start when the database is ready:
  ```yaml
  healthcheck:
    test: ["CMD-SHELL", "pg_isready -U postgres"]
    interval: 10s
    timeout: 5s
    retries: 5
  ```
- Fixed the `windmill_server` service to include required environment variables:
  ```yaml
  environment:
    - DATABASE_URL=${DATABASE_URL}
    - BASE_URL=http://localhost:3002
  ```
- Fixed the Caddy service to ensure correct reverse proxying:
  - Set the `BASE_URL` environment variable to `:80` for Caddy.
  - Confirmed the `Caddyfile` proxies to `windmill_server:8000`.
  - Used port mapping `80:80` to expose the UI at `http://localhost`.

## 4. Data Directory Migration Error
- Encountered an error due to leftover database files from a previous Postgres 16 installation.
- Fixed by running:
  ```sh
  docker compose down -v
  ```
  to remove all old Docker volumes and start with a fresh database.

## 5. Service Startup & Verification
- Used `docker compose up -d` to start all services.
- Verified that all containers were running and healthy using:
  ```sh
  docker compose ps
  ```
- Checked logs for troubleshooting with:
  ```sh
  docker compose logs <service>
  ```

## 6. Accessing Windmill
- Windmill UI is available at: [http://localhost](http://localhost)
- Default admin credentials:
  - Username: `admin@windmill.dev`
  - Password: `changeme`

---

## Troubleshooting Tips
- If you see `ERR_CONNECTION_REFUSED`, check that Caddy and Windmill containers are running and healthy.
- If you see database errors, ensure the `DATABASE_URL` matches the running Postgres instance and that the volume is clean.
- For port conflicts, change the exposed ports in `docker-compose.yml`.
- Always check logs with `docker compose logs <service>` for detailed error messages.

---

## Summary
These steps resolved common setup and environment issues, resulting in a working local Windmill deployment using Docker Compose. Adjustments focused on environment variables, port mappings, Docker volumes, and healthchecks.
