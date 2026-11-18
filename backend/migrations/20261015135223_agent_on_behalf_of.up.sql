-- An AI agent's `run_on_behalf_of` makes runs from its page take the identity in its
-- `on_behalf_of`, which the server resolves on every write. Before this, both keys were ordinary
-- JSON any writer of the agent could set, so a value written then could name someone else's
-- identity unchecked: clear them, and every agent starts with the toggle off.
UPDATE resource
   SET value = value - 'on_behalf_of' - 'run_on_behalf_of'
 WHERE resource_type = 'ai_agent'
   AND jsonb_typeof(value) = 'object'
   AND (value ? 'on_behalf_of' OR value ? 'run_on_behalf_of');

-- A deleted agent comes back from the trash as stored, so its copy there is cleared too.
UPDATE trashbin
   SET item_data = jsonb_set(item_data, '{row,value}',
                             (item_data->'row'->'value') - 'on_behalf_of' - 'run_on_behalf_of')
 WHERE item_kind = 'resource'
   AND item_data->'row'->>'resource_type' = 'ai_agent'
   AND jsonb_typeof(item_data->'row'->'value') = 'object'
   AND (item_data->'row'->'value' ? 'on_behalf_of'
        OR item_data->'row'->'value' ? 'run_on_behalf_of');
