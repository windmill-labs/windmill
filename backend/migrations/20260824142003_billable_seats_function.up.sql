-- One definition of a workspace's billable seats, shared by every consumer: the backend's
-- `count_paid_seats`, the workspace settings and sidebar meters that render it, and the
-- out-of-band billing job that invoices it. Those disagreeing is what bills a workspace for seats
-- the product never credits it for.

-- Who is billable. Service accounts cannot log in and do not take a seat; a disabled member is
-- not billed either.
-- This is the cloud, per-workspace rule. EE license seats are a different model and are counted by
-- `count_offline_potential_seats`: distinct emails instance-wide, author-anywhere-wins, pending
-- invites included. Reusing this view for those would quietly under-count them.
CREATE OR REPLACE VIEW billable_member AS
    SELECT workspace_id, email, username, operator
    FROM usr
    WHERE NOT disabled AND NOT is_service_account;

-- Objects created after the one-time GRANT ALL in 20250205131523 need explicit grants: ALTER
-- DEFAULT PRIVILEGES only covers objects created by the role that set them. Without these the
-- application role cannot read the view, and every caller of `billable_seats()` fails.
GRANT ALL ON billable_member TO windmill_user;
GRANT ALL ON billable_member TO windmill_admin;

-- How billable members become seats: a developer is a whole seat, an operator is half of one.
CREATE OR REPLACE FUNCTION billable_seats(w_id TEXT) RETURNS BIGINT AS $$
    SELECT CEIL(
        COUNT(*) FILTER (WHERE NOT operator)
        + 0.5 * COUNT(*) FILTER (WHERE operator)
    )::BIGINT
    FROM billable_member
    WHERE workspace_id = w_id;
$$ LANGUAGE SQL STABLE;
