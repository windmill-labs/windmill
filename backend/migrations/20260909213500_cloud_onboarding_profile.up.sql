-- Context an invite carried about the account's owner, written at provisioning and read by
-- onboarding to tailor itself (skip the source question it knows the answer to, later
-- template picks and starter prompts). Free-form JSON so new fields need no migration.
CREATE TABLE cloud_onboarding_profile (
    email VARCHAR(255) PRIMARY KEY REFERENCES password(email) ON DELETE CASCADE ON UPDATE CASCADE,
    profile JSONB NOT NULL,
    created_by VARCHAR(255) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
