-- Workspace members granted through an instance group that no longer exists become manual
-- members. Group deletion used to leave both the workspace's auto_invite reference and the
-- members behind; under state-based reconciliation those members belong to zero configured
-- groups, so the first reconcile touching their workspace would otherwise remove them and
-- destroy their drafts, favorites, tokens and permissions. The original group name is kept
-- under 'migrated_from_instance_group' so admins can identify and prune them deliberately.
UPDATE usr
SET added_via = jsonb_build_object(
    'source', 'manual',
    'migrated_from_instance_group', added_via->>'group'
)
WHERE added_via->>'source' = 'instance_group'
  AND NOT EXISTS (
      SELECT 1 FROM instance_group ig WHERE ig.name = usr.added_via->>'group'
  );

-- Strip auto_invite references to groups that no longer exist, so a later group created
-- with the same name cannot silently re-acquire the mapping.
UPDATE workspace_settings
SET auto_invite = jsonb_set(
    jsonb_set(
        auto_invite,
        '{instance_groups}',
        COALESCE(
            (SELECT jsonb_agg(elem)
             FROM jsonb_array_elements(auto_invite->'instance_groups') elem
             WHERE EXISTS (SELECT 1 FROM instance_group ig WHERE ig.name = elem #>> '{}')),
            '[]'::jsonb
        )
    ),
    '{instance_groups_roles}',
    CASE WHEN jsonb_typeof(auto_invite->'instance_groups_roles') = 'object'
         THEN (SELECT COALESCE(jsonb_object_agg(key, value), '{}'::jsonb)
               FROM jsonb_each(auto_invite->'instance_groups_roles')
               WHERE EXISTS (SELECT 1 FROM instance_group ig WHERE ig.name = key))
         ELSE '{}'::jsonb
    END
)
WHERE jsonb_typeof(auto_invite->'instance_groups') = 'array'
  AND EXISTS (
      SELECT 1 FROM jsonb_array_elements(auto_invite->'instance_groups') elem
      WHERE NOT EXISTS (SELECT 1 FROM instance_group ig WHERE ig.name = elem #>> '{}')
  );
