import type { DbType } from '$lib/components/dbTypes'

export function makeFetchTableEditorDefinitionQuery(dbType: DbType): string {
	if (dbType !== 'postgresql') {
		throw new Error(`makeFetchTableEditorDefinitionQuery only supports postgresql for now`)
	}
	const query = `-- result_collection=last_statement_first_row_scalar
-- $1 schema
-- $2 table
WITH table_info AS (
  SELECT
    c.table_schema,
    c.table_name,
    c.column_name,
    c.ordinal_position,
    c.column_default,
    c.is_nullable,
    c.data_type,
    c.character_maximum_length,
    c.numeric_precision,
    c.numeric_scale,
    c.udt_name,
    -- Get primary key information
    (SELECT kcu.constraint_name
     FROM information_schema.key_column_usage kcu
     JOIN information_schema.table_constraints tc
       ON kcu.constraint_name = tc.constraint_name
       AND kcu.table_schema = tc.table_schema
       AND kcu.table_name = tc.table_name
     WHERE tc.constraint_type = 'PRIMARY KEY'
       AND kcu.table_schema = c.table_schema
       AND kcu.table_name = c.table_name
       AND kcu.column_name = c.column_name
    ) AS pk_constraint_name
  FROM information_schema.columns c
  WHERE c.table_schema = $1
    AND c.table_name = $2
  ORDER BY c.ordinal_position
),
foreign_keys AS (
  SELECT
    tc.constraint_name,
    kcu.column_name AS source_column,
    ccu.table_schema AS target_schema,
    ccu.table_name AS target_table,
    ccu.column_name AS target_column,
    rc.update_rule,
    rc.delete_rule,
    kcu.ordinal_position
  FROM information_schema.table_constraints tc
  JOIN information_schema.key_column_usage kcu
    ON tc.constraint_name = kcu.constraint_name
    AND tc.table_schema = kcu.table_schema
  JOIN information_schema.constraint_column_usage ccu
    ON ccu.constraint_name = tc.constraint_name
    AND ccu.constraint_schema = tc.table_schema
  JOIN information_schema.referential_constraints rc
    ON rc.constraint_name = tc.constraint_name
    AND rc.constraint_schema = tc.table_schema
  WHERE tc.constraint_type = 'FOREIGN KEY'
    AND tc.table_schema = $1
    AND tc.table_name = $2
  ORDER BY tc.constraint_name, kcu.ordinal_position
),
columns_json AS (
  SELECT json_agg(
    json_build_object(
      'name', column_name,
      'initialName', column_name,
      'datatype', CASE
        -- Handle array types: udt_name starts with underscore for arrays
        WHEN data_type = 'ARRAY' THEN
          UPPER(SUBSTRING(udt_name FROM 2)) || '[]'
        -- Map PostgreSQL types to standard types
        WHEN data_type = 'character varying' THEN 'VARCHAR'
        WHEN data_type = 'character' THEN 'CHAR'
        WHEN data_type = 'timestamp without time zone' THEN 'TIMESTAMP'
        WHEN data_type = 'timestamp with time zone' THEN 'TIMESTAMPTZ'
        WHEN data_type = 'time without time zone' THEN 'TIME'
        ELSE UPPER(TRIM(data_type))
      END,
      'primaryKey', pk_constraint_name IS NOT NULL,
      'nullable', is_nullable = 'YES',
      'defaultValue', CASE
        WHEN column_default IS NULL THEN NULL
        ELSE '{' || column_default || '}'
      END,
      'datatype_length', character_maximum_length
    ) ORDER BY ordinal_position
  ) AS columns
  FROM table_info
),
foreign_keys_grouped AS (
  SELECT
    constraint_name,
    MAX(target_schema) as target_schema,
    MAX(target_table) as target_table,
    MAX(delete_rule) as delete_rule,
    MAX(update_rule) as update_rule,
    json_agg(
      json_build_object(
        'sourceColumn', source_column,
        'targetColumn', target_column
      ) ORDER BY ordinal_position
    ) as columns
  FROM foreign_keys
  GROUP BY constraint_name
),
foreign_keys_json AS (
  SELECT json_agg(
    json_build_object(
      'targetTable', target_schema || '.' || target_table,
      'columns', columns,
      'onDelete', CASE
        WHEN delete_rule = 'CASCADE' THEN 'CASCADE'
        WHEN delete_rule = 'SET NULL' THEN 'SET NULL'
        ELSE 'NO ACTION'
      END,
      'onUpdate', CASE
        WHEN update_rule = 'CASCADE' THEN 'CASCADE'
        WHEN update_rule = 'SET NULL' THEN 'SET NULL'
        ELSE 'NO ACTION'
      END,
      'fk_constraint_name', constraint_name
    )
  ) AS foreign_keys
  FROM foreign_keys_grouped
)
SELECT json_build_object(
  'name', $2,
  'columns', COALESCE((SELECT columns FROM columns_json), '[]'::json),
  'foreignKeys', COALESCE((SELECT foreign_keys FROM foreign_keys_json), '[]'::json),
  'pk_constraint_name', (SELECT MAX(pk_constraint_name) FROM table_info WHERE pk_constraint_name IS NOT NULL)
) AS table_definition;
`

	return query
}
