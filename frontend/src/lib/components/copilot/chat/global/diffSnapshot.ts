/**
 * Materialized draft-vs-deployed diff cache for the chat `diff` tool.
 *
 * The backend has no diff endpoint — it serves item sides, the frontend
 * computes patches. This module fetches each changed item's sides once,
 * computes the stable-YAML patch once, and answers every subsequent query
 * (workspace index, item read, later: search) from memory.
 *
 * Freshness model, cheapest signal first:
 * - `drafts/list` is refetched per access (throttled) — one indexed query that
 *   yields the authoritative row set. The server bumps a draft row's
 *   `created_at` on every update, so an unchanged (path, created_at) pair
 *   proves the draft side of a cached patch is current.
 * - `getWorkspaceDraftsVersion` bumps on every in-app deploy/discard/draft
 *   write; a bump drops cached patches because the deployed side may have
 *   changed without touching any draft row.
 * - Deploys from OTHER clients are invisible to both signals, so index
 *   accesses also refresh patches older than `INDEX_ENTRY_STALE_MS`.
 */
import { get } from 'svelte/store'
import {
	getDraftItems,
	getWorkspaceDraftsVersion,
	type DraftItem
} from '$lib/workspaceDrafts.svelte'
import {
	FlowService,
	ResourceService,
	ScriptService,
	VariableService,
	type UserDraftItemKind,
	type WorkspaceItemDiff
} from '$lib/gen'
import { getDraftDiffValues } from '$lib/utils_draft_deploy'
import { UserDraftDbSyncer } from '$lib/userDraftDbSyncer.svelte'
import { getItemValue } from '$lib/utils_workspace_deploy'
import type { Kind as DeployKind } from '$lib/utils_deployable'
import { userWorkspaces } from '$lib/stores'
import { fetchWorkspaceComparison } from '$lib/workspaceComparison'
import { appSourceToDraftValue } from '$lib/components/raw_apps/rawAppDraftValue'
import { textFilePatch, yamlValuePatch } from './draftDiff'
import { itemTypeForKind } from './userDraftAdapter'
import { TRIGGER_KINDS, type TriggerKind, type WorkspaceItemType } from './workspaceItems'

const LIST_REUSE_MS = 5_000
const INDEX_ENTRY_STALE_MS = 60_000
const READ_ENTRY_REUSE_MS = 15_000
/** Max patches computed per index access, so a workspace with hundreds of
 * drafts still answers its first index quickly; the rest report `pending`
 * and materialize on item read. */
const EAGER_MATERIALIZE_CAP = 50
const FETCH_CONCURRENCY = 6

export type WorkspaceDiffStatus =
	| 'new'
	| 'modified'
	| 'unchanged'
	| 'pending'
	| 'error'
	| 'not_diffable'

/** One changed file inside a multi-file (raw) app. */
export interface DiffFileView {
	status: 'added' | 'deleted' | 'modified'
	patch: string
	lineCount: number
}

export interface WorkspaceDiffEntryView {
	kind: UserDraftItemKind
	/** Chat-facing addressing; undefined when the chat cannot address the kind. */
	type?: WorkspaceItemType
	triggerKind?: TriggerKind
	/** Friendly path the model should use (draft_path when present). */
	path: string
	storagePath: string
	summary?: string
	status: WorkspaceDiffStatus
	/** Unified patch; present once materialized ('' when unchanged). For a
	 * multi-file app this is the config-only patch — file contents live in
	 * `files`. */
	patch?: string
	patchLineCount?: number
	/** Per-file patches for multi-file apps (changed files only). */
	files?: Record<string, DiffFileView>
	/** Variable content was placeholder-masked (values never reach the chat);
	 * the patch marks a value change without revealing it. */
	valueMasked?: boolean
	/** Secret variable: neither side's real value is readable, so "no visible
	 * changes" cannot prove the value is unchanged. */
	valueUncomparable?: boolean
	/** True when the item has never been deployed — the whole draft is new. */
	noDeployed?: boolean
	errorMessage?: string
}

export interface WorkspaceDiffIndexView {
	entries: WorkspaceDiffEntryView[]
	otherUsersDraftCount: number
}

interface Materialized {
	status: 'new' | 'modified' | 'unchanged' | 'error'
	patch: string
	lineCount: number
	files?: Record<string, DiffFileView>
	valueMasked?: boolean
	valueUncomparable?: boolean
	noDeployed: boolean
	errorMessage?: string
	fetchedAt: number
}

/** `files` maps of string contents mark a multi-file app value; anything else
 * (classic apps, other kinds) diffs as one document. */
