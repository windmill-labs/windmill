import type { DbType } from './components/apps/components/display/dbtable/utils'

export const DEFAULT_WEBHOOK_TYPE: 'async' | 'sync' = 'async'

export const HOME_SHOW_HUB = true

export const HOME_SHOW_CREATE_FLOW = true
export const HOME_SEARCH_SHOW_FLOW = true

export const HOME_SHOW_CREATE_APP = true

export const HOME_SEARCH_PLACEHOLDER = 'Search Scripts, Flows & Apps'

export const SIDEBAR_SHOW_SCHEDULES = true

export const WORKSPACE_SHOW_SLACK_CMD = true
export const WORKSPACE_SHOW_WEBHOOK_CLI_SYNC = true

export const SCRIPT_VIEW_SHOW_PUBLISH_TO_HUB = true

export const SCRIPT_VIEW_SHOW_SCHEDULE = true
export const SCRIPT_VIEW_SHOW_EXAMPLE_CURL = true

export const SCRIPT_VIEW_SHOW_CREATE_TOKEN_BUTTON = true

export const SCRIPT_VIEW_SHOW_RUN_FROM_CLI = true

export const SCRIPT_VIEW_SHOW_SCHEDULE_RUN_LATER = true

export const SCRIPT_VIEW_WEBHOOK_INFO_TIP = `Pass the input as a json payload, the token as a Bearer token (header: 'Authorization:
Bearer XXXX') or as query arg \`?token=XXX\`, and pass as header: 'Content-Type:
application/json'`

export const SCRIPT_VIEW_WEBHOOK_INFO_LINK = 'https://www.windmill.dev/docs/core_concepts/webhooks'

export const SCRIPT_EDITOR_SHOW_EXPLORE_OTHER_SCRIPTS = true

export const SCRIPT_CUSTOMISE_SHOW_KIND = true

export const WORKER_S3_BUCKET_SYNC_SETTING = 'worker_s3_bucket_sync'
export const CUSTOM_TAGS_SETTING = 'custom_tags'
export const DEFAULT_TAGS_PER_WORKSPACE_SETTING = 'default_tags_per_workspace'
export const DEFAULT_TAGS_WORKSPACES_SETTING = 'default_tags_workspaces'

export const WORKSPACE_SLACK_BOT_TOKEN_PATH = 'f/slack_bot/bot_token'

export const POSTGRES_TYPES = [
	'VARCHAR',
	'VARCHAR[]',
	'TEXT',
	'TEXT[]',
	'INT',
	'INT[]',
	'BIGINT',
	'BIGINT[]',
	'BOOL',
	'BOOL[]',
	'CHAR',
	'CHAR[]',
	'SMALLINT',
	'SMALLINT[]',
	'SMALLSERIAL',
	'SMALLSERIAL[]',
	'SERIAL',
	'SERIAL[]',
	'BIGSERIAL',
	'BIGSERIAL[]',
	'REAL',
	'REAL[]',
	'FLOAT8',
	'FLOAT8[]',
	'NUMERIC',
	'NUMERIC[]',
	'DECIMAL',
	'DECIMAL[]',
	'OID',
	'OID[]',
	'DATE',
	'DATE[]',
	'TIME',
	'TIME[]',
	'TIMETZ',
	'TIMETZ[]',
	'TIMESTAMP',
	'TIMESTAMP[]',
	'TIMESTAMPTZ',
	'TIMESTAMPTZ[]',
	'JSON',
	'JSON[]',
	'JSONB',
	'JSONB[]',
	'BYTEA',
	'BYTEA[]'
]

export const MYSQL_TYPES = [
	'varchar',
	'char',
	'bin',
	'varbinary',
	'blob',
	'text',
	'enum',
	'set',
	'int',
	'uint',
	'integer',
	'bool',
	'bit',
	'float',
	'real',
	'dec',
	'fixed',
	'date',
	'datetime',
	'timestamp',
	'time'
]

export const ORACLEDB_TYPES = [
	'varchar2',
	'char',
	'nchar',
	'nvarchar2',
	'clob',
	'nclob',
	'blob',
	'number',
	'float',
	'binary_float',
	'binary_double',
	'date',
	'timestamp',
	'long',
	'rowid',
	'urowid'
]
export const BIGQUERY_TYPES = [
	'string',
	'string[]',
	'bytes',
	'bytes[]',
	'json',
	'json[]',
	'timestamp',
	'timestamp[]',
	'date',
	'date[]',
	'time',
	'time[]',
	'datetime',
	'datetime[]',
	'integer',
	'integer[]',
	'int64',
	'int64[]',
	'float',
	'float[]',
	'float64',
	'float64[]',
	'numeric',
	'numeric[]',
	'bignumeric',
	'bignumeric[]',
	'bool',
	'bool[]'
]

export const SNOWFLAKE_TYPES = [
	'varchar',
	'binary',
	'date',
	'time',
	'timestamp',
	'int',
	'float',
	'boolean'
]

export const MSSQL_TYPES = [
	'char',
	'varchar',
	'text',
	'nchar',
	'nvarchar',
	'ntext',
	'binary',
	'varbinary',
	'image',
	'date',
	'datetime2',
	'datetime',
	'datetimeoffset',
	'smalldatetime',
	'time',
	'bigint',
	'int',
	'tinyint',
	'smallint',
	'float',
	'real',
	'numeric',
	'decimal',
	'bit'
]

export const DB_TYPES: Record<DbType, string[]> = {
	bigquery: BIGQUERY_TYPES,
	ms_sql_server: MSSQL_TYPES,
	mysql: MYSQL_TYPES,
	postgresql: POSTGRES_TYPES,
	snowflake: SNOWFLAKE_TYPES
}
