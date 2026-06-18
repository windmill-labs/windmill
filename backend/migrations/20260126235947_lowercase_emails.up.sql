-- Normalize emails to lowercase for consistency with external login normalization
-- This migration lowercases email columns in critical authentication tables

-- Lowercase emails in password table (primary user identity)
UPDATE password SET email = LOWER(email) WHERE email != LOWER(email);

-- Lowercase emails in usr table (workspace users)
UPDATE usr SET email = LOWER(email) WHERE email != LOWER(email);

-- Lowercase emails in email_to_igroup table (instance group memberships)
UPDATE email_to_igroup SET email = LOWER(email) WHERE email != LOWER(email);

-- Lowercase emails in token table (active sessions)
UPDATE token SET email = LOWER(email) WHERE email != LOWER(email);
