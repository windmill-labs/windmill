-- Drop indexes first (though dropping table will cascade)
DROP INDEX IF EXISTS workspace_dependencies_id_workspace ;
DROP INDEX IF EXISTS workspace_dependencies_workspace_lang_name_archived_idx ;
DROP INDEX IF EXISTS workspace_dependencies_workspace_archived_idx;
DROP INDEX IF EXISTS one_non_archived_per_null_name_language_constraint;
DROP INDEX IF EXISTS one_non_archived_per_name_language_constraint;

-- Drop table and sequence
DROP TABLE IF EXISTS workspace_dependencies;
DROP SEQUENCE IF EXISTS workspace_dependencies_id_seq CASCADE;
