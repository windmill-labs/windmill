import { extToLang } from '$lib/editorLangUtils'
import { cleanValueProperties, orderedYamlStringify, replaceFalseWithUndefined } from '$lib/utils'

// A raw app rendered as a *folder of files* for diffing. Each entry is one
// virtual file: real `files` keep their natural path, runnables become
// `runnables/<name>`, and the remaining app metadata collapses into a single
// `app.yaml` leaf. Only changed entries are emitted (unchanged are omitted).
export type RawAppDiffStatus = 'added' | 'removed' | 'modified'

export interface RawAppDiffEntry {
	/** Tree path. Real file path, `runnables/<name>`, or `app.yaml`. */
	path: string
	status: RawAppDiffStatus
	/** Original (parent) side content. Undefined when status is `added`. */
	original?: string
	/** Current (fork) side content. Undefined when status is `removed`. */
	current?: string
	/** Monaco language id for the per-file diff editor. */
	lang: string
	/** True for the synthesized metadata leaf. Tracked as a flag (not by matching
	 * `path === 'app.yaml'`) so it survives `reserveUnique` moving the leaf to
	 * `app.yaml~2` when a real file is literally named `app.yaml`. */
	isMetadata?: boolean
}

// Loose shape — diff inputs come from `getItemValue` / drafts and aren't
// strictly typed, and arrive in TWO shapes:
//   - the deployed app row (getAppByPath): `{ summary, policy, custom_path?,
//     value: { files, runnables, data } }` — files/runnables/data nested under
//     `value`.
//   - the flat draft/runtime shape (RawAppDraft / RuntimeRawApp): everything at
//     the top level.
// `normalizeRawApp` coalesces both. Anything not present is empty/absent.
export interface RawAppish {
	value?: Record<string, unknown>
	files?: Record<string, string>
	runnables?: Record<string, unknown>
	summary?: unknown
	data?: unknown
	policy?: unknown
	custom_path?: unknown
	[k: string]: unknown
}

interface NormalizedRawApp {
	files: Record<string, string>
	runnables: Record<string, unknown>
	summary: unknown
	data: unknown
	policy: unknown
	custom_path: unknown
}

export const RAW_APP_METADATA_PATH = 'app.yaml'
const RUNNABLES_PREFIX = 'runnables/'
const METADATA_FIELDS = ['summary', 'data', 'policy', 'custom_path'] as const

// Real file keys may carry a leading slash (`/App.tsx`) which `joinAppPath`
// strips. Collision reservation must compare in the stripped space, else a real
// `/app.yaml` and the synthetic `app.yaml` leaf both become `<appPath>/app.yaml`.
const stripLeadingSlash = (p: string) => p.replace(/^\/+/, '')

function isObject(v: unknown): v is Record<string, unknown> {
	return !!v && typeof v === 'object'
}

function asFileMap(f: unknown): Record<string, string> {
	if (!isObject(f)) return {}
	const out: Record<string, string> = {}
	for (const [k, v] of Object.entries(f)) {
		// Canonicalize the key (strip the leading slash `joinAppPath` would strip
		// anyway) so the same file keyed `/App.tsx` on one side and `App.tsx` on the
		// other is treated as one file — not two leaves colliding at the same
		// composite path. Coerce non-string content so the diff editor gets a string.
		out[stripLeadingSlash(k)] = typeof v === 'string' ? v : String(v ?? '')
	}
	return out
}

// Coalesce the two raw-app shapes (app row with a `value` wrapper vs flat draft)
// into a canonical view. Returns undefined when the whole app is absent.
//
// Per-field precedence is deliberately asymmetric and mirrors the app-row shape:
// the editor payload (`files`/`runnables`/`data`) lives under `value`, so those
// prefer `value` first; the row-level metadata (`summary`/`policy`/`custom_path`)
// lives at the top level, so those prefer `raw` first. Each falls back to the
// other side so a flat draft (everything top-level) still normalizes correctly.
function normalizeRawApp(raw: RawAppish | undefined): NormalizedRawApp | undefined {
	if (!isObject(raw)) return undefined
	const value = isObject(raw.value) ? raw.value : undefined
	return {
		files: asFileMap(value?.files ?? raw.files),
		runnables: isObject(value?.runnables ?? raw.runnables)
			? ((value?.runnables ?? raw.runnables) as Record<string, unknown>)
			: {},
		summary: raw.summary ?? value?.summary,
		data: value?.data ?? raw.data,
		policy: raw.policy ?? value?.policy,
		custom_path: raw.custom_path ?? value?.custom_path
	}
}

