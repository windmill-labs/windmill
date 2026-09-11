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

-- Workspace members whose instance-group grant can no longer be re-derived become manual
-- members. Group deletion, overwrite imports and some SCIM paths used to mutate groups
-- without carrying the change through to workspace membership, leaving members whose
-- granting group was deleted — or who were dropped from a group that still exists. Under
-- state-based reconciliation those members belong to zero configured groups, so the first
-- reconcile touching their workspace would otherwise remove them and destroy their drafts,
-- favorites, tokens and permissions. The original group name is kept under
-- 'migrated_from_instance_group' so admins can identify and prune them deliberately.
UPDATE usr
SET added_via = jsonb_build_object(
    'source', 'manual',
    'migrated_from_instance_group', added_via->>'group'
)
WHERE added_via->>'source' = 'instance_group'
  AND NOT EXISTS (
      SELECT 1
      FROM workspace_settings ws
      JOIN LATERAL jsonb_array_elements_text(
          CASE WHEN jsonb_typeof(ws.auto_invite->'instance_groups') = 'array'
               THEN ws.auto_invite->'instance_groups'
               ELSE '[]'::jsonb
          END
      ) g ON true
      JOIN email_to_igroup e ON e.igroup = g.value AND e.email = usr.email
      WHERE ws.workspace_id = usr.workspace_id
  );
