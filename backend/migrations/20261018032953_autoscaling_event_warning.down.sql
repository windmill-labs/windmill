-- PostgreSQL cannot drop an enum value; 'warning' stays on AUTOSCALING_EVENT_TYPE.
-- Binaries older than it fail to decode a 'warning' row in their cooldown query, which aborts
-- their autoscaling tick, so the rows go.
DELETE FROM autoscaling_event WHERE event_type = 'warning';
