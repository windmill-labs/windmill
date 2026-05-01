import type { ScriptLang, AssetKind } from '$lib/gen'

// What kind of asset the new script will produce. Drives the auto-generated
// output annotation, the random output path scheme, and the body skeleton.
//
// `none` is the conservative default — no output asset annotation, body just
// has a "fill in" comment. The other kinds inject their respective wmill SDK
// calls / SQL setup so the script is runnable (modulo schema definition) the
// moment it's created.
export type PipelineOutputKind = 'none' | 'datatable' | 'ducklake' | 's3_parquet' | 's3_object'

export type PipelineOutputKindMeta = {
	id: PipelineOutputKind
	label: string
	description: string
}

// The order here is the order shown in the picker. Datatable + ducklake are
// the dataset-shaped kinds we want users to gravitate toward; s3 parquet/
// object are the escape hatches for arbitrary blobs; none is last because
// picking it disables the whole "auto-generated output" feature.
export const PIPELINE_OUTPUT_KINDS: PipelineOutputKindMeta[] = [
	{
		id: 'datatable',
		label: 'DataTable',
		description: 'Postgres-backed typed table'
	},
	{
		id: 'ducklake',
		label: 'DuckLake',
		description: 'DuckDB lakehouse table'
	},
	{
		id: 's3_parquet',
		label: 'S3 Parquet',
		description: 'Columnar file in object storage'
	},
	{
		id: 's3_object',
		label: 'S3 Object',
		description: 'Generic file (JSON/CSV/binary)'
	},
	{
		id: 'none',
		label: 'No output',
		description: 'Side-effect only / fill in manually'
	}
]

// Per-language compatibility — output kinds that have a hand-rolled template
// for `lang`. Falling outside this list means the picker grays out the kind
// (and the generator falls back to `none`-like behavior). The DuckDB lang
// ducks both datatable + ducklake natively via attached catalogs; the
// non-postgres warehouse SQL dialects don't get datatable templates because
// their wmill SDK story doesn't include cross-dialect bridge scripts.
const LANG_COMPATIBILITY: Record<ScriptLang, PipelineOutputKind[]> = {
	bun: ['datatable', 'ducklake', 's3_parquet', 's3_object', 'none'],
	deno: ['datatable', 'ducklake', 's3_parquet', 's3_object', 'none'],
	python3: ['datatable', 'ducklake', 's3_parquet', 's3_object', 'none'],
	duckdb: ['datatable', 'ducklake', 's3_parquet', 's3_object', 'none'],
	postgresql: ['datatable', 'none'],
	mysql: ['none'],
	mssql: ['none'],
	bigquery: ['none'],
	snowflake: ['none'],
	oracledb: ['none'],
	bash: ['s3_object', 'none'],
	go: ['none'],
	rust: ['none'],
	php: ['none'],
	powershell: ['s3_object', 'none'],
	nu: ['none'],
	ansible: ['none'],
	java: ['none'],
	csharp: ['none'],
	graphql: ['none'],
	bunnative: ['none'],
	nativets: ['none']
} as any

export function compatibleOutputKinds(lang: ScriptLang): PipelineOutputKind[] {
	return LANG_COMPATIBILITY[lang] ?? ['none']
}

// Generates the random suffix portion of an auto-named output asset. Same
// 7-char Crockford-ish alphabet we use for script paths so the slugs read
// consistently.
function randomSlug(len = 7): string {
	const a = 'abcdefghijklmnopqrstuvwxyz0123456789'
	let out = ''
	for (let i = 0; i < len; i++) out += a[Math.floor(Math.random() * a.length)]
	return out
}

