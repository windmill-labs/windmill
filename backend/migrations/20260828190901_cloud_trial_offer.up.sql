-- A pre-approved self-hosted Enterprise trial offered to an account created through a
-- pre-approved invite. No expiry: the offer lasts until a trial or subscription exists.
-- The cascade follows the account out on deletion and rename. A superadmin users-import
-- replaces every account by deleting and reinserting it, which takes these rows with it:
-- the offers, like the onboarding profiles, are recorded by the portal that minted them.
CREATE TABLE cloud_trial_offer (
    email VARCHAR(255) PRIMARY KEY REFERENCES password(email) ON DELETE CASCADE ON UPDATE CASCADE,
    consumed_at TIMESTAMPTZ,
    created_by VARCHAR(255) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
