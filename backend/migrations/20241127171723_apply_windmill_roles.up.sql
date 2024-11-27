-- for some managed dbs (e.g Azure Postgres) the role is not 
-- automatically applied to the user when created
GRANT windmill_user to CURRENT_USER;
GRANT windmill_admin to CURRENT_USER;