function extractAppFiles(value: unknown): Record<string, string> | undefined {
	const files = (value as { files?: unknown } | null | undefined)?.files
	if (files == null || typeof files !== 'object' || Array.isArray(files)) return undefined
	const entries = Object.entries(files as Record<string, unknown>)
	if (entries.length === 0 || !entries.every(([, v]) => typeof v === 'string')) return undefined
	return files as Record<string, string>
}

interface AppSplit {
	files: Record<string, DiffFileView>
	configPatch: string
	totalLines: number
	hasChanges: boolean
}

/** Split a multi-file app diff into per-file text patches plus a config-only
 * YAML patch, so code diffs read file-by-file and file addressing works.
 * Returns undefined when neither side carries a file map. */
function computeAppSplit(
	before: unknown,
	after: unknown,
	beforeLabel: string,
	afterLabel: string
): AppSplit | undefined {
	const beforeFiles = extractAppFiles(before)
	const afterFiles = extractAppFiles(after)
	if (!beforeFiles && !afterFiles) return undefined
	const files: Record<string, DiffFileView> = {}
	let totalLines = 0
	const names = [
		...new Set([...Object.keys(beforeFiles ?? {}), ...Object.keys(afterFiles ?? {})])
	].sort()
	for (const name of names) {
		const beforeContent = beforeFiles?.[name]
		const afterContent = afterFiles?.[name]
		const patch = textFilePatch(beforeContent, afterContent, beforeLabel, afterLabel)
		// An EMPTY file appearing or disappearing yields no text patch, but file
		// presence is a change in its own right (imports/bundling see it).
		const presenceChanged = (beforeContent === undefined) !== (afterContent === undefined)
		if (!patch && !presenceChanged) continue
		const lineCount = patch === '' ? 0 : patch.split('\n').length
		totalLines += lineCount
		files[name] = {
			status:
				beforeContent === undefined ? 'added' : afterContent === undefined ? 'deleted' : 'modified',
			patch,
			lineCount
		}
	}
	const withoutFiles = (value: unknown) => {
		if (value == null || typeof value !== 'object') return value
		const { files: _files, ...rest } = value as Record<string, unknown>
		return rest
	}
	const configPatch = yamlValuePatch(
		withoutFiles(before),
		withoutFiles(after),
		beforeLabel,
		afterLabel
	)
	totalLines += configPatch === '' ? 0 : configPatch.split('\n').length
	return {
		files,
		configPatch,
		totalLines,
		hasChanges: Object.keys(files).length > 0 || configPatch !== ''
	}
}

export interface DiffParts {
	/** Whole-value patch, or the config-only patch for a multi-file app. */
	patch: string
	files?: Record<string, DiffFileView>
	lineCount: number
	hasChanges: boolean
}

/** Patch parts for a before/after value pair: per-file text patches for
 * multi-file apps, one whole-value YAML patch otherwise. */
export function computeDiffParts(
	before: unknown,
	after: unknown,
	beforeLabel: string,
	afterLabel: string
): DiffParts {
	const split = computeAppSplit(before, after, beforeLabel, afterLabel)
	if (split) {
		return {
			patch: split.configPatch,
			files: split.files,
			lineCount: split.totalLines,
			hasChanges: split.hasChanges
		}
	}
	const patch = yamlValuePatch(before, after, beforeLabel, afterLabel)
	return {
		patch,
		lineCount: patch === '' ? 0 : patch.split('\n').length,
		hasChanges: patch !== ''
	}
}

interface CacheEntry {
	row: DraftItem
	type?: WorkspaceItemType
	triggerKind?: TriggerKind
	displayPath: string
	materialized?: Materialized
	materializing?: Promise<void>
}

interface WorkspaceCache {
	version: number
	listFetchedAt: number
	entries: Map<string, CacheEntry>
	otherUsersDraftCount: number
}

const caches = new Map<string, WorkspaceCache>()
const reconciling = new Map<string, Promise<WorkspaceCache>>()

function entryKey(kind: UserDraftItemKind, storagePath: string): string {
	return `${kind}:${storagePath}`
}

/** Expire the throttled drafts listing for a workspace so the next access
 * refetches it. Called after a flush persists edits: a flush bumps neither the
 * drafts version nor a row's cached `created_at`, so within `LIST_REUSE_MS`
 * the diff would otherwise be computed from the pre-flush listing. Cached
 * patches survive — reconciliation drops exactly the rows whose `created_at`
 * moved. */
export function expireWorkspaceDiffList(workspace: string): void {
	const cache = caches.get(workspace)
	if (cache) cache.listFetchedAt = 0
}

