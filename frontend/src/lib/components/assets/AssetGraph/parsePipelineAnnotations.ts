import type { AssetKind } from '$lib/gen'
import type { NativeTriggerKind } from './types'

// Mirror of backend/parsers/windmill-parser/src/asset_parser.rs
// `parse_pipeline_annotations`, ported to TS so the pipeline editor can
// reflect annotation edits live before deploy. Keep the two implementations
// behaviorally identical — any divergence means the graph preview lies.
//
// `// pipeline` is intentionally strict (keyword must be alone on the line):
// the word "pipeline" is common enough in casual prose that requiring it to
// be the only content is the cheapest way to keep false-positives out.

const COMMENT_PREFIXES = ['//', '--', '#'] as const

const ASSET_PREFIXES: Array<[string, AssetKind]> = [
	['s3://', 's3object'],
	['res://', 'resource'],
	['$res:', 'resource'],
	['ducklake://', 'ducklake'],
	['datatable://', 'datatable'],
	['volume://', 'volume']
]

// Native trigger keywords — `// on <kind>`. Each is marker-only: the
// binding lives on the matching trigger row's own `script_path` field.
// Schedule joins the family — the cron is stored on the schedule row the
// user creates separately, not in the annotation.
const NATIVE_TRIGGER_KEYWORDS: NativeTriggerKind[] = [
	'schedule',
	'webhook',
	'email',
	'kafka',
	'mqtt',
	'nats',
	'postgres',
	'sqs',
	'gcp',
	'data_upload'
]

// Token recognized inside declared asset URIs that the runtime substitutes
// with the current partition value — kept as a single well-known token so
// the parser never has to guess which `{...}` placeholders are partition
// variables. Substitution happens at runtime, not in this parser.
export const PARTITION_TOKEN = '{partition}'

export type PipelineTriggerAsset = { kind: AssetKind; path: string }
// Marker-only — native trigger annotations carry no path. The binding lives
// on the trigger row's own `script_path` field; the graph endpoint resolves
// it by querying the per-kind trigger tables.
export type PipelineNativeTrigger = { kind: NativeTriggerKind }

export type PartitionKind =
	| { kind: 'daily' }
	| { kind: 'hourly' }
	| { kind: 'weekly' }
	| { kind: 'monthly' }
	| { kind: 'dynamic'; key: string }

export type PartitionSpec = PartitionKind & {
	tz?: string
	format?: string
	start?: string
}

export type FreshnessSpec = {
	duration: string
}

// Mirrors backend `parse_duration_secs` (windmill-common assets.rs): a bare
// integer means seconds, otherwise `<n>` with an `s`/`m`/`h`/`d` suffix
// (e.g. `30s`, `5m`, `2h`, `1d`). Returns undefined for malformed or
// non-positive input so a typo'd `// freshness` window fails safe (the chip
// stays neutral instead of guessing a staleness verdict).
export function parseDurationSecs(s: string): number | undefined {
	const t = s.trim()
	if (!t) return undefined
	const last = t[t.length - 1]
	const mult =
		last === 's' ? 1 : last === 'm' ? 60 : last === 'h' ? 3600 : last === 'd' ? 86400 : undefined
	const num = (mult !== undefined ? t.slice(0, -1) : t).trim()
	// `+?`: Rust's i64 parsing accepts an explicit plus sign (`+5m`), so the
	// mirror must too — divergence here would leave the chip neutral for a
	// window the deploy path and watchdog honor.
	if (mult === undefined && !/^\+?\d+$/.test(t)) return undefined
	if (!/^\+?\d+$/.test(num)) return undefined
	const secs = Number(num) * (mult ?? 1)
	if (!Number.isSafeInteger(secs) || secs <= 0 || secs > 2147483647) return undefined
	return secs
}

// `// retry <count> [<delay>]` — see backend RetrySpec. Delay is kept as the
// raw duration string and resolved to seconds at deploy.
export type RetrySpec = {
	count: number
	delay?: string
}