// The serialized metadata blob for one side, or undefined when the whole app
// is absent (so the `app.yaml` leaf reads as added/removed).
function metadataYaml(app: NormalizedRawApp | undefined): string | undefined {
	if (!app) return undefined
	const meta: Record<string, unknown> = {}
	for (const field of METADATA_FIELDS) {
		if (app[field] !== undefined) meta[field] = app[field]
	}
	return orderedYamlStringify(meta)
}

function extOf(path: string): string {
	const base = path.split('/').pop() ?? path
	const dot = base.lastIndexOf('.')
	return dot >= 0 ? base.slice(dot + 1).toLowerCase() : ''
}

function diffStatus(o: string | undefined, c: string | undefined): RawAppDiffStatus | undefined {
	if (o === undefined && c !== undefined) return 'added'
	if (o !== undefined && c === undefined) return 'removed'
	if (o !== c) return 'modified'
	return undefined
}

// Reserve a synthesized path, disambiguating against already-taken paths so a
// real file literally named `app.yaml` or under `runnables/` never collides
// with a synthesized leaf.
function reserveUnique(path: string, taken: Set<string>): string {
	if (!taken.has(path)) {
		taken.add(path)
		return path
	}
	let i = 2
	while (taken.has(`${path}~${i}`)) i++
	const p = `${path}~${i}`
	taken.add(p)
	return p
}

/**
 * Diff two raw-app objects into a flat list of changed virtual files. Either
 * side may be undefined (whole app added or removed). Consumers build a tree
 * from the returned paths with `buildFileTree`.
 */
export function parseRawAppDiff(
	original: RawAppish | undefined,
	current: RawAppish | undefined,
	opts: { includeRunnables?: boolean } = {}
): RawAppDiffEntry[] {
	const { includeRunnables = true } = opts
	const oApp = normalizeRawApp(original)
	const cApp = normalizeRawApp(current)
	const oFiles = oApp?.files ?? {}
	const cFiles = cApp?.files ?? {}
	const oRunnables = oApp?.runnables ?? {}
	const cRunnables = cApp?.runnables ?? {}

	// Reserve every real file path (changed or not) up front so synthesized
	// runnable/metadata paths can dodge collisions — slash-normalized so a real
	// `/app.yaml` or `/runnables/x` is seen as colliding with the synthetic leaf.
	const taken = new Set<string>(
		[...Object.keys(oFiles), ...Object.keys(cFiles)].map(stripLeadingSlash)
	)

	const entries: RawAppDiffEntry[] = []

	// Real files.
	for (const path of [...new Set([...Object.keys(oFiles), ...Object.keys(cFiles)])].sort()) {
		const o = Object.prototype.hasOwnProperty.call(oFiles, path) ? oFiles[path] : undefined
		const c = Object.prototype.hasOwnProperty.call(cFiles, path) ? cFiles[path] : undefined
		const status = diffStatus(o, c)
		if (!status) continue
		entries.push({ path, status, original: o, current: c, lang: extToLang(extOf(path)) })
	}

	// Runnables → one YAML leaf each under `runnables/`. Consumers that render
	// runnables as script/flow rows (rawAppDiffToItems) skip these and diff the
	// runnable objects themselves.
	const runnableNames = includeRunnables
		? [...new Set([...Object.keys(oRunnables), ...Object.keys(cRunnables)])].sort()
		: []
	for (const name of runnableNames) {
		const inO = Object.prototype.hasOwnProperty.call(oRunnables, name)
		const inC = Object.prototype.hasOwnProperty.call(cRunnables, name)
		const o = inO ? orderedYamlStringify(oRunnables[name]) : undefined
		const c = inC ? orderedYamlStringify(cRunnables[name]) : undefined
		const status = diffStatus(o, c)
		if (!status) continue
		entries.push({
			path: reserveUnique(`${RUNNABLES_PREFIX}${name}`, taken),
			status,
			original: o,
			current: c,
			lang: 'yaml'
		})
	}

	// Remaining metadata → single `app.yaml` leaf.
	const oMeta = metadataYaml(oApp)
	const cMeta = metadataYaml(cApp)
	const metaStatus = diffStatus(oMeta, cMeta)
	if (metaStatus) {
		entries.push({
			path: reserveUnique(RAW_APP_METADATA_PATH, taken),
			status: metaStatus,
			original: oMeta,
			current: cMeta,
			lang: 'yaml',
			isMetadata: true
		})
	}

	return entries
}

