-- A draft's owner is any principal the instance authenticates, not only one holding a login
-- account: an external JWT names its subject by address and `password` never gets a row for it,
-- so the foreign key rejected those users' drafts outright. `usr.email` — workspace membership —
-- has never had one either. The delete/rename cascades it provided are now explicit at the sites
-- that remove or rename an account (see `delete_drafts_of_email` / `rename_drafts_of_email`).
ALTER TABLE draft DROP CONSTRAINT IF EXISTS draft_password_fkey;
