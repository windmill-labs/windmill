-- An `ai_agent` resource's draft gets a kind of its own, so every draft listing
-- names an agent without reading the draft's value or the deployed row. The
-- drafts already stored as `resource` are retagged by the next migration: a
-- value added to an enum cannot be used in the transaction that adds it.
ALTER TYPE DRAFT_KIND ADD VALUE IF NOT EXISTS 'agent';
