-- Restore the instance_group source for members the up migration converted. The stripped
-- auto_invite references cannot be restored (the groups they named no longer exist).
UPDATE usr
SET added_via = jsonb_build_object(
    'source', 'instance_group',
    'group', added_via->>'migrated_from_instance_group'
)
WHERE added_via->>'source' = 'manual'
  AND added_via ? 'migrated_from_instance_group';