// Fully-qualified asset URI for a freshly-created output, given the picked
// kind. The path is auto-generated to land under `pipelines/<folder>/` for
// s3 / parquet, and uses the more conventional `main/<name>` form for
// datatable / ducklake (matches what the schema/preview UI expects). The
// name suffix is randomized to avoid collisions when the user adds multiple
// downstream scripts in one session.
export function autoOutputAsset(
	kind: PipelineOutputKind,
	folder: string,
	language?: ScriptLang
): { kind: AssetKind; path: string } | undefined {
	const slug = randomSlug()
	switch (kind) {
		case 'datatable':
			return { kind: 'datatable', path: `main/${folder}_${slug}` }
		case 'ducklake':
			return { kind: 'ducklake', path: `main/${folder}_${slug}` }
		case 's3_parquet':
			return { kind: 's3object', path: `pipelines/${folder}/out_${slug}.parquet` }
		case 's3_object': {
			// duckdb's natural output for a generic blob is CSV (one COPY TO
			// statement). TS/Python templates serialize JSON, so default to
			// .json for those — keeps the body and the file extension in
			// sync without the user having to rename anything.
			const ext = language === 'duckdb' ? 'csv' : 'json'
			return { kind: 's3object', path: `pipelines/${folder}/out_${slug}.${ext}` }
		}
		case 'none':
			return undefined
	}
}

// URI prefix used in `// on <ref>` annotations — matches ASSET_PREFIX in
// the page-level handler. Duplicated here so the templates module is
// self-contained (no circular import with the pipeline page).
const ASSET_URI_PREFIX: Record<AssetKind, string> = {
	s3object: 's3://',
	resource: '$res:',
	ducklake: 'ducklake://',
	datatable: 'datatable://',
	volume: 'volume://'
}

export function assetUri(asset: { kind: AssetKind; path: string }): string {
	return `${ASSET_URI_PREFIX[asset.kind]}${asset.path}`
}

// Comment prefix per language; mirrors what the parser accepts.
function commentPrefix(lang: ScriptLang): string {
	switch (lang) {
		case 'python3':
		case 'bash':
		case 'powershell':
		case 'nu':
		case 'ansible':
			return '#'
		case 'postgresql':
		case 'mysql':
		case 'bigquery':
		case 'snowflake':
		case 'mssql':
		case 'oracledb':
		case 'duckdb':
			return '--'
		default:
			return '//'
	}
}

export type DraftTriggerSource =
	| { kind: 'schedule'; cron: string }
	| { kind: 'asset'; ref: string }
	| {
			kind: 'webhook' | 'email' | 'kafka' | 'mqtt' | 'nats' | 'postgres' | 'sqs' | 'gcp'
			path: string | undefined
	  }

export type TemplateContext = {
	language: ScriptLang
	outputKind: PipelineOutputKind
	output?: { kind: AssetKind; path: string }
	// The upstream asset that triggered creation of this script (if any).
	// When present, the body loads from it; the `// on <ref>` annotation is
	// emitted regardless via the trigger source list passed to header().
	input?: { kind: AssetKind; path: string }
	triggers: DraftTriggerSource[]
}

// Header: `// pipeline` + every trigger source as its own annotation line.
// Output asset is NOT declared here — it's reconstructed from the body's
// SDK calls / SQL by the asset parser, same as production scripts.
function header(language: ScriptLang, triggers: DraftTriggerSource[]): string {
	const p = commentPrefix(language)
	const lines = triggers.map((t) => {
		switch (t.kind) {
			case 'schedule':
				return `${p} schedule "${t.cron}"`
			case 'asset':
				return `${p} on ${t.ref}`
			default:
				return `${p} on ${t.kind} ${t.path ?? '<trigger-path>'}`
		}
	})
	// Discoverability hint — the three annotations users most often miss
	// when authoring their first pipeline script. Single line, real
	// example values (not placeholders) so users see the syntax. Docs
	// link is the canonical reference once they want the details.
	const more = `${p} More: partitioned daily, freshness 1h, retry 3, tag heavy — https://www.windmill.dev/docs/pipelines/annotations`
	return [`${p} pipeline`, ...lines, more, ''].join('\n')
}