// `// materialize [manual] <asset> [append] [key=<col>]` — see backend
// MaterializeSpec. Managed by default (the runtime generates the write DDL
// around a single SELECT); `manual` opts out (the script writes its own DDL,
// track-only). `append` / `key` / `history` / `track` are managed-mode strategy
// options; `key=<col> history` (or the `scd2` alias) selects SCD type-2 history,
// and `deletes=close` opts scd2 into hard-delete-close.
export type MaterializeSpec = {
	targetKind: AssetKind
	targetPath: string
	// track-only escape hatch; absent === false (managed)
	manual?: boolean
	// INSERT-only strategy; absent === false
	append?: boolean
	// merge key; absent === replace (or append). Also the SCD2 natural key.
	uniqueKey?: string
	// SCD2 managed history mode (valid_from/valid_to/is_current); absent === false
	scd2?: boolean
	// SCD2 tracked columns (change ⇒ new version); empty ⇒ all non-key columns
	track?: string[]
	// SCD2 hard-delete-close (`deletes=close`): close absent keys; absent === false
	closeDeleted?: boolean
}

// `// data_test <kind> …` — a data-quality assertion run against the
// freshly-materialized asset, failing the run on violation. See backend
// `DataTest`. The first extensible annotation family: a keyword head selects
// the variant, the rest is parsed per-variant; a sibling family (e.g.
// column-lineage) follows the same shape. Built-ins mirror dbt's generic data
// tests; `custom` is dbt's singular-test escape hatch (a DuckDB script path).
// Keyword is `data_test`, NOT `test`, to stay clear of the unrelated `// test:`
// CI-test annotation.
// Field names are snake_case to match the Rust `DataTest` serde output verbatim
// — the same type is populated both by this parser (live drafts) and by the
// backend graph endpoint (deployed nodes), so the two must be wire-identical.
export type DataTest =
	| { type: 'unique'; column: string }
	| { type: 'not_null'; column: string }
	| { type: 'accepted_values'; column: string; values: string[] }
	| {
			type: 'relationships'
			column: string
			to_kind: AssetKind
			to_path: string
			to_column: string
	  }
	| { type: 'custom'; path: string }

// `// column <out_col> <- <asset-uri>.<col>[, …]` — declared column-level
// lineage: one output column and the upstream source columns it derives from.
// See backend `ColumnLineage`. A sibling of `DataTest` in the extensible
// annotation family — same parse shape, accumulating (one line per output
// column) — but pure metadata: drives the column-lineage graph view, runs no
// probe. Field names are snake_case to match the Rust `ColumnLineage` serde
// output verbatim, so the live-draft parse and the backend graph endpoint
// (deployed nodes) produce wire-identical shapes.
export type ColumnRef = {
	from_kind: AssetKind
	from_path: string
	from_column: string
}
export type ColumnLineage = {
	column: string
	inputs: ColumnRef[]
}

// Combine body-inferred column lineage with `// column` annotations, the
// annotation winning per output column. Mirrors the Rust `merge_column_lineage`
// (`asset_parser.rs`) so the live-draft preview matches what deploys: the
// backend already merges inferred + annotated server-side, and the live graph
// must apply the same precedence to the WASM-inferred lineage.
export function mergeColumnLineage(
	inferred: ColumnLineage[],
	annotated: ColumnLineage[]
): ColumnLineage[] {
	const seen = new Set(annotated.map((c) => c.column))
	const out = [...annotated]
	for (const c of inferred) {
		if (!seen.has(c.column)) {
			seen.add(c.column)
			out.push(c)
		}
	}
	return out
}

export type PipelineAnnotations = {
	inPipeline: boolean
	triggerAssets: PipelineTriggerAsset[]
	nativeTriggers: PipelineNativeTrigger[]
	partition?: PartitionSpec
	freshness?: FreshnessSpec
	tag?: string
	retry?: RetrySpec
	// `// materialize [manual] <asset> [append] [key=<col>]` — target + strategy.
	materialize?: MaterializeSpec
	// `// data_test <kind> …` — accumulating data-quality checks (multiple lines).
	dataTests: DataTest[]
	// `// column <out> <- <src>.<col>[, …]` — accumulating column lineage.
	columnLineage: ColumnLineage[]
	// Bare `// macros` (alone on the line, like `// pipeline`) — marks this
	// DuckDB script as a workspace macro library.
	macros: boolean
	// `// use <lib_script_path>` — force-inject the named macro library into
	// this script's jobs. Accumulating, declaration order, deduped.
	useLibs: string[]
}

