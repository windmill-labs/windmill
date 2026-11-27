-- Whether a token carries its owner's workspace admin rights. Existing rows and every insert that
-- does not name it keep them; `POST /users/tokens/create` withholds them unless asked.
ALTER TABLE token ADD COLUMN IF NOT EXISTS admin BOOLEAN NOT NULL DEFAULT true;
