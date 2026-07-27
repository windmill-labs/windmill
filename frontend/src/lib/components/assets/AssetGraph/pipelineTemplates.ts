import type { ScriptLang, AssetKind } from '$lib/gen'
import { random_adj } from '$lib/components/random_positive_adjetive'
import { parseDbInputFromAssetSyntax } from '$lib/utils'

// What kind of asset the new script will produce. Drives the random output
// path scheme and the body skeleton. For most kinds the output asset is NOT
// declared in a comment annotation — it's reconstructed from the body's SDK
// calls / SQL by the asset parser, same as production scripts, so it can't go
// stale. The exception is `materialize`, which declares its target explicitly
// via the `// materialize` annotation (the runtime generates the write).
//
// `none` is the conservative default — body just has a "fill in" comment. The
// other kinds inject their respective wmill SDK calls / SQL setup so the
// script is runnable (modulo schema definition) the moment it's created.
export type PipelineOutputKind =
	| 'none'
	| 'datatable'
	| 'ducklake'
	| 'materialize'
	| 'data_test'
	| 's3_parquet'
	| 's3_object'
	| 'macros'

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
		id: 'materialize',
		label: 'Materialized table',
		description: 'Managed DuckLake table — idempotent, versioned, tracked'
	},
	{
		id: 'data_test',
		label: 'Data test',
		description: 'Custom assertion — a single SELECT returning offending rows (empty = pass)'
	},
	{
		id: 'datatable',
		label: 'Data table',
		description: 'Postgres-backed typed table'
	},
	{
		id: 'ducklake',
		label: 'Ducklake',
		description: 'DuckDB lakehouse table (raw write)'
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
		id: 'macros',
		label: 'Macro library',
		description: 'Reusable DuckDB macros, callable from every script in the workspace'
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
	// `materialize` is DuckDB-only: it generates the managed write around a
	// single SELECT. The Python/TS `wmll.ducklake` helper currently takes a SQL
	// SELECT (not in-memory rows), so a polyglot managed materialize is a
	// separate follow-up — those langs keep the `ducklake` raw-write kind.
	duckdb: [
		'materialize',
		'data_test',
		'datatable',
		'ducklake',
		's3_parquet',
		's3_object',
		'macros',
		'none'
	],
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
	nativets: ['none'],
	ruby: ['none'],
	rlang: ['none']
}

export function compatibleOutputKinds(lang: ScriptLang): PipelineOutputKind[] {
	return LANG_COMPATIBILITY[lang] ?? ['none']
}

// Short uniqueness backstop appended to the human-readable name, so that two
// drafts created in the same session don't collide on `<adj>_<noun>`. 4 chars
// of base36 → ~1.7M variants, more than enough alongside the adj×noun prefix.
function shortSlug(len = 4): string {
	const a = 'abcdefghijklmnopqrstuvwxyz0123456789'
	let out = ''
	for (let i = 0; i < len; i++) out += a[Math.floor(Math.random() * a.length)]
	return out
}

// Asset-shaped noun pools. Mirrors the `random_adj() + '_' + namePlaceholder`
// convention used by Path.svelte for scripts/flows/apps, but the noun is
// chosen to read like the thing the asset *is*: a table for datatable /
// ducklake, a dataset / file payload for S3 outputs.
const TABLE_NOUNS = [
	'table',
	'dataset',
	'records',
	'events',
	'rows',
	'entries',
	'metrics',
	'snapshot'
]
const DATASET_NOUNS = ['dataset', 'snapshot', 'extract', 'export', 'records', 'batch']
const FILE_NOUNS = ['payload', 'output', 'report', 'extract', 'snapshot', 'dump']

function pick(arr: string[]): string {
	return arr[Math.floor(Math.random() * arr.length)]
}