// Bun / Deno bodies. These share the wmill SDK surface, so we treat them
// uniformly. The differences vs the previous generic body:
//   - input asset → real `wmill.loadS3File` / `wmill.datatable(...).fetch()`
//   - output asset → real `wmill.writeS3File` / `wmill.datatable(...).fetch()`
//   - no module-scope OUT constant — the parser reads write paths from the
//     SDK call sites directly, so the constant was redundant.
function bodyTs(ctx: TemplateContext): string {
	const { input, output, outputKind } = ctx
	const importLine = `import * as wmill from "windmill-client"\n`

	const inputBlock = (() => {
		if (!input) return ''
		switch (input.kind) {
			case 's3object':
				return [
					`  // Upstream: ${assetUri(input)}`,
					`  const buf = await wmill.loadS3File({ s3: ${JSON.stringify(input.path)} })`,
					`  const rows = JSON.parse(new TextDecoder().decode(buf))`,
					``
				].join('\n')
			case 'datatable':
				return [
					`  // Upstream: ${assetUri(input)}`,
					`  const src = wmill.datatable(${JSON.stringify(input.path.split('/')[0] ?? 'main')})`,
					`  const rows = await src\`SELECT * FROM ${input.path.split('/').slice(1).join('_') || 'table_name'}\`.fetch()`,
					``
				].join('\n')
			case 'ducklake':
				return [
					`  // Upstream: ${assetUri(input)}`,
					`  const lake = wmill.ducklake(${JSON.stringify(input.path.split('/')[0] ?? 'main')})`,
					`  const rows = await lake\`SELECT * FROM ${input.path.split('/').slice(1).join('_') || 'table_name'}\`.fetch()`,
					``
				].join('\n')
			default:
				return `  // Upstream: ${assetUri(input)}\n`
		}
	})()

	const outputBlock = (() => {
		if (!output) return `  // (no output asset — fill in side-effect logic)`
		switch (outputKind) {
			case 's3_parquet':
			case 's3_object':
				return [
					`  // Output: ${assetUri(output)}`,
					`  const payload = new TextEncoder().encode(JSON.stringify(rows))`,
					`  await wmill.writeS3File({ s3: ${JSON.stringify(output.path)} }, payload)`
				].join('\n')
			case 'datatable': {
				const dbName = output.path.split('/')[0] ?? 'main'
				const tableName = output.path.split('/').slice(1).join('_') || 'table_name'
				return [
					`  // Output: ${assetUri(output)}`,
					`  const dst = wmill.datatable(${JSON.stringify(dbName)})`,
					`  await dst\`CREATE TABLE IF NOT EXISTS ${tableName} (id serial primary key, payload jsonb)\`.fetch()`,
					`  for (const r of rows) {`,
					`    await dst\`INSERT INTO ${tableName} (payload) VALUES (\${JSON.stringify(r)}::jsonb)\`.fetch()`,
					`  }`
				].join('\n')
			}
			case 'ducklake': {
				const dbName = output.path.split('/')[0] ?? 'main'
				const tableName = output.path.split('/').slice(1).join('_') || 'table_name'
				return [
					`  // Output: ${assetUri(output)}`,
					`  const lakeOut = wmill.ducklake(${JSON.stringify(dbName)})`,
					`  await lakeOut\`CREATE TABLE IF NOT EXISTS ${tableName} (payload JSON)\`.fetch()`,
					`  for (const r of rows) {`,
					`    await lakeOut\`INSERT INTO ${tableName} VALUES (\${JSON.stringify(r)})\`.fetch()`,
					`  }`
				].join('\n')
			}
			default:
				return `  // (fill in)`
		}
	})()

	const rowsFallback = !input ? `  const rows: any[] = []\n` : ''

	return [
		importLine,
		'export async function main() {',
		inputBlock,
		rowsFallback,
		outputBlock,
		'}',
		''
	].join('\n')
}

