import type { AssetKind } from '$lib/gen'
import { parsePipelineAnnotations, type PipelineAnnotations } from './parsePipelineAnnotations'

// Edits to a pipeline script's leading annotation header (`// on …`, `// mute …`),
// the comment block `parsePipelineAnnotations` reads. Lines are kept in the
// script's own comment style, and nothing past the header is touched.

/** Asset kind → the prefix of its `// on <ref>` spelling. Mirrors ASSET_KINDS in
 * backend/parsers/windmill-parser/src/asset_parser.rs. */
export const ASSET_REF_PREFIX: Record<AssetKind, string> = {
	s3object: 's3://',
	resource: '$res:',
	ducklake: 'ducklake://',
	datatable: 'datatable://',
	volume: 'volume://',
	dbt: 'dbt://'
}

// A `// pipeline` script reading one of these kinds runs after each write to it
// unless muted, with no `// on` line (see AUTO_TRIGGER_KINDS in resolveGraph).
const AUTO_TRIGGER_KINDS: ReadonlySet<AssetKind> = new Set(['ducklake', 's3object'])

export function assetRef(asset: { kind: AssetKind; path: string }): string {
	return `${ASSET_REF_PREFIX[asset.kind]}${asset.path}`
}

const PREFIXES = ['//', '--', '#']

type HeaderLine = { index: number; prefix: string; inner: string }

/** The header: leading comment lines (blank lines allowed), up to the first code. */
function header(lines: string[]): HeaderLine[] {
	const out: HeaderLine[] = []
	for (let i = 0; i < lines.length; i++) {
		const t = lines[i].trimStart()
		if (t === '') continue
		const prefix = PREFIXES.find((p) => t.startsWith(p))
		if (!prefix) break
		out.push({ index: i, prefix, inner: t.slice(prefix.length).trim() })
	}
	return out
}

// A header line matched the way the parser reads it, so `on <ref> debounce=60s`,
// `res://` vs `$res:` spellings and dbt path quoting all count as the same line.
type LineMatch = (parsed: PipelineAnnotations) => boolean
const onAsset =
	(asset: { kind: AssetKind; path: string }): LineMatch =>
	(p) =>
		p.triggerAssets.some((a) => a.kind === asset.kind && a.path === asset.path)
const muteAsset =
	(asset: { kind: AssetKind; path: string }): LineMatch =>
	(p) =>
		p.muteAssets.some((a) => a.kind === asset.kind && a.path === asset.path)
const muteAll: LineMatch = (p) => p.muteAll
const onTrigger =
	(kind: string): LineMatch =>
	(p) =>
		p.nativeTriggers.some((t) => t.kind === kind)

function matches(line: HeaderLine, match: LineMatch): boolean {
	return match(parsePipelineAnnotations(`${line.prefix} ${line.inner}`))
}

function hasDirective(content: string, match: LineMatch): boolean {
	return header(content.split('\n')).some((l) => matches(l, match))
}

/** Adds `<prefix> <directive>` at the end of the header, in its comment style. */
function addDirective(content: string, directive: string): string {
	const lines = content.split('\n')
	const h = header(lines)
	const last = h[h.length - 1]
	const line = `${last?.prefix ?? '//'} ${directive}`
	lines.splice(last ? last.index + 1 : 0, 0, line)
	return lines.join('\n')
}

function removeDirective(content: string, match: LineMatch): string {
	const lines = content.split('\n')
	const drop = new Set(
		header(lines)
			.filter((l) => matches(l, match))
			.map((l) => l.index)
	)
	return lines.filter((_, i) => !drop.has(i)).join('\n')
}

/** The script runs after each write to the asset: `// on <ref>`, and any
 * `// mute <ref>` lifted. An auto-triggering kind needs no `// on` once unmuted,
 * unless `// mute all` still silences it. */
export function listenToAsset(content: string, asset: { kind: AssetKind; path: string }): string {
	const ref = assetRef(asset)
	let out = removeDirective(content, muteAsset(asset))
	const auto = AUTO_TRIGGER_KINDS.has(asset.kind) && !hasDirective(out, muteAll)
	if (!auto && !hasDirective(out, onAsset(asset))) out = addDirective(out, `on ${ref}`)
	return out
}

/** The script no longer runs after writes to the asset: its `// on <ref>` goes,
 * and an auto-triggering kind it `reads` is muted, since the read alone would
 * still trigger it. */
export function stopListeningToAsset(
	content: string,
	asset: { kind: AssetKind; path: string },
	reads: boolean
): string {
	const ref = assetRef(asset)
	let out = removeDirective(content, onAsset(asset))
	if (reads && AUTO_TRIGGER_KINDS.has(asset.kind) && !hasDirective(out, muteAsset(asset))) {
		out = addDirective(out, `mute ${ref}`)
	}
	return out
}

/** Adds a `// on <trigger kind>` line (`schedule`, `webhook`, …) unless it has one. */
export function addTriggerDirective(content: string, kind: string): string {
	return hasDirective(content, onTrigger(kind)) ? content : addDirective(content, `on ${kind}`)
}

/** Drops a `// on <trigger kind>` line (`kafka`, `schedule`, …). */
export function removeTriggerDirective(content: string, kind: string): string {
	return removeDirective(content, onTrigger(kind))
}

/** What each edit says it changes, for the button that applies it. */
export function listenChangeText(asset: { kind: AssetKind; path: string }): string {
	return AUTO_TRIGGER_KINDS.has(asset.kind)
		? `Removes any \`mute ${assetRef(asset)}\` line (adds \`on ${assetRef(asset)}\` if all reads are muted)`
		: `Adds \`on ${assetRef(asset)}\``
}
export function stopListeningChangeText(
	asset: { kind: AssetKind; path: string },
	reads: boolean
): string {
	return reads && AUTO_TRIGGER_KINDS.has(asset.kind)
		? `Removes \`on ${assetRef(asset)}\` and adds \`mute ${assetRef(asset)}\``
		: `Removes \`on ${assetRef(asset)}\``
}
