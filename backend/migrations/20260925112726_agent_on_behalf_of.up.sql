-- An AI agent's value carries `run_on_behalf_of`, which makes runs from its page take the
-- identity in its `on_behalf_of`, as a flow's `on_behalf_of` does. The server resolves that
-- identity on every write of the agent. Before this, both keys were ordinary JSON any writer of
-- the agent could set, so neither is trusted: every agent starts with the toggle off and no
-- identity, running as its caller as it did.

-- Clearing keys nobody could have relied on is not an edit of the agent, so it records no version.
ALTER TABLE resource DISABLE TRIGGER record_resource_version_update_trigger;

UPDATE resource
   SET value = value - 'on_behalf_of' - 'run_on_behalf_of'
 WHERE resource_type = 'ai_agent'
   AND jsonb_typeof(value) = 'object'
   AND (value ? 'on_behalf_of' OR value ? 'run_on_behalf_of');

ALTER TABLE resource ENABLE TRIGGER record_resource_version_update_trigger;
