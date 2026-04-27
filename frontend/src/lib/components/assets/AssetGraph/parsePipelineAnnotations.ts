import type { AssetKind } from '$lib/gen'
import type { NativeTriggerKind } from './types'

// Mirror of backend/parsers/windmill-parser/src/asset_parser.rs
// `parse_pipeline_annotations` + `parse_trigger_spec`, ported to TS so the
// pipeline editor can reflect `// materialize` / `// on <spec>` edits live
// before deploy. Keep the two implementations behaviorally identical — any
// divergence means the graph preview lies.

const COMMENT_PREFIXES = ['//', '--', '#'] as const

const ASSET_PREFIXES: Array<[string, AssetKind]> = [
	['s3://', 's3object'],
	['res://', 'resource'],
	['$res:', 'resource'],
	['ducklake://', 'ducklake'],
	['datatable://', 'datatable'],
	['volume://', 'volume']
]

// Non-native, non-schedule trigger keywords — each carries a workspace
// trigger-path reference (`on <kind> <path>`).
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

export type PipelineTriggerAsset = { kind: AssetKind; path: string }
export type PipelineNativeTrigger = { kind: NativeTriggerKind; path: string }

export type PipelineAnnotations = {
	isMaterializer: boolean
	triggerAssets: PipelineTriggerAsset[]
	schedules: string[] // raw cron expressions, in insertion order
	nativeTriggers: PipelineNativeTrigger[]
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

function parseAssetSyntax(s: string): PipelineTriggerAsset | undefined {
	for (const [prefix, kind] of ASSET_PREFIXES) {
		if (s.startsWith(prefix)) {
			return { kind, path: s.slice(prefix.length) }
		}
	}
	return undefined
}

type ParsedSpec =
	| { kind: 'asset'; value: PipelineTriggerAsset }
	| { kind: 'schedule'; value: string }
	| { kind: 'native'; value: PipelineNativeTrigger }

function parseTriggerSpec(s: string): ParsedSpec | undefined {
	if (s.startsWith('schedule')) {
		const rest = s.slice('schedule'.length).trimStart()
		const cron = unquote(rest)
		if (cron && cron.trim() !== '') return { kind: 'schedule', value: cron }
		return undefined
	}
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

function stripCommentPrefix(line: string): string | undefined {
	const trimmed = line.trimStart()
	for (const p of COMMENT_PREFIXES) {
		if (trimmed.startsWith(p)) return trimmed.slice(p.length)
	}
	return undefined
}

export function parsePipelineAnnotations(code: string): PipelineAnnotations {
	let isMaterializer = false
	const triggerAssets: PipelineTriggerAsset[] = []
	const schedules: string[] = []
	const nativeTriggers: PipelineNativeTrigger[] = []

	for (const rawLine of code.split('\n')) {
		const rest = stripCommentPrefix(rawLine)
		if (rest === undefined) continue
		const inner = rest.trimStart()

		if (inner.startsWith('materialize')) {
			const after = inner.slice('materialize'.length)
			if (after === '' || /\s/.test(after[0])) isMaterializer = true
			continue
		}

		if (inner.startsWith('on')) {
			const after = inner.slice('on'.length)
			if (after === '' || !/\s/.test(after[0])) continue
			const specText = after.trim()
			if (!specText) continue
			const spec = parseTriggerSpec(specText)
			if (!spec) continue
			if (spec.kind === 'asset') {
				if (!triggerAssets.some((a) => a.kind === spec.value.kind && a.path === spec.value.path)) {
					triggerAssets.push(spec.value)
				}
			} else if (spec.kind === 'schedule') {
				if (!schedules.includes(spec.value)) schedules.push(spec.value)
			} else {
				if (!nativeTriggers.some((n) => n.kind === spec.value.kind && n.path === spec.value.path)) {
					nativeTriggers.push(spec.value)
				}
			}
		}
	}

	return { isMaterializer, triggerAssets, schedules, nativeTriggers }
}