function bodyPython(ctx: TemplateContext): string {
	const { input, output, outputKind } = ctx
	const importLine = `import wmill\n`

	const inputBlock = (() => {
		if (!input) return ''
		switch (input.kind) {
			case 's3object':
				return [
					`    # Upstream: ${assetUri(input)}`,
					`    buf = wmill.load_s3_file(${JSON.stringify(input.path)})`,
					`    import json; rows = json.loads(buf.decode("utf-8"))`
				].join('\n')
			case 'datatable':
				return [
					`    # Upstream: ${assetUri(input)}`,
					`    src = wmill.datatable(${JSON.stringify(input.path.split('/')[0] ?? 'main')})`,
					`    rows = src.query("SELECT * FROM ${input.path.split('/').slice(1).join('_') || 'table_name'}").fetch()`
				].join('\n')
			case 'ducklake':
				return [
					`    # Upstream: ${assetUri(input)}`,
					`    lake = wmill.ducklake(${JSON.stringify(input.path.split('/')[0] ?? 'main')})`,
					`    rows = lake.query("SELECT * FROM ${input.path.split('/').slice(1).join('_') || 'table_name'}").fetch()`
				].join('\n')
			default:
				return `    # Upstream: ${assetUri(input)}`
		}
	})()

	const outputBlock = (() => {
		if (!output) return `    # (no output asset — fill in side-effect logic)`
		switch (outputKind) {
			case 's3_parquet':
			case 's3_object':
				return [
					`    # Output: ${assetUri(output)}`,
					`    import json`,
					`    wmill.write_s3_file(${JSON.stringify(output.path)}, json.dumps(rows).encode("utf-8"))`
				].join('\n')
			case 'datatable': {
				const dbName = output.path.split('/')[0] ?? 'main'
				const tableName = output.path.split('/').slice(1).join('_') || 'table_name'
				return [
					`    # Output: ${assetUri(output)}`,
					`    dst = wmill.datatable(${JSON.stringify(dbName)})`,
					`    dst.query("CREATE TABLE IF NOT EXISTS ${tableName} (id serial primary key, payload jsonb)").execute()`,
					`    for r in rows:`,
					`        dst.query("INSERT INTO ${tableName} (payload) VALUES ($1::jsonb)", json.dumps(r)).execute()`
				].join('\n')
			}
			case 'ducklake': {
				const dbName = output.path.split('/')[0] ?? 'main'
				const tableName = output.path.split('/').slice(1).join('_') || 'table_name'
				return [
					`    # Output: ${assetUri(output)}`,
					`    lake_out = wmill.ducklake(${JSON.stringify(dbName)})`,
					`    lake_out.query("CREATE TABLE IF NOT EXISTS ${tableName} (payload JSON)").execute()`,
					`    for r in rows:`,
					`        lake_out.query("INSERT INTO ${tableName} VALUES ($payload)", payload=json.dumps(r)).execute()`
				].join('\n')
			}
			default:
				return `    # (fill in)`
		}
	})()

	const rowsFallback = !input ? `    rows: list = []` : ''

	return [importLine, 'def main():', inputBlock, rowsFallback, outputBlock, ''].join('\n')
}