// Tokenize a `key=value [key="quoted value"] ...` option string. Bare
// values run until the next whitespace; quoted values (single or double)
// consume until the matching quote. Malformed pairs are skipped.
function parseKvOpts(s: string): Map<string, string> {
	const out = new Map<string, string>()
	let i = 0
	const n = s.length
	while (i < n) {
		while (i < n && /\s/.test(s[i])) i++
		if (i >= n) break
		// key
		const keyStart = i
		while (i < n && s[i] !== '=' && !/\s/.test(s[i])) i++
		const key = s.slice(keyStart, i)
		if (i >= n || s[i] !== '=' || key === '') {
			while (i < n && !/\s/.test(s[i])) i++
			continue
		}
		i++ // consume '='
		let value: string
		if (i < n && (s[i] === '"' || s[i] === "'")) {
			const q = s[i]
			i++
			const valStart = i
			while (i < n && s[i] !== q) i++
			value = s.slice(valStart, i)
			if (i < n) i++ // consume closing quote
		} else {
			const valStart = i
			while (i < n && !/\s/.test(s[i])) i++
			value = s.slice(valStart, i)
		}
		out.set(key, value)
	}
	return out
}

function parseAssetSyntax(s: string): PipelineTriggerAsset | undefined {
	for (const [prefix, kind] of ASSET_PREFIXES) {
		if (s.startsWith(prefix)) {
			return { kind, path: s.slice(prefix.length) }
		}
	}
	return undefined
}

// Mirror of Rust `parse_asset_syntax(s, enable_default_syntax=true)`: the bare
// words `ducklake` / `datatable` are shorthand for their `…://main` form. Used
// by `// materialize` (but NOT by `// on`, which is default-syntax off, so the
// trigger parser keeps using `parseAssetSyntax`).
function parseAssetSyntaxDefault(s: string): PipelineTriggerAsset | undefined {
	if (s === 'datatable') return { kind: 'datatable', path: 'main' }
	if (s === 'ducklake') return { kind: 'ducklake', path: 'main' }
	return parseAssetSyntax(s)
}

// Parse a `// materialize [manual] <asset> [append] [key=<col>] [history]
// [track=<c1,c2>]` right-hand side. Optional leading `manual` word opts out of
// managed mode; a leading `scd2` word is an alias for the `history` flag; the
// next token is the target asset URI (default-syntax shorthands enabled); the
// remainder are strategy options (`append` flag, `key=<col>`, `history` flag,
// `track=<c1,c2,…>`, `deletes=close`). Missing/empty target → undefined (dropped).
function parseMaterializeSpec(s: string): MaterializeSpec | undefined {
	// One optional leading mode keyword: `manual` (track-only) or `scd2` (an alias
	// for the `history` flag below).
	let manual = false
	let scd2Kw = false
	let rest = s
	const afterManual = consumeKeyword(s, 'manual')
	const afterScd2 = afterManual === undefined ? consumeKeyword(s, 'scd2') : undefined
	if (afterManual !== undefined) {
		manual = true
		rest = afterManual.trimStart()
	} else if (afterScd2 !== undefined) {
		scd2Kw = true
		rest = afterScd2.trimStart()
	}
	rest = rest.trim()
	const m = rest.match(/^(\S+)(?:\s+(.*))?$/)
	if (!m) return undefined
	const asset = parseAssetSyntaxDefault(m[1])
	if (!asset || asset.path === '') return undefined
	const optsStr = m[2] ?? ''
	const optTokens = optsStr.split(/\s+/)
	const append = optTokens.some((t) => t === 'append')
	// SCD type-2 history: primary spelling is the bare `history` flag on a keyed
	// merge; the leading `scd2` keyword is a recognized alias.
	const scd2 = scd2Kw || optTokens.some((t) => t === 'history')
	const opts = parseKvOpts(optsStr)
	const key = opts.get('key')
	const uniqueKey = key && key !== '' ? key : undefined
	// `track=<c1,c2,…>` (scd2): comma-separated tracked columns; empty ⇒ all.
	// The value is whitespace-terminated (like every `=`-option), so it must have
	// no spaces (`track=a,b`, not `track=a, b` — the rest is dropped).
	const track = (opts.get('track') ?? '')
		.split(',')
		.map((c) => c.trim())
		.filter((c) => c !== '')
	// `deletes=close` (scd2 only) opts into hard-delete-close; any other value
	// (or absence) keeps the soft-delete default.
	const closeDeleted = opts.get('deletes') === 'close'
	return {
		targetKind: asset.kind,
		targetPath: asset.path,
		manual,
		append,
		uniqueKey,
		scd2,
		track,
		closeDeleted
	}
}