// A raw-app file rendered as a standalone diff item, shaped like a
// WorkspaceItemDiff (so it flows through the existing list / tree / search /
// count machinery) plus the embedded diff payload. `kind: 'raw_app_file'`
// keeps it distinct from the backend kinds. The composite `path`
// (`<appPath>/<file>`) nests it under the app's folder in the tree.
export interface RawAppFileItem {
	kind: 'raw_app_file'
	path: string
	/** Friendly composite path (`<displayAppPath>/<file>`) for tree display only;
	 * `path` stays storage-keyed so a never-deployed draft still loads/edits via
	 * its `…/draft_<uuid>` path. Defaults to `path` when no friendly path differs. */
	displayPath?: string
	ahead: number
	behind: number
	has_changes: boolean
	exists_in_source: boolean
	exists_in_fork: boolean
	/** Workspace path of the owning raw app (for the edit link). */
	appPath: string
	status: RawAppDiffStatus
	original?: string
	current?: string
	lang: string
	/** True for the synthesized `app.yaml` metadata item. */
	isMetadata: boolean
	/** Whole serialized app (both sides) — only on the metadata item, for its
	 * optional "expand to full YAML" view. */
	fullYamlOriginal?: string
	fullYamlCurrent?: string
}

// A raw-app runnable rendered as a script/flow item (what it actually is). It
// carries the reshaped runnable object per side so the normal script-style
// diff (Content + Metadata) renders it — inline code shows with proper syntax
// highlighting instead of a YAML blob. `kind` drives the row icon/label
// (script vs flow); the body is always rendered script-style.
export interface RawAppRunnableItem {
	kind: 'script' | 'flow'
	path: string
	/** Friendly composite path for tree display only (see RawAppFileItem). */
	displayPath?: string
	ahead: number
	behind: number
	has_changes: boolean
	exists_in_source: boolean
	exists_in_fork: boolean
	appPath: string
	status: RawAppDiffStatus
	/** Reshaped runnable (content/language hoisted) for the diff viewer. */
	originalRaw?: unknown
	currentRaw?: unknown
}

export type RawAppSyntheticItem = RawAppFileItem | RawAppRunnableItem

function joinAppPath(appPath: string, filePath: string): string {
	// File keys may carry a leading slash (e.g. `/App.tsx`); strip it so the
	// composite path has single separators and splits cleanly in the tree.
	return `${appPath}/${filePath.replace(/^\/+/, '')}`
}

// The whole serialized app for one side, matching the previous YAML escape
// hatch (cleaned + ordered). Undefined when the app is absent on that side.
function wholeAppYaml(raw: RawAppish | undefined): string | undefined {
	if (!isObject(raw)) return undefined
	return orderedYamlStringify(cleanValueProperties(replaceFalseWithUndefined(raw)))
}

