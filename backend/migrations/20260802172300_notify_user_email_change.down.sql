-- Add down migration script here
DROP TRIGGER IF EXISTS password_superadmin_delete_trigger ON password;
DROP TRIGGER IF EXISTS password_superadmin_insert_trigger ON password;
DROP TRIGGER IF EXISTS password_superadmin_update_trigger ON password;
DROP TRIGGER IF EXISTS usr_email_update_trigger ON usr;
DROP TRIGGER IF EXISTS usr_email_change_trigger ON usr;
DROP FUNCTION IF EXISTS notify_superadmin_identity_change();
DROP FUNCTION IF EXISTS notify_usr_email_change();
