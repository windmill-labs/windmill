ALTER TABLE capture RENAME COLUMN trigger_extra to preprocessor_args;
ALTER TABLE capture RENAME COLUMN payload to main_args;