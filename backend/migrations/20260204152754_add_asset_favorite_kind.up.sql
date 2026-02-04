-- Add up migration script here

DO
$do$
BEGIN
  ALTER TYPE FAVORITE_KIND ADD VALUE 'asset';
EXCEPTION WHEN OTHERS THEN
  RAISE NOTICE 'Couldn''t create FAVORITE_KIND::asset: %', SQLERRM;
END
$do$;
