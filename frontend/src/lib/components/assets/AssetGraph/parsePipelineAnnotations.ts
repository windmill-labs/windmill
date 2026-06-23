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

// `// retry <count> [<delay>]` — see backend RetrySpec. Delay is kept as the
// raw duration string and resolved to seconds at deploy.
export type RetrySpec = {
	count: number
	delay?: string
}

// `// materialize [manual] <asset> [append] [key=<col>]` — see backend
// MaterializeSpec. Managed by default (the runtime generates the write DDL
// around a single SELECT); `manual` opts out (the script writes its own DDL,
// track-only). `append` / `key` are managed-mode strategy options.
export type MaterializeSpec = {
	targetKind: AssetKind
	targetPath: string
	// track-only escape hatch; absent === false (managed)
	manual?: boolean
	// INSERT-only strategy; absent === false
	append?: boolean
	// merge key; absent === replace (or append)
	uniqueKey?: string
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

// Parse a `// materialize [manual] <asset> [append] [key=<col>]` right-hand
// side. Optional leading `manual` word opts out of managed mode; the next token
// is the target asset URI (default-syntax shorthands enabled); the remainder
// are strategy options (`append` flag, `key=<col>`). Missing/empty target →
// undefined (dropped).
function parseMaterializeSpec(s: string): MaterializeSpec | undefined {
	let manual = false
	let rest = s
	const afterManual = consumeKeyword(s, 'manual')
	if (afterManual !== undefined) {
		manual = true
		rest = afterManual.trimStart()
	}
	rest = rest.trim()
	const m = rest.match(/^(\S+)(?:\s+(.*))?$/)
	if (!m) return undefined
	const asset = parseAssetSyntaxDefault(m[1])
	if (!asset || asset.path === '') return undefined
	const optsStr = m[2] ?? ''
	const append = optsStr.split(/\s+/).some((t) => t === 'append')
	const key = parseKvOpts(optsStr).get('key')
	const uniqueKey = key && key !== '' ? key : undefined
	return { targetKind: asset.kind, targetPath: asset.path, manual, append, uniqueKey }
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
		nativeTriggers: []
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