// Hoist an inline runnable's code + language to the top level so the
// script-style diff viewer (which reads top-level `content`/`language`) shows
// the code in the Content tab and the rest as Metadata. Path-referencing
// runnables (no inline script) just fall through to a YAML metadata diff.
function reshapeRunnable(runnable: unknown): unknown {
	if (!isObject(runnable)) return runnable
	const inline = isObject(runnable.inlineScript) ? runnable.inlineScript : undefined
	if (!inline) return runnable
	return {
		...runnable,
		content: inline.content,
		language: inline.language,
		// Drop the now-hoisted code so it isn't duplicated in the Metadata tab.
		inlineScript: { ...inline, content: undefined }
	}
}

// Runnables become script/flow rows; `runType: 'flow'` picks the flow icon.
function runnableKind(...sides: unknown[]): 'script' | 'flow' {
	return sides.some((s) => isObject(s) && s.runType === 'flow') ? 'flow' : 'script'
}

/**
 * Expand a raw-app diff into standalone items for `appPath`:
 * - files + the `app.yaml` metadata leaf → `RawAppFileItem`s (the metadata item
 *   also carries the whole-app YAML for its expand view);
 * - runnables → `RawAppRunnableItem`s (script/flow rows).
 */
export function rawAppDiffToItems(
	appPath: string,
	original: RawAppish | undefined,
	current: RawAppish | undefined,
	displayAppPath: string = appPath
): RawAppSyntheticItem[] {
	// Files + metadata (runnables handled separately as script/flow rows).
	const fileItems = parseRawAppDiff(original, current, { includeRunnables: false }).map(
		(e): RawAppFileItem => ({
			kind: 'raw_app_file',
			path: joinAppPath(appPath, e.path),
			displayPath: joinAppPath(displayAppPath, e.path),
			ahead: 0,
			behind: 0,
			has_changes: true,
			exists_in_source: e.status !== 'added',
			exists_in_fork: e.status !== 'removed',
			appPath,
			status: e.status,
			original: e.original,
			current: e.current,
			lang: e.lang,
			isMetadata: e.isMetadata ?? false
		})
	)
	const metaItem = fileItems.find((i) => i.isMetadata)
	if (metaItem) {
		metaItem.fullYamlOriginal = wholeAppYaml(original)
		metaItem.fullYamlCurrent = wholeAppYaml(current)
	}

	// Runnables → script/flow rows, diffed on their object value.
	const oApp = normalizeRawApp(original)
	const cApp = normalizeRawApp(current)
	const oRun = oApp?.runnables ?? {}
	const cRun = cApp?.runnables ?? {}
	// Reserve each runnable's composite leaf against the real file paths, mirroring
	// parseRawAppDiff, so a real file literally named `runnables/<name>` can't yield
	// a second leaf at the same composite path. Normalize the leading slash (which
	// joinAppPath strips) so `/runnables/x` and `runnables/x` are seen as equal.
	const taken = new Set<string>(
		[...Object.keys(oApp?.files ?? {}), ...Object.keys(cApp?.files ?? {})].map(stripLeadingSlash)
	)
	const runnableItems: RawAppRunnableItem[] = []
	for (const name of [...new Set([...Object.keys(oRun), ...Object.keys(cRun)])].sort()) {
		const inO = Object.prototype.hasOwnProperty.call(oRun, name)
		const inC = Object.prototype.hasOwnProperty.call(cRun, name)
		const oStr = inO ? orderedYamlStringify(oRun[name]) : undefined
		const cStr = inC ? orderedYamlStringify(cRun[name]) : undefined
		const status = diffStatus(oStr, cStr)
		if (!status) continue
		const rel = reserveUnique(`${RUNNABLES_PREFIX}${name}`, taken)
		runnableItems.push({
			kind: runnableKind(oRun[name], cRun[name]),
			path: joinAppPath(appPath, rel),
			displayPath: joinAppPath(displayAppPath, rel),
			ahead: 0,
			behind: 0,
			has_changes: true,
			exists_in_source: status !== 'added',
			exists_in_fork: status !== 'removed',
			appPath,
			status,
			originalRaw: inO ? reshapeRunnable(oRun[name]) : undefined,
			currentRaw: inC ? reshapeRunnable(cRun[name]) : undefined
		})
	}

	return [...fileItems, ...runnableItems]
}
