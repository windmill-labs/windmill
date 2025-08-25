-- Add up migration script here
create table alerts (
  id serial PRIMARY KEY,
  alert_type varchar(50) NOT NULL,
  message text NOT NULL,
  created_at timestamptz NOT NULL DEFAULT CURRENT_TIMESTAMP
);

create table healthchecks (
  id bigserial PRIMARY KEY,
  check_type varchar(50) NOT NULL,
  healthy boolean NOT NULL,
  created_at timestamptz NOT NULL DEFAULT CURRENT_TIMESTAMP
);

create index healthchecks_check_type_created_at on healthchecks(check_type, created_at);