/** Mark one item's cached patch stale and expire the listing throttle, so the
 * next access refetches both — regardless of the reuse windows. Driven by the
 * syncer's save hook: the moment a draft write lands (an editor autosave, a
 * chat write, a delete), the pre-write patch must never be served again. */
export function markWorkspaceDiffEntryStale(
	workspace: string,
	itemKind: UserDraftItemKind,
	path: string
): void {
	const cache = caches.get(workspace)
	if (!cache) return
	cache.listFetchedAt = 0
	const entry = cache.entries.get(entryKey(itemKind, path))
	if (entry) entry.materialized = undefined
}

// One app-lifetime subscription: every persisted draft write invalidates its
// item eagerly instead of waiting out the listing throttle / read-reuse
// windows. Fork caches are untouched — they compare deployed sides only.
UserDraftDbSyncer.onAnySaved(({ workspace, itemKind, path }) => {
	markWorkspaceDiffEntryStale(workspace, itemKind, path)
})

export function invalidateWorkspaceDiffCache(workspace?: string): void {
	if (workspace === undefined) {
		caches.clear()
		reconciling.clear()
		forkCaches.clear()
		forkReconciling.clear()
	} else {
		caches.delete(workspace)
		reconciling.delete(workspace)
		forkCaches.delete(workspace)
		forkReconciling.delete(workspace)
	}
}

async function mapPool<T>(
	items: T[],
	limit: number,
	fn: (item: T) => Promise<void>
): Promise<void> {
	const queue = [...items]
	const workers = Array.from({ length: Math.min(limit, queue.length) }, async () => {
		for (let item = queue.shift(); item !== undefined; item = queue.shift()) {
			await fn(item)
		}
	})
	await Promise.all(workers)
}

/** Refetch the workspace draft rows and rebuild the entry map, carrying over
 * each cached patch whose draft row is provably unchanged. */
async function reconcile(workspace: string): Promise<WorkspaceCache> {
	const prev = caches.get(workspace)
	const version = getWorkspaceDraftsVersion(workspace)
	if (prev && prev.version === version && Date.now() - prev.listFetchedAt < LIST_REUSE_MS) {
		return prev
	}
	const inflight = reconciling.get(workspace)
	if (inflight) return inflight
	const run = (async () => {
		const rows = await getDraftItems(workspace, true)
		const mine = rows.filter((r) => r.mine)
		const entries = new Map<string, CacheEntry>()
		for (const row of mine) {
			const key = entryKey(row.kind, row.path)
			const old = prev?.entries.get(key)
			const reusable =
				old && prev!.version === version && old.row.created_at === row.created_at
					? old.materialized
					: undefined
			const addressing = itemTypeForKind(row.kind)
			entries.set(key, {
				row,
				type: addressing?.type,
				triggerKind: addressing?.triggerKind,
				displayPath: row.draft_path || row.path,
				materialized: reusable
			})
		}
		const cache: WorkspaceCache = {
			version,
			listFetchedAt: Date.now(),
			entries,
			otherUsersDraftCount: rows.length - mine.length
		}
		caches.set(workspace, cache)
		return cache
	})()
	reconciling.set(workspace, run)
	try {
		return await run
	} finally {
		reconciling.delete(workspace)
	}
}

const VARIABLE_VALUE_PLACEHOLDER = '<variable value — never shown in chat>'
const VARIABLE_VALUE_CHANGED_PLACEHOLDER = '<variable value — never shown in chat (changed)>'

/** Chat-side redaction: variable VALUES never reach a tool result — the same
 * invariant read_workspace_item enforces, and not limited to secrets. The
 * placeholder pair still shows WHETHER the value changed, never its content. */
export function maskVariableDiffSides(
	before: unknown,
	after: unknown
): { before: unknown; after: unknown; valueUncomparable: boolean } {
	const beforeObj =
		before !== null && typeof before === 'object' ? (before as Record<string, unknown>) : undefined
	const afterObj =
		after !== null && typeof after === 'object' ? (after as Record<string, unknown>) : undefined
	// A secret's sides are already masked upstream (draft rows store '' and the
	// deployed value is never decrypted), so equality between them proves
	// nothing — the value may have changed invisibly. Only non-secret sides
	// carry real content worth comparing.
	const secret = beforeObj?.is_secret === true || afterObj?.is_secret === true
	const valueUncomparable = secret && beforeObj !== undefined && afterObj !== undefined
	const valueChanged =
		!secret &&
		beforeObj !== undefined &&
		afterObj !== undefined &&
		JSON.stringify(beforeObj.value) !== JSON.stringify(afterObj.value)
	return {
		before: beforeObj ? { ...beforeObj, value: VARIABLE_VALUE_PLACEHOLDER } : before,
		after: afterObj
			? {
					...afterObj,
					value: valueChanged ? VARIABLE_VALUE_CHANGED_PLACEHOLDER : VARIABLE_VALUE_PLACEHOLDER
				}
			: after,
		valueUncomparable
	}
}

