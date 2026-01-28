-- Safely normalize emails to lowercase, handling duplicates
-- For password table: delete mixed-case versions if lowercase already exists, then lowercase the rest

-- Password table (email is primary key)
DELETE FROM password
WHERE email != LOWER(email)
  AND LOWER(email) IN (SELECT email FROM password WHERE email = LOWER(email));

UPDATE password SET email = LOWER(email) WHERE email != LOWER(email);

-- Usr table (composite key workspace_id + username, but email should be unique per workspace)
DELETE FROM usr
WHERE email != LOWER(email)
  AND EXISTS (
    SELECT 1 FROM usr u2
    WHERE u2.workspace_id = usr.workspace_id
      AND u2.email = LOWER(usr.email)
  );

UPDATE usr SET email = LOWER(email) WHERE email != LOWER(email);

-- Email_to_igroup table
DELETE FROM email_to_igroup
WHERE email != LOWER(email)
  AND EXISTS (
    SELECT 1 FROM email_to_igroup e2
    WHERE e2.igroup = email_to_igroup.igroup
      AND e2.email = LOWER(email_to_igroup.email)
  );

UPDATE email_to_igroup SET email = LOWER(email) WHERE email != LOWER(email);

-- Token table (token is primary key, email is just a column - duplicates are OK)
UPDATE token SET email = LOWER(email) WHERE email != LOWER(email);