// A single bare identifier token (column name). Rejects empty / multi-token
// input. Mirrors Rust `single_ident`.
function singleIdent(s: string): string | undefined {
	const t = s.trim()
	if (t === '' || t.split(/\s+/).length !== 1) return undefined
	return t
}

// Strip one layer of matching surrounding single or double quotes.
function unquote(s: string): string {
	if (s.length >= 2 && (s[0] === '"' || s[0] === "'") && s[s.length - 1] === s[0]) {
		return s.slice(1, -1)
	}
	return s
}

// `<col> = a,b,c`. Mirrors Rust `parse_accepted_values`.
function parseAcceptedValues(s: string): DataTest | undefined {
	const eq = s.indexOf('=')
	if (eq < 0) return undefined
	const column = singleIdent(s.slice(0, eq))
	if (!column) return undefined
	const values = s
		.slice(eq + 1)
		.split(',')
		.map((v) => unquote(v.trim()))
		.filter((v) => v !== '')
	if (values.length === 0) return undefined
	return { type: 'accepted_values', column, values }
}

// `<col> -> <asset-uri>.<refcol>`. Mirrors Rust `parse_relationships`.
function parseRelationships(s: string): DataTest | undefined {
	const arrow = s.indexOf('->')
	if (arrow < 0) return undefined
	const column = singleIdent(s.slice(0, arrow))
	if (!column) return undefined
	const target = s.slice(arrow + 2).trim()
	const dot = target.lastIndexOf('.')
	if (dot < 0) return undefined
	const toColumn = singleIdent(target.slice(dot + 1))
	if (!toColumn) return undefined
	const asset = parseAssetSyntaxDefault(target.slice(0, dot).trim())
	if (!asset || asset.path === '') return undefined
	return {
		type: 'relationships',
		column,
		to_kind: asset.kind,
		to_path: asset.path,
		to_column: toColumn
	}
}

// `<asset-uri>.<col>` upstream reference. The column is the segment after the
// final `.`; the rest is the asset URI (default-syntax shorthands enabled).
// Mirrors Rust `parse_column_ref`.
function parseColumnRef(s: string): ColumnRef | undefined {
	const dot = s.lastIndexOf('.')
	if (dot < 0) return undefined
	const fromColumn = singleIdent(s.slice(dot + 1))
	if (!fromColumn) return undefined
	const asset = parseAssetSyntaxDefault(s.slice(0, dot).trim())
	if (!asset || asset.path === '') return undefined
	return { from_kind: asset.kind, from_path: asset.path, from_column: fromColumn }
}

// `<out_col> <- <ref>[, <ref> …]`. Individually malformed refs are dropped; the
// line is kept iff ≥1 ref parses (mirrors `parseAcceptedValues`). A missing
// `<-`, a non-ident output column, or zero valid refs drops the line.
// Mirrors Rust `parse_column_lineage_spec`.
function parseColumnLineageSpec(s: string): ColumnLineage | undefined {
	const arrow = s.indexOf('<-')
	if (arrow < 0) return undefined
	const column = singleIdent(s.slice(0, arrow))
	if (!column) return undefined
	const inputs = s
		.slice(arrow + 2)
		.split(',')
		.map((r) => parseColumnRef(r.trim()))
		.filter((r): r is ColumnRef => r !== undefined)
	if (inputs.length === 0) return undefined
	return { column, inputs }
}

// Parse a `// data_test <kind> …` right-hand side into one `DataTest`. The
// leading token selects the variant; anything not a built-in keyword is the
// `custom` escape hatch (a single script-path token). Returns `undefined` for
// malformed input so a typo fails safe. Mirrors Rust `parse_data_test_spec`.
function parseDataTestSpec(s: string): DataTest | undefined {
	const trimmed = s.trim()
	if (trimmed === '') return undefined
	const m = trimmed.match(/^(\S+)(?:\s+([\s\S]*))?$/)
	if (!m) return undefined
	const head = m[1]
	const rest = (m[2] ?? '').trim()
	switch (head) {
		case 'unique': {
			const column = singleIdent(rest)
			return column ? { type: 'unique', column } : undefined
		}
		case 'not_null': {
			const column = singleIdent(rest)
			return column ? { type: 'not_null', column } : undefined
		}
		case 'accepted_values':
			return parseAcceptedValues(rest)
		case 'relationships':
			return parseRelationships(rest)
		default:
			// Custom escape hatch: the whole right-hand side must be one path
			// token. Trailing content after a non-built-in head is rejected.
			return rest === '' ? { type: 'custom', path: head } : undefined
	}
}