/** Fetch one entry's sides and compute its patch, deduping concurrent calls.
 * Reuses the cached patch when younger than `maxAgeMs`. */
async function materialize(workspace: string, entry: CacheEntry, maxAgeMs: number): Promise<void> {
	if (entry.materialized && Date.now() - entry.materialized.fetchedAt < maxAgeMs) return
	if (entry.materializing) return entry.materializing
	const run = (async () => {
		try {
			const { deployed, draft, noDeployed } = await getDraftDiffValues(
				entry.row.kind,
				entry.row.path,
				workspace
			)
			let before = noDeployed ? undefined : deployed
			let after: unknown = draft
			const valueMasked = entry.row.kind === 'variable'
			let valueUncomparable = false
			if (valueMasked) {
				;({ before, after, valueUncomparable } = maskVariableDiffSides(before, after))
			}
			const parts = computeDiffParts(before, after, 'deployed', 'draft')
			entry.materialized = {
				status: noDeployed ? 'new' : parts.hasChanges ? 'modified' : 'unchanged',
				patch: parts.patch,
				lineCount: parts.lineCount,
				files: parts.files,
				valueMasked,
				valueUncomparable,
				noDeployed,
				fetchedAt: Date.now()
			}
		} catch (e) {
			entry.materialized = {
				status: 'error',
				patch: '',
				lineCount: 0,
				noDeployed: false,
				errorMessage:
					(e as { status?: number } | null | undefined)?.status === 404
						? 'item not found (the draft may reference a deleted item)'
						: ((e as Error | null | undefined)?.message ?? 'failed to compute diff'),
				fetchedAt: Date.now()
			}
		}
	})()
	entry.materializing = run
	try {
		await run
	} finally {
		entry.materializing = undefined
	}
}

function toView(entry: CacheEntry): WorkspaceDiffEntryView {
	const m = entry.materialized
	return {
		kind: entry.row.kind,
		type: entry.type,
		triggerKind: entry.triggerKind,
		path: entry.displayPath,
		storagePath: entry.row.path,
		summary: entry.row.summary,
		status: m ? m.status : entry.type === undefined ? 'not_diffable' : 'pending',
		patch: m?.patch,
		patchLineCount: m?.lineCount,
		files: m?.files,
		valueMasked: m?.valueMasked,
		valueUncomparable: m?.valueUncomparable,
		noDeployed: m?.noDeployed,
		errorMessage: m?.errorMessage
	}
}

/** Workspace index: every draft of the current user with its change status,
 * materializing missing/stale patches up to the eager cap (`materializeAll`
 * lifts the cap — search needs every patch). */
export async function getWorkspaceDiffIndex(
	workspace: string,
	opts: { materializeAll?: boolean } = {}
): Promise<WorkspaceDiffIndexView> {
	const cache = await reconcile(workspace)
	const addressable = [...cache.entries.values()].filter((e) => e.type !== undefined)
	let toMaterialize = addressable.filter(
		(e) => !e.materialized || Date.now() - e.materialized.fetchedAt >= INDEX_ENTRY_STALE_MS
	)
	if (!opts.materializeAll) {
		toMaterialize = toMaterialize.slice(0, EAGER_MATERIALIZE_CAP)
	}
	await mapPool(toMaterialize, FETCH_CONCURRENCY, (e) =>
		materialize(workspace, e, INDEX_ENTRY_STALE_MS)
	)
	return {
		entries: [...cache.entries.values()].map(toView),
		otherUsersDraftCount: cache.otherUsersDraftCount
	}
}

/** Resolve a requested path to the draft row that owns it — by exact key or
 * friendly draft_path — across the given kinds. Item mode flushes/probes the
 * RESOLVED key: a renamed classic app's cell lives at its original storage
 * path, which only the listing knows. */
export async function resolveWorkspaceDiffTarget(
	workspace: string,
	kinds: UserDraftItemKind[],
	path: string
): Promise<{ kind: UserDraftItemKind; storagePath: string } | undefined> {
	const cache = await reconcile(workspace)
	for (const kind of kinds) {
		if (cache.entries.has(entryKey(kind, path))) return { kind, storagePath: path }
	}
	const entry = [...cache.entries.values()].find(
		(e) => kinds.includes(e.row.kind) && (e.displayPath === path || e.row.path === path)
	)
	return entry ? { kind: entry.row.kind, storagePath: entry.row.path } : undefined
}

