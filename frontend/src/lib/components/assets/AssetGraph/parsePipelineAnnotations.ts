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

// Non-asset, non-schedule trigger keywords — `on <kind> <path>`.
const NATIVE_TRIGGER_KEYWORDS: NativeTriggerKind[] = [
	'webhook',
	'email',
	'kafka',
	'mqtt',
	'nats',
	'postgres',
	'sqs',
	'gcp'
]

// Token recognized inside declared asset URIs that the runtime substitutes
// with the current partition value — kept as a single well-known token so
// the parser never has to guess which `{...}` placeholders are partition
// variables. Substitution happens at runtime, not in this parser.
export const PARTITION_TOKEN = '{partition}'

export type PipelineTriggerAsset = { kind: AssetKind; path: string }
export type PipelineNativeTrigger = { kind: NativeTriggerKind; path: string }

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

export type PipelineAnnotations = {
	inPipeline: boolean
	triggerAssets: PipelineTriggerAsset[]
	schedules: string[] // raw cron expressions, in insertion order
	nativeTriggers: PipelineNativeTrigger[]
	partition?: PartitionSpec
	freshness?: FreshnessSpec
}

function unquote(s: string): string | undefined {
	if (s.length >= 2) {
		const q = s[0]
		if ((q === '"' || q === "'") && s.endsWith(q)) {
			return s.slice(1, -1)
		}
	}
	return undefined
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

type ParsedTriggerSpec =
	| { kind: 'asset'; value: PipelineTriggerAsset }
	| { kind: 'native'; value: PipelineNativeTrigger }

// Parse a single `on <spec>` right-hand side. The top-level `// schedule`
// is handled separately at the line level (not via `on`).
function parseTriggerSpec(s: string): ParsedTriggerSpec | undefined {
	for (const kw of NATIVE_TRIGGER_KEYWORDS) {
		if (s.startsWith(kw)) {
			const after = s.slice(kw.length)
			// Require whitespace so `kafkalike` doesn't match `kafka`.
			if (after.length === 0 || !/\s/.test(after[0])) continue
			const path = after.trim()
			if (!path) return undefined
			return { kind: 'native', value: { kind: kw, path } }
		}
	}
	const asset = parseAssetSyntax(s)
	if (asset) return { kind: 'asset', value: asset }
	return undefined
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
		schedules: [],
		nativeTriggers: []
	}

	for (const rawLine of code.split('\n')) {
		const rest = stripCommentPrefix(rawLine)
		if (rest === undefined) continue
		const inner = rest.trimStart()

		const afterPipeline = consumeKeyword(inner, 'pipeline')
		if (afterPipeline !== undefined) {
			// Strict — keyword must be alone on the line.
			if (afterPipeline.trim() === '') out.inPipeline = true
			continue
		}

		const afterSchedule = consumeKeyword(inner, 'schedule')
		if (afterSchedule !== undefined) {
			const cron = unquote(afterSchedule.trim())
			if (cron && cron.trim() !== '' && !out.schedules.includes(cron)) {
				out.schedules.push(cron)
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
				if (
					!out.nativeTriggers.some((n) => n.kind === spec.value.kind && n.path === spec.value.path)
				) {
					out.nativeTriggers.push(spec.value)
				}
			}
		}
	}

	return out
}
