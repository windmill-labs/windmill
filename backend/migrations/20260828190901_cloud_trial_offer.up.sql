-- A pre-approved self-hosted Enterprise trial offered to an account created through a
-- pre-approved invite. No expiry: the offer lasts until a trial or subscription exists.
CREATE TABLE cloud_trial_offer (
    email VARCHAR(255) PRIMARY KEY REFERENCES password(email) ON DELETE CASCADE ON UPDATE CASCADE,
    consumed_at TIMESTAMPTZ,
    created_by VARCHAR(255) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
