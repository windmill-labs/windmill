CREATE TABLE scim_deactivated_user (
    email VARCHAR(255) PRIMARY KEY,
    deactivated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);
