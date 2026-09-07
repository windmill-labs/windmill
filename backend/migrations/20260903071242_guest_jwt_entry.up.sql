-- A second way in for a guest: a JWT minted by the embedding customer's own backend and
-- verified against a key the workspace admin configured. One key shape per workspace,
-- a PEM public key or a JWKS URL, never both: a token is verified against exactly one
-- source, and two would make "which one refused it" undiagnosable.
ALTER TABLE workspace_settings
    ADD COLUMN guest_jwt_public_key TEXT,
    ADD COLUMN guest_jwt_jwks_url TEXT,
    ADD CONSTRAINT workspace_settings_guest_jwt_one_key
        CHECK (guest_jwt_public_key IS NULL OR guest_jwt_jwks_url IS NULL);

-- Whether the guest came in on a JWT that day (as opposed to, or as well as, an
-- identity-provider sign-in). The seat telemetry reports the two entries apart, since
-- an app-only user routed through a guest JWT is one that `jwt_ext_` would have counted.
ALTER TABLE guest_activity
    ADD COLUMN jwt_entry BOOLEAN NOT NULL DEFAULT false;
