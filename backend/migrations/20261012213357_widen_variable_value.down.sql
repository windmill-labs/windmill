DO $$
DECLARE
    too_long BIGINT;
BEGIN
    SELECT count(*) INTO too_long FROM variable WHERE length(value) > 15000;
    IF too_long > 0 THEN
        RAISE EXCEPTION 'cannot narrow variable.value to VARCHAR(15000): % row(s) are longer than 15000 characters', too_long;
    END IF;
END $$;

ALTER TABLE variable ALTER COLUMN value TYPE VARCHAR(15000);