/** One item's diff entry, addressed by storage path or friendly draft path.
 * Returns undefined when the current user has no draft there. */
export async function readWorkspaceDiffEntry(
	workspace: string,
	itemKind: UserDraftItemKind,
	path: string
): Promise<WorkspaceDiffEntryView | undefined> {
	const cache = await reconcile(workspace)
	let entry = cache.entries.get(entryKey(itemKind, path))
	if (!entry) {
		// Friendly-path addressing (draft-only items park at u/{user}/draft_{uuid})
		// and the classic-app/raw-app kind pair, which share the chat type 'app'.
		const kinds: UserDraftItemKind[] =
			itemKind === 'raw_app' || itemKind === 'app' ? ['raw_app', 'app'] : [itemKind]
		entry = [...cache.entries.values()].find(
			(e) => kinds.includes(e.row.kind) && (e.displayPath === path || e.row.path === path)
		)
	}
	if (!entry) return undefined
	await materialize(workspace, entry, READ_ENTRY_REUSE_MS)
	return toView(entry)
}

// ---------------------------------------------------------------------------
// Fork mode: deployed fork vs deployed parent, mirroring the compare page.
// The index comes from the shared `compareWorkspaces` fetch (same tally the
// fork banner shows); per-item content is fetched with the same
// `getItemValue` canonicalization the compare page's diff drawer uses, so the
// patches match what merge-to-parent would actually ship. Local drafts are
// NOT part of the comparison — they are only flagged on the entries.
// ---------------------------------------------------------------------------

/** How long a fetched comparison keeps serving fork index/read calls before a
 * fresh tally is requested. In-app deploys bump the drafts version and force a
 * refetch regardless. */
const FORK_COMPARISON_REUSE_MS = 30_000

export type ForkDiffStatus =
	| 'modified'
	| 'only_in_fork'
	| 'deleted_in_fork'
	| 'unchanged'
	| 'pending'
	| 'error'

export interface ForkDiffEntryView {
	/** Comparison kind (per-kind trigger names, plus folder / resource_type). */
	kind: string
	/** Chat-facing addressing; undefined when the chat cannot read the kind. */
	type?: WorkspaceItemType
	triggerKind?: TriggerKind
	path: string
	ahead: number
	behind: number
	/** The current user also has a local draft on this item — not part of the
	 * deployed-vs-deployed comparison. */
	hasLocalDraft: boolean
	status: ForkDiffStatus
	patch?: string
	patchLineCount?: number
	/** Per-file patches for multi-file apps (changed files only). */
	files?: Record<string, DiffFileView>
	/** A secret's content was placeholder-masked — content-only changes on
	 * this item cannot appear in the patch. */
	valueMasked?: boolean
	errorMessage?: string
}

export interface ForkDiffIndexView {
	parentWorkspaceId: string
	skippedComparison: boolean
	entries: ForkDiffEntryView[]
	hiddenAheadCount: number
	hiddenBehindCount: number
}

interface ForkMaterialized {
	status: 'modified' | 'only_in_fork' | 'deleted_in_fork' | 'unchanged' | 'error'
	patch: string
	lineCount: number
	files?: Record<string, DiffFileView>
	valueMasked?: boolean
	errorMessage?: string
	fetchedAt: number
}

interface ForkEntry {
	kind: string
	path: string
	ahead: number
	behind: number
	existsInParent: boolean
	existsInFork: boolean
	type?: WorkspaceItemType
	triggerKind?: TriggerKind
	hasLocalDraft: boolean
	materialized?: ForkMaterialized
	materializing?: Promise<void>
}

interface ForkCache {
	parentWorkspaceId: string
	draftsVersion: number
	fetchedAt: number
	skippedComparison: boolean
	entries: Map<string, ForkEntry>
	hiddenAheadCount: number
	hiddenBehindCount: number
}

const forkCaches = new Map<string, ForkCache>()
const forkReconciling = new Map<string, Promise<ForkCache>>()

/** Parent workspace id when `workspace` is a fork/dev workspace, else undefined. */
export function getForkParentWorkspaceId(workspace: string): string | undefined {
	return get(userWorkspaces).find((w) => w.id === workspace)?.parent_workspace_id ?? undefined
}

const CHAT_TRIGGER_KINDS = new Set<string>(TRIGGER_KINDS)

