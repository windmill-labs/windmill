-- azure_trigger.email duplicated an address that permissioned_as already
-- determines, and which every other trigger table derives at fire time.

ALTER TABLE azure_trigger DROP COLUMN email;
