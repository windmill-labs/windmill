import type { Script } from '$lib/gen'

export type LabelledLanguage = Script['language'] | 'bunnative' | 'claudesandbox' | 'pisandbox'

/** A language's name, as a tooltip or anywhere the full word fits. */
export const languageLabel: Record<LabelledLanguage, string> = {
	python3: 'Python',
	deno: 'TypeScript',
	go: 'Go',
	bash: 'Bash',
	powershell: 'PowerShell',
	nativets: 'HTTP',
	bunnative: 'HTTP',
	graphql: 'GraphQL',
	postgresql: 'Postgresql',
	bigquery: 'BigQuery',
	oracledb: 'Oracle Database',
	duckdb: 'DuckDB',
	snowflake: 'Snowflake',
	mysql: 'MySQL',
	mssql: 'MS SQL Server',
	bun: 'TypeScript',
	php: 'PHP',
	rust: 'Rust',
	ansible: 'Ansible',
	csharp: 'C#',
	nu: 'Nu',
	java: 'Java',
	ruby: 'Ruby',
	rlang: 'R',
	dbt: 'dbt',
	claudesandbox: 'Claude Sandbox',
	pisandbox: 'Pi Sandbox'
	// for related places search: ADD_NEW_LANG
}

/** A few letters for a list row, where a logo per row is more than the eye needs. Every SQL
 * dialect reads "SQL": which one is the tooltip's job (`languageLabel`). */
export const languageShortLabel: Record<LabelledLanguage, string> = {
	python3: 'Py',
	deno: 'TS',
	bun: 'TS',
	go: 'Go',
	bash: 'Sh',
	powershell: 'PS',
	nativets: 'Rest',
	bunnative: 'Rest',
	graphql: 'GQL',
	postgresql: 'SQL',
	bigquery: 'SQL',
	oracledb: 'SQL',
	duckdb: 'SQL',
	snowflake: 'SQL',
	mysql: 'SQL',
	mssql: 'SQL',
	php: 'PHP',
	rust: 'Rs',
	ansible: 'Ans',
	csharp: 'C#',
	nu: 'Nu',
	java: 'Java',
	ruby: 'Rb',
	rlang: 'R',
	dbt: 'dbt',
	claudesandbox: 'AI',
	pisandbox: 'AI'
	// for related places search: ADD_NEW_LANG
}
