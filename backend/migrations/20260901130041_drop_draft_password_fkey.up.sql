-- The delete and rename this cascaded are now explicit, at the sites that remove or rename an
-- account; `windmill_common::user_drafts::delete_drafts_of_email` carries the reasoning.
ALTER TABLE draft DROP CONSTRAINT IF EXISTS draft_password_fkey;
