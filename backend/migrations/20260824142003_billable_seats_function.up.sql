-- One definition of a workspace's billable seats, shared by every consumer: the backend's
-- `count_paid_seats`, the workspace settings and sidebar meters that render it, and the
-- out-of-band billing job that invoices it. Those disagreeing is what bills a workspace for seats
-- the product never credits it for.

-- Who is billable. Service accounts cannot log in and do not take a seat; a disabled member is
-- not billed either.
CREATE OR REPLACE VIEW billable_member AS
    SELECT workspace_id, email, username, operator
    FROM usr
    WHERE NOT disabled AND NOT is_service_account;

-- How billable members become seats: a developer is a whole seat, an operator is half of one.
CREATE OR REPLACE FUNCTION billable_seats(w_id TEXT) RETURNS BIGINT AS $$
    SELECT CEIL(
        COUNT(*) FILTER (WHERE NOT operator)
        + 0.5 * COUNT(*) FILTER (WHERE operator)
    )::BIGINT
    FROM billable_member
    WHERE workspace_id = w_id;
$$ LANGUAGE SQL STABLE;