function bodyDuckdb(ctx: TemplateContext): string {
	const { input, output, outputKind } = ctx
	const lines: string[] = []
	if (input) lines.push(`-- Upstream: ${assetUri(input)}`)
	if (output) lines.push(`-- Output: ${assetUri(output)}`)
	lines.push('')

	const inSql = (() => {
		if (!input) return null
		switch (input.kind) {
			case 's3object':
				return `read_parquet('s3://${input.path}')`
			case 'ducklake': {
				return null // attach + read shown below
			}
			default:
				return null
		}
	})()

	if (input?.kind === 'ducklake') {
		lines.push(
			`-- DuckLake catalog is auto-attached as 'lake' inside windmill duckdb scripts.`,
			`-- Reference upstream tables as lake.<table_name>.`
		)
	}

	switch (outputKind) {
		case 's3_parquet':
			if (output) {
				const fromExpr = inSql ?? '(SELECT 1 AS placeholder)'
				lines.push(
					`COPY (`,
					`  SELECT *`,
					`  FROM ${fromExpr}`,
					`) TO 's3://${output.path}' (FORMAT 'parquet');`
				)
			}
			break
		case 's3_object':
			if (output) {
				const fromExpr = inSql ?? '(SELECT 1 AS placeholder)'
				lines.push(
					`COPY (`,
					`  SELECT *`,
					`  FROM ${fromExpr}`,
					`) TO 's3://${output.path}' (FORMAT 'csv', HEADER);`
				)
			}
			break
		case 'ducklake':
			if (output) {
				const tableName = output.path.split('/').slice(1).join('_') || 'table_name'
				lines.push(
					`-- Write into the auto-attached DuckLake catalog.`,
					`CREATE TABLE IF NOT EXISTS lake.${tableName} AS`,
					`SELECT * FROM ${inSql ?? '(SELECT 1 AS placeholder)'};`
				)
			}
			break
		case 'datatable':
			if (output) {
				const tableName = output.path.split('/').slice(1).join('_') || 'table_name'
				lines.push(
					`-- DataTable is exposed as the 'pg' attached database in duckdb scripts.`,
					`CREATE TABLE IF NOT EXISTS pg.${tableName} AS`,
					`SELECT * FROM ${inSql ?? '(SELECT 1 AS placeholder)'};`
				)
			}
			break
		case 'none':
		default:
			lines.push(`SELECT 1;`)
	}
	lines.push('')
	return lines.join('\n')
}

function bodyPostgres(ctx: TemplateContext): string {
	const { input, output, outputKind } = ctx
	const lines: string[] = []
	if (input) lines.push(`-- Upstream: ${assetUri(input)}`)
	if (output) lines.push(`-- Output: ${assetUri(output)}`)
	lines.push('')

	if (outputKind === 'datatable' && output) {
		const tableName = output.path.split('/').slice(1).join('_') || 'table_name'
		lines.push(
			`-- The pipeline-script harness runs this query against the`,
			`-- datatable backing the configured workspace storage.`,
			`CREATE TABLE IF NOT EXISTS ${tableName} (`,
			`  id serial primary key,`,
			`  payload jsonb,`,
			`  computed_at timestamptz default now()`,
			`);`,
			``,
			`-- Replace with your real INSERT.`,
			`INSERT INTO ${tableName} (payload) VALUES ('{}'::jsonb);`
		)
	} else {
		lines.push(`SELECT 1;`)
	}
	lines.push('')
	return lines.join('\n')
}

function bodyBash(ctx: TemplateContext): string {
	const { input, output, outputKind } = ctx
	const lines: string[] = []
	if (input) lines.push(`# Upstream: ${assetUri(input)}`)
	if (output) lines.push(`# Output: ${assetUri(output)}`)
	if (outputKind === 's3_object' && output) {
		lines.push(
			``,
			`# Use the wmill CLI / curl to upload to ${output.path}`,
			`echo '{}' > /tmp/out.json`
		)
	} else {
		lines.push(``, `true`)
	}
	lines.push('')
	return lines.join('\n')
}

function genericBody(ctx: TemplateContext): string {
	const p = commentPrefix(ctx.language)
	const lines: string[] = []
	if (ctx.input) lines.push(`${p} Upstream: ${assetUri(ctx.input)}`)
	if (ctx.output) lines.push(`${p} Output: ${assetUri(ctx.output)}`)
	lines.push(`${p} Fill in pipeline logic.`)
	return lines.join('\n') + '\n'
}

// Entry point: returns the full source (header + body) for the new draft.
// The returned content is ready to drop into a Script as `content` — no
// further mutation needed, including for the trigger annotations.
export function generatePipelineDraft(ctx: TemplateContext): string {
	const head = header(ctx.language, ctx.triggers)
	const body = (() => {
		switch (ctx.language) {
			case 'bun':
			case 'deno':
				return bodyTs(ctx)
			case 'python3':
				return bodyPython(ctx)
			case 'duckdb':
				return bodyDuckdb(ctx)
			case 'postgresql':
				return bodyPostgres(ctx)
			case 'bash':
				return bodyBash(ctx)
			default:
				return genericBody(ctx)
		}
	})()
	return head + body
}