// Fully-qualified asset URI for a freshly-created output, given the picked
// kind. Names follow `<adjective>_<noun>_<slug>` to read like real assets
// (e.g. `nimble_dataset_a3kp`) instead of opaque hashes — matches the
// `random_adj()_*` naming we already use for scripts/flows/apps. Layout:
//   - datatable / ducklake → `main/<adj>_<noun>_<slug>` (table refs)
//   - s3 parquet / object  → `pipelines/<folder>/<adj>_<noun>_<slug>.<ext>`
export function autoOutputAsset(
	kind: PipelineOutputKind,
	folder: string,
	language?: ScriptLang
): { kind: AssetKind; path: string } | undefined {
	const slug = shortSlug()
	const adj = random_adj()
	switch (kind) {
		case 'datatable':
			return { kind: 'datatable', path: `main/${adj}_${pick(TABLE_NOUNS)}_${slug}` }
		case 'ducklake':
		case 'materialize':
			return { kind: 'ducklake', path: `main/${adj}_${pick(TABLE_NOUNS)}_${slug}` }
		// s3 paths carry the canonical leading slash of a default-storage
		// object (`s3:///<key>` parses to path `/<key>`). The deploy-time
		// parser stores writes in that form — a slashless seeded path would
		// never match it, and the post-deploy drift check would report the
		// output as lost (it isn't; the key differs by one '/'). Bodies emit
		// the path verbatim after `s3://`, so the slash round-trips.
		case 's3_parquet':
			return {
				kind: 's3object',
				path: `/pipelines/${folder}/${adj}_${pick(DATASET_NOUNS)}_${slug}.parquet`
			}
		case 's3_object': {
			// duckdb's natural output for a generic blob is CSV (one COPY TO
			// statement). TS/Python templates serialize JSON, so default to
			// .json for those — keeps the body and the file extension in
			// sync without the user having to rename anything.
			const ext = language === 'duckdb' ? 'csv' : 'json'
			return {
				kind: 's3object',
				path: `/pipelines/${folder}/${adj}_${pick(FILE_NOUNS)}_${slug}.${ext}`
			}
		}
		// A macro library produces no asset — its "output" is the registry
		// entries the deploy records. A custom data test produces no asset
		// either — it asserts against an existing materialized target.
		case 'macros':
		case 'data_test':
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

// Splits a datatable asset path (`<db>/<table>` or `<db>/<schema>.<table>`)
// into its constituent parts. The `<schema>.<table>` grammar is owned by
// `parseDbInputFromAssetSyntax` in $lib/utils.ts (which consumes a full
// `datatable://…` URI): we delegate to it for the schema/table split so the
// two stay in lockstep, and only keep the leading `<db>/` extraction here —
// the util folds the db into `resourcePath`, which is awkward to read back,
// and it has no slashless fallback (the SQL emitters tolerate a bare db).
function parseDatatablePath(p: string): {
	db: string
	schema: string | undefined
	table: string
} {
	const slash = p.indexOf('/')
	if (slash < 0) return { db: p, schema: undefined, table: '' }
	const db = p.slice(0, slash)
	// `datatable://` URIs always parse to the `database` variant, so the
	// `specificSchema`/`specificTable` accessors below are always present.
	const parsed = parseDbInputFromAssetSyntax(`datatable://${p}`)
	const schema = parsed?.type === 'database' ? parsed.specificSchema : undefined
	return { db, schema, table: parsed?.specificTable ?? '' }
}

// Schema-qualified table reference for SQL emission. We always emit the
// schema (defaulting to `public`) so the generated statement doesn't rely
// on whatever search-path / default-schema the engine picks at attach
// time — important for DuckDB attaching to Postgres as `pg`, where bare
// `pg.<table>` is fragile.
function qualifiedDatatableRef(p: string, defaultSchema = 'public'): string {
	const parsed = parseDatatablePath(p)
	return `${parsed.schema ?? defaultSchema}.${parsed.table || 'table_name'}`
}

// Catalog-relative table reference. Emits `<schema>.<table>` only when the
// asset path explicitly carries a schema (`main/myschema.mytable`); otherwise
// returns the bare `<table>`. Used inside DuckDB attached-catalog references
// (`pg.<ref>`, `lake.<ref>`) where the asset parser maps a 2-part name like
// `pg.foo` straight to `datatable://main/foo` — adding a redundant `public.`
// would shift the parsed asset to `datatable://main/public.foo` and silently
// desync from the output asset the user picked.
function catalogTableRef(p: string): string {
	const parsed = parseDatatablePath(p)
	const table = parsed.table || 'table_name'
	return parsed.schema ? `${parsed.schema}.${table}` : table
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
	| { kind: 'asset'; ref: string }
	| {
			// All native triggers are marker-only — the binding lives on the
			// trigger row's own `script_path` field. `path` is unused for
			// drafts (kept as a no-op slot for shape parity with attached
			// edges); the user creates the trigger row in its dedicated UI.
			kind:
				| 'schedule'
				| 'webhook'
				| 'email'
				| 'kafka'
				| 'mqtt'
				| 'amqp'
				| 'nats'
				| 'postgres'
				| 'sqs'
				| 'gcp'
				| 'data_upload'
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
function header(ctx: TemplateContext): string {
	const { language, triggers, output, outputKind, input } = ctx
	const p = commentPrefix(language)
	// A custom data test is a standalone script, not a graph node that produces
	// an asset — so it gets no `// pipeline` / output annotation. Instead, tell
	// the author how to wire it up (the `data_test` reference) and the two rules
	// that aren't obvious: single SELECT, returning the offending rows.
	if (outputKind === 'data_test') {
		return [
			`${p} Custom data test — reference it from a materialize script with \`${p} data_test <this-script-path>\`.`,
			`${p} It must be a single SELECT returning the offending rows; the run fails if any row comes back.`,
			''
		].join('\n')
	}
	// A ducklake/s3 read the body performs auto-wires its cascade edge from the
	// FROM clause, so an explicit `// on` for that same asset would be redundant
	// now that auto-derivation is the default. Only drop it when the generated
	// body actually READS the input: bun/deno/python/duckdb emit an input load,
	// but postgres/bash/generic bodies (and `data_upload`, which reads the picker
	// `file` instead) do not — dropping there would leave the script with no
	// cascade at all. Also keep `// on` for kinds inference can't derive
	// (datatable/resource/…) and native triggers.
	const bodyReadsInput = READS_INPUT_LANGS.has(language) && !isDataUpload(triggers)
	const inputRef = input ? assetUri(input) : undefined
	const inputAutoDerives = input?.kind === 'ducklake' || input?.kind === 's3object'
	const lines = triggers.flatMap((t) => {
		switch (t.kind) {
			case 'asset':
				if (bodyReadsInput && inputAutoDerives && t.ref === inputRef) return []
				return [`${p} on ${t.ref}`]
			default:
				// Native triggers (incl. schedule): marker-only — the
				// binding lives on the trigger row's own `script_path`.
				return [`${p} on ${t.kind}`]
		}
	})
	// Managed materialization is the one kind that declares its output
	// explicitly — the runtime generates the write around the body's SELECT, so
	// the target can't be inferred from the body. Emit `// materialize <uri>`
	// plus a hint about the strategy options that go on the same line. The hint
	// must NOT start with a parser keyword (`materialize`, `on`, …) or it would
	// be read as an annotation — `Strategy:` is safe.
	const matLine =
		outputKind === 'materialize' && output
			? [
					`${p} materialize ${assetUri(output)}`,
					`${p} Strategy: add key=<col> to merge (upsert), or append for insert-only; default replaces the partition`
				]
			: []
	// Macro library: the `// macros` marker registers every CREATE MACRO below
	// into the workspace registry at deploy. The hint must not start with a
	// parser keyword — `Consumers` is safe.
	const macrosLine =
		outputKind === 'macros'
			? [
					`${p} macros`,
					`${p} Consumers just call these by name; add \`${p} use <this-script-path>\` in a consumer to force-inject the whole library`
				]
			: []
	// Discoverability hint — a single pointer to the docs rather than a dump of
	// every optional annotation. New users found the old feature list
	// (mute/partitioned/freshness/retry/tag on one line) overwhelming; the docs
	// are the canonical reference once they want any of it. A blank line
	// separates it from the parsed annotations above (`// pipeline`, `// on …`)
	// so the editor reads as "real annotations, then a hint".
	const more = `${p} Optional: partitioning, freshness, retries & cascade control — https://www.windmill.dev/docs/core_concepts/pipelines`
	return [`${p} pipeline`, ...lines, ...matLine, ...macrosLine, '', more, ''].join('\n')
}

// Bun / Deno bodies. These share the wmill SDK surface, so we treat them
// uniformly. The differences vs the previous generic body:
//   - input asset → real `wmill.loadS3File` / `wmill.datatable(...).fetch()`
//   - output asset → real `wmill.writeS3File` / `wmill.datatable(...).fetch()`
//   - no module-scope OUT constant — the parser reads write paths from the
//     SDK call sites directly, so the constant was redundant.
// Whether this draft is a UI-first "data upload" entry point. When true the
// generated `main` takes an `S3Object` parameter (rendered as the
// auto-generated S3 picker on the run form) instead of reading from a fixed
// upstream asset — the user uploads the file that runs the pipeline.
function isDataUpload(triggers: DraftTriggerSource[]): boolean {
	return triggers.some((t) => t.kind === 'data_upload')
}

// Languages whose generated body reads the `input` asset (an SDK load / SQL
// FROM), so a ducklake/s3 input auto-derives its cascade and the explicit
// `// on` can be dropped. postgres/bash/generic bodies ignore `input`.
const READS_INPUT_LANGS: ReadonlySet<ScriptLang> = new Set(['bun', 'deno', 'python3', 'duckdb'])

function bodyTs(ctx: TemplateContext): string {
	const { input, output, outputKind } = ctx
	const dataUpload = isDataUpload(ctx.triggers)
	const importLine = `import * as wmill from "windmill-client"\n`

	const inputBlock = (() => {
		if (dataUpload) {
			// `file` comes from the auto-generated S3 picker on the run form.
			return [
				`  // Uploaded via the S3 picker on the run form.`,
				`  const buf = await wmill.loadS3File(file)`,
				`  const rows = JSON.parse(new TextDecoder().decode(buf))`,
				``
			].join('\n')
		}
		if (!input) return ''
		switch (input.kind) {
			case 's3object':
				// `input.path` encodes storage as `<storage>/<key>` (an empty
				// storage segment — leading slash — is the workspace default).
				// Emit it verbatim after `s3://` so a named-storage input keeps
				// its storage; stripping the slash reads the default-storage key.
				return [
					`  const buf = await wmill.loadS3File(${JSON.stringify(`s3://${input.path}`)})`,
					`  const rows = JSON.parse(new TextDecoder().decode(buf))`,
					``
				].join('\n')
			case 'datatable':
				return [
					`  const src = wmill.datatable(${JSON.stringify(input.path.split('/')[0] ?? 'main')})`,
					`  const rows = await src\`SELECT * FROM ${input.path.split('/').slice(1).join('_') || 'table_name'}\`.fetch()`,
					``
				].join('\n')
			case 'ducklake':
				return [
					`  const lake = wmill.ducklake(${JSON.stringify(input.path.split('/')[0] ?? 'main')})`,
					`  const rows = await lake\`SELECT * FROM ${input.path.split('/').slice(1).join('_') || 'table_name'}\`.fetch()`,
					``
				].join('\n')
			default:
				return ''
		}
	})()

	const outputBlock = (() => {
		if (!output) return `  // (no output asset — fill in side-effect logic)`
		switch (outputKind) {
			case 's3_parquet':
			case 's3_object':
				// `s3://<storage>/<key>` URI — see the loadS3File note above.
				return [
					`  const payload = new TextEncoder().encode(JSON.stringify(rows))`,
					`  await wmill.writeS3File(${JSON.stringify(`s3://${output.path}`)}, payload)`
				].join('\n')
			case 'datatable': {
				const dbName = output.path.split('/')[0] ?? 'main'
				const tableName = output.path.split('/').slice(1).join('_') || 'table_name'
				return [
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

	const rowsFallback = !input && !dataUpload ? `  const rows: any[] = []\n` : ''
	const signature = dataUpload
		? 'export async function main(file: wmill.S3Object) {'
		: 'export async function main() {'

	return [importLine, signature, inputBlock, rowsFallback, outputBlock, '}', ''].join('\n')
}

function bodyPython(ctx: TemplateContext): string {
	const { input, output, outputKind } = ctx
	const dataUpload = isDataUpload(ctx.triggers)
	const importLine = `import wmill\n`

	const inputBlock = (() => {
		if (dataUpload) {
			// `file` comes from the auto-generated S3 picker on the run form.
			return [
				`    # Uploaded via the S3 picker on the run form.`,
				`    buf = wmill.load_s3_file(file)`,
				`    import json; rows = json.loads(buf.decode("utf-8"))`
			].join('\n')
		}
		if (!input) return ''
		switch (input.kind) {
			case 's3object':
				// SDK string params must be s3:// URIs (bare keys are rejected).
				// `input.path` encodes storage as `<storage>/<key>` (empty storage
				// segment — leading slash — is the workspace default), so emit it
				// verbatim after `s3://` to preserve a named-storage input.
				return [
					`    buf = wmill.load_s3_file(${JSON.stringify(`s3://${input.path}`)})`,
					`    import json; rows = json.loads(buf.decode("utf-8"))`
				].join('\n')
			case 'datatable':
				return [
					`    src = wmill.datatable(${JSON.stringify(input.path.split('/')[0] ?? 'main')})`,
					`    rows = src.query("SELECT * FROM ${input.path.split('/').slice(1).join('_') || 'table_name'}").fetch()`
				].join('\n')
			case 'ducklake':
				return [
					`    lake = wmill.ducklake(${JSON.stringify(input.path.split('/')[0] ?? 'main')})`,
					`    rows = lake.query("SELECT * FROM ${input.path.split('/').slice(1).join('_') || 'table_name'}").fetch()`
				].join('\n')
			default:
				return ''
		}
	})()

	const outputBlock = (() => {
		if (!output) return `    # (no output asset — fill in side-effect logic)`
		switch (outputKind) {
			case 's3_parquet':
			case 's3_object':
				// `s3://<storage>/<key>` URI — see the load_s3_file note above.
				return [
					`    import json`,
					`    wmill.write_s3_file(${JSON.stringify(`s3://${output.path}`)}, json.dumps(rows).encode("utf-8"))`
				].join('\n')
			case 'datatable': {
				const dbName = output.path.split('/')[0] ?? 'main'
				const tableName = output.path.split('/').slice(1).join('_') || 'table_name'
				return [
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

	const rowsFallback = !input && !dataUpload ? `    rows: list = []` : ''
	const signature = dataUpload ? 'def main(file: wmill.S3Object):' : 'def main():'

	return [importLine, signature, inputBlock, rowsFallback, outputBlock, ''].join('\n')
}

function bodyDuckdb(ctx: TemplateContext): string {
	const { input, output, outputKind } = ctx
	const dataUpload = isDataUpload(ctx.triggers)
	if (outputKind === 'data_test') {
		// Standalone custom data test: it reads ONLY the freshly-materialized
		// target, which the runtime attaches under the internal `_wm_target`
		// schema — so it emits no ATTACH / input load of its own. When the test
		// was created off a ducklake asset, seed its table name; otherwise a
		// clear placeholder. This exact shape (single SELECT vs `_wm_target`) is
		// what the backend's self-teaching errors ask for.
		const testTable = input?.kind === 'ducklake' ? catalogTableRef(input.path) : 'your_table'
		return [
			'',
			`-- Return the rows that VIOLATE your assertion; an empty result means the test passes.`,
			`-- \`_wm_target\` is the freshly-materialized target, attached by the runtime.`,
			`SELECT * FROM _wm_target.${testTable} WHERE your_condition;`,
			''
		].join('\n')
	}
	const lines: string[] = []
	if (dataUpload) {
		// `(s3object)` param declaration → the run form renders the S3 picker
		// for `$file`; DuckDB reads the uploaded file directly via read_*().
		lines.push(`-- $file (s3object)`)
		lines.push(`-- \`file\` is uploaded via the S3 picker on the run form.`)
	}
	lines.push('')

	// Resolve the catalog db name to ATTACH. Output's db wins when both sides
	// share a kind (the typical pipeline shape: read from one table, write to
	// another in the same database). We emit at most one ATTACH per catalog
	// kind so the generated script stays readable.
	const datatableDb = (() => {
		if (output?.kind === 'datatable') return parseDatatablePath(output.path).db
		if (input?.kind === 'datatable') return parseDatatablePath(input.path).db
		return undefined
	})()
	const ducklakeDb = (() => {
		if (outputKind === 'ducklake' && output?.kind === 'ducklake')
			return parseDatatablePath(output.path).db
		if (input?.kind === 'ducklake') return parseDatatablePath(input.path).db
		return undefined
	})()
	if (datatableDb) {
		lines.push(`ATTACH 'datatable://${datatableDb}' AS pg;`)
	}
	if (ducklakeDb) {
		lines.push(`ATTACH 'ducklake://${ducklakeDb}' AS lake;`)
	}
	if (datatableDb || ducklakeDb) lines.push('')

	const inSql = (() => {
		// Uploaded file param wins — read the S3 object the user picks at run
		// time. read_json_auto matches the JSON default of the TS/Python
		// templates; swap for read_csv_auto / read_parquet as needed.
		if (dataUpload) return `read_json_auto($file)`
		if (!input) return null
		switch (input.kind) {
			case 's3object':
				return `read_parquet('s3://${input.path}')`
			case 'datatable':
				// `pg` is the attached Postgres catalog (see ATTACH above).
				// Use a 2-part `pg.<table>` ref so the asset parser maps it
				// back to `datatable://<db>/<table>` — matching the input asset.
				// Schema is only emitted if the asset path explicitly includes
				// one (`main/myschema.mytable`).
				return `pg.${catalogTableRef(input.path)}`
			case 'ducklake':
				return `lake.${catalogTableRef(input.path)}`
			default:
				return null
		}
	})()

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
				const ref = `lake.${catalogTableRef(output.path)}`
				lines.push(
					`CREATE TABLE IF NOT EXISTS ${ref} AS`,
					`SELECT * FROM ${inSql ?? '(SELECT 1 AS placeholder)'};`
				)
			}
			break
		case 'materialize':
			// Managed materialization: the body is just the SELECT that produces
			// the slice — the runtime wraps it into the idempotent write +
			// snapshot (see the `// materialize` annotation in the header). No
			// CREATE TABLE / INSERT, and the target is NOT attached here (the
			// runtime attaches it).
			//
			// Partitioned? `{partition}` is the slice's identity string. For a
			// time grain the runtime injects a `wm_partition(ts)` macro that
			// buckets a timestamp with that same identity format, so one
			// grain-agnostic filter line targets the active slice — no
			// hand-written strftime format, no `= TIMESTAMP {partition}` cast
			// (which errors for hourly/weekly/monthly). One commented line keeps
			// the scaffold light; docs cover dynamic-grain keys.
			lines.push(
				`-- Partitioned? Filter the source to the active slice: WHERE wm_partition(<ts_col>) = {partition}`,
				`SELECT * FROM ${inSql ?? '(SELECT 1 AS placeholder)'};`
			)
			break
		case 'datatable':
			if (output) {
				// 2-part `pg.<table>` so the asset parser resolves the
				// CREATE TABLE write to `datatable://<db>/<table>` —
				// matching the output asset the user picked. Postgres'
				// search_path defaults to `public`, so the DDL still
				// lands in the right schema. We only schema-qualify
				// when the asset path itself carries an explicit schema.
				const ref = `pg.${catalogTableRef(output.path)}`
				lines.push(
					`CREATE TABLE IF NOT EXISTS ${ref} AS`,
					`SELECT * FROM ${inSql ?? '(SELECT 1 AS placeholder)'};`
				)
			}
			break
		case 'macros':
			// Library body: only CREATE [OR REPLACE] MACRO statements (plus plain
			// setup). One scalar + one table example; bodies may only call macros
			// defined EARLIER in the file (DuckDB bind-checks at creation).
			lines.push(
				`CREATE OR REPLACE MACRO safe_div(a, b, fallback := 0) AS`,
				`  CASE WHEN b = 0 THEN fallback ELSE a / b END;`,
				``,
				`CREATE OR REPLACE MACRO sample_rows(src, n) AS TABLE`,
				`  SELECT * FROM query_table(src) LIMIT n;`
			)
			break
		case 'none':
		default:
			// With an uploaded file but no output asset, at least surface its
			// rows so the script runs end-to-end against the picked file.
			lines.push(dataUpload && inSql ? `SELECT * FROM ${inSql};` : `SELECT 1;`)
	}
	lines.push('')
	return lines.join('\n')
}

function bodyPostgres(ctx: TemplateContext): string {
	const { output, outputKind } = ctx
	const lines: string[] = []
	lines.push('')

	if (outputKind === 'datatable' && output) {
		// Schema-qualified — Postgres respects search_path so a bare
		// `<table>` would land in `public` by default, but emitting
		// `<schema>.<table>` makes the target explicit and survives any
		// non-default search_path the connection might carry.
		const ref = qualifiedDatatableRef(output.path)
		lines.push(
			`-- The pipeline-script harness runs this query against the`,
			`-- datatable backing the configured workspace storage.`,
			`CREATE TABLE IF NOT EXISTS ${ref} (`,
			`  id serial primary key,`,
			`  payload jsonb,`,
			`  computed_at timestamptz default now()`,
			`);`,
			``,
			`-- Replace with your real INSERT.`,
			`INSERT INTO ${ref} (payload) VALUES ('{}'::jsonb);`
		)
	} else {
		lines.push(`SELECT 1;`)
	}
	lines.push('')
	return lines.join('\n')
}

function bodyBash(ctx: TemplateContext): string {
	const { output, outputKind } = ctx
	const lines: string[] = []
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
	lines.push(`${p} Fill in pipeline logic.`)
	return lines.join('\n') + '\n'
}

// Entry point: returns the full source (header + body) for the new draft.
// The returned content is ready to drop into a Script as `content` — no
// further mutation needed, including for the trigger annotations.
export function generatePipelineDraft(ctx: TemplateContext): string {
	const head = header(ctx)
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