type ParsedTriggerSpec =
	| { kind: 'asset'; value: PipelineTriggerAsset }
	| { kind: 'native'; value: PipelineNativeTrigger }

// Parse a single `on <spec>` right-hand side. The top-level `// schedule`
// is handled separately at the line level (not via `on`).
//
// Native trigger keywords (kafka, mqtt, …) are marker-only — `// on kafka`
// without a trailing path. Trailing content makes the line malformed and
// is rejected. Asset triggers always carry an `<asset-prefix><path>` ref.
function parseTriggerSpec(s: string): ParsedTriggerSpec | undefined {
	for (const kw of NATIVE_TRIGGER_KEYWORDS) {
		if (s.startsWith(kw)) {
			const after = s.slice(kw.length)
			// `kafka` must end the line (modulo whitespace) — `kafkalike`
			// is not `kafka`. Anything trailing makes it malformed.
			if (after.length === 0) {
				return { kind: 'native', value: { kind: kw } }
			}
			if (!/\s/.test(after[0])) continue
			if (after.trim().length > 0) return undefined
			return { kind: 'native', value: { kind: kw } }
		}
	}
	const asset = parseAssetSyntax(s)
	if (asset) return { kind: 'asset', value: asset }
	return undefined
}

// Parse a `// retry <count> [<delay>]` right-hand side. `<count>` is a
// non-negative decimal; `<delay>` is an optional raw duration string left
// for the deploy path to validate. Zero / non-numeric count is rejected so
// a typo can't silently disable cascade retries.
function parseRetrySpec(s: string): RetrySpec | undefined {
	const m = s.match(/^(\S+)(?:\s+(.*))?$/)
	if (!m) return undefined
	// Strict whole-token decimal — match backend `parse::<u32>()` semantics
	// (rejects `3foo`, leading signs, hex, floats).
	if (!/^\d+$/.test(m[1])) return undefined
	const count = Number.parseInt(m[1], 10)
	if (count <= 0) return undefined
	const delay = m[2]?.trim()
	return delay ? { count, delay } : { count }
}

// Parse a `// partitioned <kind> [opts]` right-hand side.
function parsePartitionedSpec(s: string): PartitionSpec | undefined {
	const m = s.match(/^(\S+)(?:\s+(.*))?$/)
	if (!m) return undefined
	const kindWord = m[1]
	const opts = parseKvOpts(m[2] ?? '')
	let kind: PartitionKind
	switch (kindWord) {
		case 'daily':
			kind = { kind: 'daily' }
			break
		case 'hourly':
			kind = { kind: 'hourly' }
			break
		case 'weekly':
			kind = { kind: 'weekly' }
			break
		case 'monthly':
			kind = { kind: 'monthly' }
			break
		case 'dynamic': {
			const key = opts.get('key')
			if (!key) return undefined
			kind = { kind: 'dynamic', key }
			break
		}
		default:
			return undefined
	}
	return {
		...kind,
		tz: opts.get('tz'),
		format: opts.get('format'),
		start: opts.get('start')
	}
}

function stripCommentPrefix(line: string): string | undefined {
	const trimmed = line.trimStart()
	for (const p of COMMENT_PREFIXES) {
		if (trimmed.startsWith(p)) return trimmed.slice(p.length)
	}
	return undefined
}

// Try to consume `<keyword>` as a complete word from `inner`. Returns the
// trailing text after the keyword if it matched, undefined otherwise.
// Avoids `partitioned` matching `partition`, `kafkalike` matching `kafka`,
// etc.
function consumeKeyword(inner: string, kw: string): string | undefined {
	if (!inner.startsWith(kw)) return undefined
	const after = inner.slice(kw.length)
	if (after === '' || /\s/.test(after[0])) return after
	return undefined
}