function forkKindAddressing(
	kind: string
): { type: WorkspaceItemType; triggerKind?: TriggerKind } | undefined {
	switch (kind) {
		case 'script':
		case 'flow':
		case 'resource':
		case 'variable':
		case 'schedule':
			return { type: kind as WorkspaceItemType }
		case 'app':
		case 'raw_app':
			return { type: 'app' }
		default: {
			const triggerKind = kind.endsWith('_trigger') ? kind.slice(0, -'_trigger'.length) : undefined
			return triggerKind && CHAT_TRIGGER_KINDS.has(triggerKind)
				? { type: 'trigger', triggerKind: triggerKind as TriggerKind }
				: undefined
		}
	}
}

/** Draft kind holding local drafts for a comparison kind (for the flag join). */
function draftKindForForkKind(kind: string): UserDraftItemKind | undefined {
	if (kind === 'schedule') return 'trigger_schedule'
	if (kind.endsWith('_trigger')) {
		const t = kind.slice(0, -'_trigger'.length)
		return `trigger_${t}` as UserDraftItemKind
	}
	if (kind === 'resource_type' || kind === 'folder') return undefined
	return kind as UserDraftItemKind
}

async function reconcileFork(workspace: string, parentWorkspaceId: string): Promise<ForkCache> {
	const prev = forkCaches.get(workspace)
	const version = getWorkspaceDraftsVersion(workspace)
	if (
		prev &&
		prev.parentWorkspaceId === parentWorkspaceId &&
		prev.draftsVersion === version &&
		Date.now() - prev.fetchedAt < FORK_COMPARISON_REUSE_MS
	) {
		return prev
	}
	const inflight = forkReconciling.get(workspace)
	if (inflight) return inflight
	const run = (async () => {
		// A drafts-version bump means something deployed in-app: demand a fresh
		// tally. Otherwise piggyback on a recent fetch (e.g. the fork banner's).
		const comparisonMaxAge = prev && prev.draftsVersion !== version ? 0 : FORK_COMPARISON_REUSE_MS
		const [comparison, draftsCache] = await Promise.all([
			fetchWorkspaceComparison(parentWorkspaceId, workspace, { maxAgeMs: comparisonMaxAge }),
			reconcile(workspace)
		])
		const localDraftKeys = new Set(
			[...draftsCache.entries.values()].map((e) => entryKey(e.row.kind, e.row.path))
		)
		const entries = new Map<string, ForkEntry>()
		for (const diff of comparison.diffs as WorkspaceItemDiff[]) {
			const key = `${diff.kind}:${diff.path}`
			const old = prev?.entries.get(key)
			const reusable =
				old &&
				prev!.draftsVersion === version &&
				old.ahead === diff.ahead &&
				old.behind === diff.behind &&
				old.existsInParent === diff.exists_in_source &&
				old.existsInFork === diff.exists_in_fork
					? old.materialized
					: undefined
			const addressing = forkKindAddressing(diff.kind)
			const draftKind = draftKindForForkKind(diff.kind)
			entries.set(key, {
				kind: diff.kind,
				path: diff.path,
				ahead: diff.ahead,
				behind: diff.behind,
				existsInParent: diff.exists_in_source,
				existsInFork: diff.exists_in_fork,
				type: addressing?.type,
				triggerKind: addressing?.triggerKind,
				hasLocalDraft: draftKind ? localDraftKeys.has(entryKey(draftKind, diff.path)) : false,
				materialized: reusable
			})
		}
		const cache: ForkCache = {
			parentWorkspaceId,
			draftsVersion: version,
			fetchedAt: Date.now(),
			skippedComparison: comparison.skipped_comparison,
			entries,
			hiddenAheadCount: comparison.hidden_ahead?.total ?? 0,
			hiddenBehindCount: comparison.hidden_behind?.total ?? 0
		}
		forkCaches.set(workspace, cache)
		return cache
	})()
	forkReconciling.set(workspace, run)
	try {
		return await run
	} finally {
		forkReconciling.delete(workspace)
	}
}

// App-row fields that differ between workspaces without being part of what a
// merge deploys (ids, version history, audit fields) — plus bundle_secret,
// which must never reach a tool result.
const APP_ROW_CROSS_WORKSPACE_IGNORE = new Set([
	'id',
	'workspace_id',
	'versions',
	'created_by',
	'created_at',
	'extra_perms',
	'bundle_secret'
])

/** Inline flow scripts pin a `hash` the server recomputes per workspace —
 * never comparable across workspaces. Generic deep walk so nested module
 * shapes (loops, branches) need no taxonomy here. */
