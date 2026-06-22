-- AND-join barrier slot state. For a `// trigger all` subscriber, each
-- partition-bearing input arrival (an `// on` asset whose declared path
-- contains the `{partition}` token) is recorded against the
-- (subscriber, partition) slot. The subscriber is dispatched once, for a
-- given partition, only when every partition-bearing input it declares has
-- arrived for that partition — skew-immune (unlike a debounce). The slot
-- is cleared on fire so later writes re-accumulate and can re-materialize.
--
-- trigger_ref stores the literal `{partition}`-token form (lineage is
-- partition-agnostic; the concrete value is the `partition` column),
-- matching how script_trigger / asset rows store it.
CREATE TABLE join_pending_inputs (
  workspace_id    VARCHAR(50) NOT NULL REFERENCES workspace(id) ON DELETE CASCADE ON UPDATE CASCADE,
  subscriber_path VARCHAR(255) NOT NULL,
  partition       TEXT NOT NULL,
  trigger_ref     TEXT NOT NULL,
  received_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
  PRIMARY KEY (workspace_id, subscriber_path, partition, trigger_ref)
);