export function parsePipelineAnnotations(code: string): PipelineAnnotations {
	const out: PipelineAnnotations = {
		inPipeline: false,
		triggerAssets: [],
		nativeTriggers: [],
		dataTests: [],
		columnLineage: [],
		macros: false,
		useLibs: []
	}

	for (const rawLine of code.split('\n')) {
		// Annotations live in the leading comment header: skip blank lines but
		// stop at the first line of actual code, so comments inside the body
		// (e.g. a regular `# tag ...` prose comment) can't false-positive.
		// Mirrors the Rust parse_pipeline_annotations header scan.
		if (rawLine.trim() === '') continue
		const rest = stripCommentPrefix(rawLine)
		if (rest === undefined) break
		const inner = rest.trimStart()

		const afterPipeline = consumeKeyword(inner, 'pipeline')
		if (afterPipeline !== undefined) {
			// Strict — keyword must be alone on the line.
			if (afterPipeline.trim() === '') out.inPipeline = true
			continue
		}

		const afterMacros = consumeKeyword(inner, 'macros')
		if (afterMacros !== undefined) {
			// Strict like `pipeline`: keyword alone on the line, so prose such
			// as `// macros are defined below` never false-positives.
			if (afterMacros.trim() === '') out.macros = true
			continue
		}

		// `// use <lib_script_path>` — accumulating. The argument must be a
		// single whitespace-free token containing `/` (all script paths do),
		// so prose like `// use this script to …` is dropped fail-safe.
		const afterUse = consumeKeyword(inner, 'use')
		if (afterUse !== undefined) {
			const path = afterUse.trim()
			if (path && !/\s/.test(path) && path.includes('/') && !out.useLibs.includes(path)) {
				out.useLibs.push(path)
			}
			continue
		}

		const afterPart = consumeKeyword(inner, 'partitioned')
		if (afterPart !== undefined) {
			if (!out.partition) {
				const spec = parsePartitionedSpec(afterPart.trim())
				if (spec) out.partition = spec
			}
			continue
		}

		const afterFresh = consumeKeyword(inner, 'freshness')
		if (afterFresh !== undefined) {
			const dur = afterFresh.trim()
			if (dur && !out.freshness) {
				out.freshness = { duration: dur }
			}
			continue
		}

		const afterTag = consumeKeyword(inner, 'tag')
		if (afterTag !== undefined) {
			const name = afterTag.trim()
			// Worker tags are single-word identifiers; a value with whitespace
			// or beyond the script.tag column width is almost certainly a
			// regular comment starting with "# tag ...".
			if (name && !out.tag && !/\s/.test(name) && name.length <= 50) {
				out.tag = name
			}
			continue
		}

		const afterRetry = consumeKeyword(inner, 'retry')
		if (afterRetry !== undefined) {
			if (!out.retry) {
				const spec = parseRetrySpec(afterRetry.trim())
				if (spec) out.retry = spec
			}
			continue
		}

		const afterMaterialize = consumeKeyword(inner, 'materialize')
		if (afterMaterialize !== undefined) {
			if (!out.materialize) {
				const spec = parseMaterializeSpec(afterMaterialize.trim())
				if (spec) out.materialize = spec
			}
			continue
		}

		// `data_test` is a complete word (so it never collides with the `// test:`
		// CI annotation, which has no whitespace after `test`). Accumulates.
		const afterDataTest = consumeKeyword(inner, 'data_test')
		if (afterDataTest !== undefined) {
			const spec = parseDataTestSpec(afterDataTest.trim())
			if (spec) out.dataTests.push(spec)
			continue
		}

		// `column` is a complete word; a body comment that merely starts with
		// `column` has no `<-` and is dropped fail-safe. Accumulates.
		const afterColumn = consumeKeyword(inner, 'column')
		if (afterColumn !== undefined) {
			const spec = parseColumnLineageSpec(afterColumn.trim())
			if (spec) out.columnLineage.push(spec)
			continue
		}

		const afterOn = consumeKeyword(inner, 'on')
		if (afterOn !== undefined) {
			const specText = afterOn.trim()
			if (!specText) continue
			const spec = parseTriggerSpec(specText)
			if (!spec) continue
			if (spec.kind === 'asset') {
				if (
					!out.triggerAssets.some((a) => a.kind === spec.value.kind && a.path === spec.value.path)
				) {
					out.triggerAssets.push(spec.value)
				}
			} else {
				if (!out.nativeTriggers.some((n) => n.kind === spec.value.kind)) {
					out.nativeTriggers.push(spec.value)
				}
			}
		}
	}

	return out
}