function stripInlineFlowHashes(node: unknown): void {
	if (Array.isArray(node)) {
		for (const item of node) stripInlineFlowHashes(item)
		return
	}
	if (node === null || typeof node !== 'object') return
	const obj = node as Record<string, any>
	if (obj.value?.type === 'script' && obj.value.hash != undefined) {
		obj.value.hash = undefined
	}
	for (const child of Object.values(obj)) stripInlineFlowHashes(child)
}

interface ForkSideValue {
	value: unknown
	/** The variable's content was replaced by the placeholder — content-only
	 * changes on this item are invisible in the patch. */
	valueMasked: boolean
}

async function fetchForkSideValue(
	kind: string,
	path: string,
	workspace: string
): Promise<ForkSideValue> {
	// Variable VALUES never reach a tool result — the chat-wide invariant, not
	// just for secrets (and secrets are additionally never decrypted). The
	// placeholder is identical on both sides, so a value-only change is
	// invisible here; the `valueMasked` flag lets callers say so instead of
	// claiming "unchanged".
	if (kind === 'variable') {
		const variable = await VariableService.getVariable({
			workspace,
			path,
			decryptSecret: false
		})
		return {
			value: {
				description: variable.description,
				is_secret: variable.is_secret,
				value: VARIABLE_VALUE_PLACEHOLDER
			},
			valueMasked: true
		}
	}
	// The shared getItemValue projection drops fields the backend comparison
	// DOES consider (script/resource description, resource_type) — a diff of
	// those fields alone would then falsely read "content matches parent".
	// Project those kinds directly with the metadata included.
	if (kind === 'script') {
		const script = await ScriptService.getScriptByPath({ workspace, path })
		return {
			value: {
				content: script.content,
				lock: script.lock,
				schema: script.schema,
				summary: script.summary,
				description: script.description,
				language: script.language
			},
			valueMasked: false
		}
	}
	if (kind === 'resource') {
		const resource = await ResourceService.getResource({ workspace, path })
		return {
			value: {
				value: resource.value,
				description: resource.description,
				resource_type: resource.resource_type
			},
			valueMasked: false
		}
	}
	if (kind === 'flow') {
		// The shared projection drops `schema`, which the backend comparison
		// counts — a schema-only change would falsely read "matches parent".
		const flow = await FlowService.getFlowByPath({ workspace, path })
		const value = structuredClone(flow.value)
		stripInlineFlowHashes(value)
		return {
			value: {
				summary: flow.summary,
				description: flow.description,
				schema: flow.schema,
				value
			},
			valueMasked: false
		}
	}
	if (kind === 'resource_type') {
		const rt = await ResourceService.getResourceType({ workspace, path })
		return {
			value: {
				schema: rt.schema,
				description: rt.description,
				format_extension: rt.format_extension,
				is_fileset: rt.is_fileset
			},
			valueMasked: false
		}
	}
	const value = await getItemValue(kind as DeployKind, path, workspace)
	if ((kind === 'app' || kind === 'raw_app') && value !== null && typeof value === 'object') {
		const row = value as Record<string, unknown>
		// Raw apps: project onto the flat files/runnables draft shape so per-file
		// splitting works and the sides match the draft-mode canonicalization.
		// parent_version is a per-workspace version counter — never comparable
		// across workspaces. Inline-script locks are server-recomputed noise.
		if (kind === 'raw_app' || row.raw_app === true) {
			const canonical = appSourceToDraftValue(row) as Record<string, unknown>
			delete canonical.parent_version
			const runnables = canonical.runnables as Record<string, any> | undefined
			if (runnables) {
				for (const k of Object.keys(runnables)) {
					if (runnables[k]?.inlineScript?.lock != undefined) {
						runnables[k].inlineScript.lock = undefined
					}
				}
			}
			return { value: canonical, valueMasked: false }
		}
		return {
			value: Object.fromEntries(
				Object.entries(row).filter(([k]) => !APP_ROW_CROSS_WORKSPACE_IGNORE.has(k))
			),
			valueMasked: false
		}
	}
	return { value, valueMasked: false }
}

