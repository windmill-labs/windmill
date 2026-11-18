-- exponential_multiplier stays INTEGER and keeps being written for workers that read only it.
ALTER TABLE retry_settings ADD COLUMN IF NOT EXISTS exponential_multiplier_float DOUBLE PRECISION;
