-- Lossy: a fractional multiplier is rounded away on the way back to INTEGER.
ALTER TABLE retry_settings
ALTER COLUMN exponential_multiplier TYPE INTEGER USING exponential_multiplier::INTEGER;