async function materializeFork(
	workspace: string,
	parentWorkspaceId: string,
	entry: ForkEntry,
	maxAgeMs: number
): Promise<void> {
	if (entry.materialized && Date.now() - entry.materialized.fetchedAt < maxAgeMs) return
	if (entry.materializing) return entry.materializing
	const run = (async () => {
		try {
			const [parentSide, forkSide] = await Promise.all([
				entry.existsInParent
					? fetchForkSideValue(entry.kind, entry.path, parentWorkspaceId)
					: undefined,
				entry.existsInFork ? fetchForkSideValue(entry.kind, entry.path, workspace) : undefined
			])
			const parentValue = parentSide?.value
			const forkValue = forkSide?.value
			const valueMasked = parentSide?.valueMasked === true || forkSide?.valueMasked === true
			const oneSidedStatus = !entry.existsInFork
				? 'deleted_in_fork'
				: !entry.existsInParent
					? 'only_in_fork'
					: undefined
			const parts = computeDiffParts(parentValue, forkValue, 'parent', 'fork')
			entry.materialized = {
				status: oneSidedStatus ?? (parts.hasChanges ? 'modified' : 'unchanged'),
				patch: parts.patch,
				lineCount: parts.lineCount,
				files: parts.files,
				valueMasked,
				fetchedAt: Date.now()
			}
		} catch (e) {
			entry.materialized = {
				status: 'error',
				patch: '',
				lineCount: 0,
				errorMessage: (e as Error | null | undefined)?.message ?? 'failed to compute diff',
				fetchedAt: Date.now()
			}
		}
	})()
	entry.materializing = run
	try {
		await run
	} finally {
		entry.materializing = undefined
	}
}

function toForkView(entry: ForkEntry): ForkDiffEntryView {
	const m = entry.materialized
	return {
		kind: entry.kind,
		type: entry.type,
		triggerKind: entry.triggerKind,
		path: entry.path,
		ahead: entry.ahead,
		behind: entry.behind,
		hasLocalDraft: entry.hasLocalDraft,
		status: m?.status ?? 'pending',
		patch: m?.patch,
		patchLineCount: m?.lineCount,
		files: m?.files,
		valueMasked: m?.valueMasked,
		errorMessage: m?.errorMessage
	}
}

/** Cheap comparison metadata (no patch materialization) — lets item/search
 * modes distinguish "no differences" from "comparison unavailable". */
export async function getForkComparisonStatus(
	workspace: string,
	parentWorkspaceId: string
): Promise<{ skippedComparison: boolean }> {
	const cache = await reconcileFork(workspace, parentWorkspaceId)
	return { skippedComparison: cache.skippedComparison }
}

/** Fork index: every item that differs between the fork and its parent
 * (`materializeAll` lifts the eager cap — search needs every patch). */
export async function getForkDiffIndex(
	workspace: string,
	parentWorkspaceId: string,
	opts: { materializeAll?: boolean } = {}
): Promise<ForkDiffIndexView> {
	const cache = await reconcileFork(workspace, parentWorkspaceId)
	let toMaterialize = [...cache.entries.values()].filter(
		(e) => !e.materialized || Date.now() - e.materialized.fetchedAt >= INDEX_ENTRY_STALE_MS
	)
	if (!opts.materializeAll) {
		toMaterialize = toMaterialize.slice(0, EAGER_MATERIALIZE_CAP)
	}
	await mapPool(toMaterialize, FETCH_CONCURRENCY, (e) =>
		materializeFork(workspace, parentWorkspaceId, e, INDEX_ENTRY_STALE_MS)
	)
	return {
		parentWorkspaceId: cache.parentWorkspaceId,
		skippedComparison: cache.skippedComparison,
		entries: [...cache.entries.values()].map(toForkView),
		hiddenAheadCount: cache.hiddenAheadCount,
		hiddenBehindCount: cache.hiddenBehindCount
	}
}

/** Fork-vs-parent entries for one path. `kinds` lists the comparison kinds
 * the chat type maps to (e.g. type 'app' → ['app', 'raw_app']); an EMPTY list
 * is a path-only wildcard — how kinds outside the chat type enum (folder,
 * resource_type, …) stay readable. A wildcard returns EVERY kind differing at
 * the path (nothing in the chat schema could disambiguate them); typed reads
 * return at most one. Empty array = no difference at that path. */
export async function readForkDiffEntries(
	workspace: string,
	parentWorkspaceId: string,
	kinds: string[],
	path: string
): Promise<ForkDiffEntryView[]> {
	const cache = await reconcileFork(workspace, parentWorkspaceId)
	let entries: ForkEntry[] = []
	if (kinds.length === 0) {
		entries = [...cache.entries.values()].filter((e) => e.path === path)
	} else {
		for (const kind of kinds) {
			const entry = cache.entries.get(`${kind}:${path}`)
			if (entry) {
				entries = [entry]
				break
			}
		}
	}
	for (const entry of entries) {
		await materializeFork(workspace, parentWorkspaceId, entry, READ_ENTRY_REUSE_MS)
	}
	return entries.map(toForkView)
}
