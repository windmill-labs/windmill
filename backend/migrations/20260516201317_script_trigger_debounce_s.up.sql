-- Opt-in debounce window (seconds) for an asset-cascade subscriber edge.
-- NULL = no debounce (fan-out, the default/current behaviour). Resolved
-- per `// on` edge at deploy as: edge `debounce=<dur>` ?? script-level
-- `// debounce <dur>` ?? none. Only asset subscriber rows carry it; the
-- dispatcher builds DebouncingSettings keyed (subscriber, partition) so
-- distinct partitions never collapse and "latest in window" falls out.
ALTER TABLE script_trigger
  ADD COLUMN debounce_s INTEGER;
