-- Single-use login links minted by a superadmin for one account. Consumed by an
-- unauthenticated GET that mints a session; the row is never a bearer credential itself.
CREATE TABLE login_link (
    token_hash CHAR(64) PRIMARY KEY,
    email VARCHAR(255) NOT NULL REFERENCES password(email) ON DELETE CASCADE ON UPDATE CASCADE,
    rd TEXT,
    expiration TIMESTAMPTZ NOT NULL,
    consumed_at TIMESTAMPTZ,
    created_by VARCHAR(255) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX login_link_email_idx ON login_link (email);
