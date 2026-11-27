-- Whether a token carries its owner's workspace admin rights. Every insert names it; the default
-- covers only rows written before it existed and servers that predate it during an upgrade, which
-- must keep minting admin-capable sessions. Flip it to false once no such server can be running.
ALTER TABLE token ADD COLUMN IF NOT EXISTS admin BOOLEAN NOT NULL DEFAULT true;
