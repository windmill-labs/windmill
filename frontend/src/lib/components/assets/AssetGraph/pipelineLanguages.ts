import type { ScriptLang } from '$lib/gen'

// Pipelines are dataset-shaped, so the menu surfaces the languages users
// actually reach for first: duckdb for in-place SQL on parquet/s3 (the
// default), bun for ergonomic data wrangling, python for ML/pandas, then
// the sql dialects for warehouse-resident transforms. Everything else
// (bash/go/deno) sits below — still creatable, just not the default
// suggestion.
export const PIPELINE_LANGUAGES: Array<{ label: string; lang: ScriptLang }> = [
	{ label: 'DuckDB', lang: 'duckdb' },
	{ label: 'TypeScript (Bun)', lang: 'bun' },
	{ label: 'Python', lang: 'python3' },
	{ label: 'PostgreSQL', lang: 'postgresql' },
	{ label: 'BigQuery', lang: 'bigquery' },
	{ label: 'Snowflake', lang: 'snowflake' },
	{ label: 'MySQL', lang: 'mysql' },
	{ label: 'MS SQL', lang: 'mssql' },
	{ label: 'Bash', lang: 'bash' },
	{ label: 'Go', lang: 'go' },
	{ label: 'Deno', lang: 'deno' }
]
