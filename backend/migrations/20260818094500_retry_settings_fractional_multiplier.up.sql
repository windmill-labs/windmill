-- The exponential backoff multiplier is a float; storing it as INTEGER rounded a
-- fractional multiplier to 0, and a 0 multiplier makes every retry delay 0 seconds.
ALTER TABLE retry_settings
ALTER COLUMN exponential_multiplier TYPE DOUBLE PRECISION;
