import { untrack } from 'svelte'
import { base } from '$lib/base'
import {
	AppService,
	FlowService,
	JobService,
	RawAppService,
	ResourceService,
	ScriptService,
	ScheduleService
} from '$lib/gen'
import { sendUserToast } from '$lib/toast'
import { sleep, emptySchema } from '$lib/utils'
import {
	buildProjectBundle,
	buildPathMap,
	classifyPath,
	extractScriptRefs,
	extractFlowRefs,
	extractAppRefs,
	extractTriggerConfigResourceRefs,
	extractVarRefsFromValue,
	rewriteTriggerConfig,
	rewriteVarRefsInValue,
	type BundleDeps,
	type BundledItem,
	type FetchedItem,
	type ItemKind,
	type ItemRef,
	type ProjectBundle
} from './projectBundle'
import {
	detectDatatableTables,
	generateDatatableMigrations,
	type GeneratedMigration
} from './projectMigrations'
import type { Kind } from '$lib/utils_deployable'
import {
	canRecord,
	canRecordSession,
	inputResourceTypes,
	mergeAppTableOrigin,
	HIDDEN_RESOURCE_TYPES,
	type DeployItem
} from './deployToHubItems'
import type { AssetGraphResponse } from '$lib/components/assets/AssetGraph/types'
import {
	CASCADE_JOB_TIMEOUT_MS,
	CASCADE_POLL_INTERVAL_MS,
	DATA_ASSET_KINDS
} from '$lib/components/assets/AssetGraph/cascadeRun'
import { capturePipelineRecording } from '$lib/components/recording/pipelineRecording.svelte'
import { buildFlowRecording, buildScriptRecording } from '$lib/components/recording/runRecording'
import type { PipelineRecording, RawAppRecording } from '$lib/components/recording/types'
import {
	TRIGGER_KINDS,
	listAllWorkspaceTriggers,
	triggerResourcePath,
	triggerHandlerRefs,
	portableTriggerConfig,
	type WorkspaceTrigger,
	type WorkspaceTriggerKind
} from '../triggers/workspaceTriggersList'

export type Phase = 'predeploy' | 'draft' | 'under_review' | 'live'
// Re-exported so the publish-flow components keep one import site.
export {
	canRecord,
	canRecordSession,
	mergeAppTableOrigin,
	type DeployItem,
	type RecStatus
} from './deployToHubItems'
export function sanitizeSlug(s: string): string {
	return s
		.toLowerCase()
		.replace(/[_\s]+/g, '-')
		.replace(/[^a-z0-9-]/g, '')
		.replace(/-+/g, '-')
		.replace(/^-+|-+$/g, '')
		.slice(0, 50)
		.replace(/-+$/g, '')
}
const SLUG_RE = /^[a-z0-9][a-z0-9-]{1,48}[a-z0-9]$/
export function isValidSlug(s: string): boolean {
	return SLUG_RE.test(s)
}

export type RunState = 'idle' | 'running' | 'success' | 'failed'

const ITEM_KIND_ROUTE: Record<ItemKind, string> = {
	script: 'scripts/get',
	flow: 'flows/get',
	app: 'apps/get',
	raw_app: 'apps_raw/get'
}

// Prune a folder's asset graph to a set of scripts so a pipeline recording only
// runs, renders and samples the project's included members — a deselected branch
// (its nodes, code, logs/results and table samples) never enters the recording.
// Assets kept are only those an included script touches; edges/triggers only
// those anchored on an included runnable.
function pruneGraphToScripts(graph: AssetGraphResponse, scripts: Set<string>): AssetGraphResponse {
	const runnables = graph.runnables.filter((r) => scripts.has(r.path))
	const edges = graph.edges.filter((e) => scripts.has(e.runnable_path))
	const keptAssets = new Set(edges.map((e) => `${e.asset_kind}:${e.asset_path}`))
	const assets = graph.assets.filter((a) => keptAssets.has(`${a.kind}:${a.path}`))
	const triggers = graph.triggers.filter((t) => scripts.has(t.runnable_path))
	const macro_edges = graph.macro_edges?.filter(
		(m) => scripts.has(m.consumer_path) && scripts.has(m.lib_path)
	)
	const test_edges = graph.test_edges?.filter(
		(t) => scripts.has(t.runnable_path) && scripts.has(t.producer_path)
	)
	return { assets, runnables, edges, triggers, macro_edges, test_edges }
}

type DependencyUsage =
	| { role: 'input'; label: string; kind: ItemKind; itemPath: string }
	| { role: 'hardcoded'; label: string; kind: ItemKind; path: string; itemPath: string }
	| { role: 'trigger'; label: string; triggerKind: WorkspaceTriggerKind; path: string }
export interface DependencyType {
	resource_type: string
	hasHardcoded: boolean
	usages: DependencyUsage[]
}

interface SessionDeps {
	hasEeLicense: () => boolean
}

/**
 * All state and async operations for one Deploy-to-Hub surface, bound to an
 * immutable (workspace, folder) pair. A workspace or folder change never mutates
 * a session — `useDeployToHubSession` replaces the instance, so in-flight async
 * work keeps writing to the discarded object and cannot leak into the new scope.
 * The only invalidation tokens left are intra-session (competing calls on the
 * same session), not lifecycle guards.
 */
export class DeployToHubSession {
	readonly workspace: string
	readonly folder: string
	/** `f/`-prefixed folder path the project is scoped to. */
	readonly selectedFolder: string

	#disposed = false
	#deps: SessionDeps

	phase = $state<Phase>('predeploy')
	workspaceItems = $state<DeployItem[]>([])
	draftItems = $state<DeployItem[]>([])
	workspaceTriggers = $state<WorkspaceTrigger[]>([])
	triggersLoading = $state(false)
	// True when a trigger kind's discovery failed (not a feature-gated 404):
	// the trigger list may be incomplete, so publishing is blocked until a
	// retry succeeds.
	triggerDiscoveryFailed = $state(false)
	schedulePreviews = $state<Record<string, string[]>>({})
	manualDeselected = $state<Set<string>>(new Set())
	loading = $state(false)
	deploymentStatus = $state<
		Record<string, { status: 'loading' | 'deployed' | 'failed'; error?: string }>
	>({})
	deploying = $state(false)

	recordTarget = $state<DeployItem | undefined>()
	recordArgs = $state<Record<string, any>>({})
	recordValid = $state(true)
	recordSchema = $state<Record<string, any>>(emptySchema())
	recordSchemaLoading = $state(false)
	runState = $state<RunState>('idle')
	runJobId = $state<string | undefined>(undefined)
	runResult = $state<unknown>(undefined)
	runError = $state<string | undefined>(undefined)
	recordings = $state<Record<string, string>>({})
	// Recent successful runs of the record target: a recording is built from any
	// completed run, so an existing one can be picked instead of running again.
	pastRuns = $state<{ id: string; started_at?: string; duration_ms?: number }[]>([])

	// Project-level data-pipeline recording. Unlike script/flow recordings (one
	// job per item) a pipeline is the whole folder cascade, so it gets a single
	// recording: the resolved asset graph, per-node status timeline, per-node job
	// streams and asset samples — replayed by PipelineRecordingReplay.
	pipelineGraph = $state<AssetGraphResponse | undefined>(undefined)
	pipelineRunState = $state<RunState>('idle')
	pipelineRecordingResult = $state<PipelineRecording | undefined>(undefined)
	pipelineRunError = $state<string | undefined>(undefined)
	pipelineRecorded = $state(false)

	hubName = $state('')
	hubSummary = $state('')
	hubReadme = $state('')
	// Custom logo state for the next publish (png/svg, base64 without the
	// data: prefix). Three-state: undefined = untouched (publishing leaves the
	// Hub's current logo alone), null = clear the Hub's logo on publish,
	// object = upload this image.
	hubLogo = $state<{ b64: string; mime: string; name: string } | null | undefined>(undefined)
	// Whether the Hub currently has a custom logo for this project (from
	// rehydration) — drives the "Remove current logo" affordance.
	hubHasRemoteLogo = $state(false)
	// A pipeline recording is attached on the Hub. An update inherits the published
	// one, which is only a demo of the new version if nothing it runs changed.
	hubHasPipelineRecording = $state(false)
	// The Hub's own verdict: this update runs different content from the published
	// version. False when there is no update in flight.
	hubItemsChanged = $state(false)
	// The attached pipeline recording is the published version's, copied when this
	// update started, rather than one recorded for it. Authoritative across reloads,
	// unlike `pipelineRecorded`, which only remembers this session.
	hubPipelineRecordingInherited = $state(false)
	effectiveSlug = $state('')
	hubItemIds = $state<Record<string, number>>({})
	// Set once the project is published: everything the wizard shows from here on
	// describes an update to it, and the published version keeps serving until that
	// update is approved. `phase` is the update's own status, not the project's.
	liveOnHub = $state(false)
	/** This Hub knows about pending updates — it answers rehydration with a `live`
	 * key. An older one takes a project offline to republish and has neither the
	 * withdraw nor the discard endpoint, so the actions built on them stay hidden. */
	hubSupportsUpdates = $state(false)
	// A reviewer's verdict on the current draft, shown so the publisher knows what
	// to fix before resubmitting.
	rejectionReason = $state<string | undefined>(undefined)
	discardingUpdate = $state(false)
	withdrawing = $state(false)

	// Best-effort data table migrations for the bundle, editable in the drawer and
	// pushed on deploy. Regenerated when the bundle drawer opens.
	migrationDrafts = $state<GeneratedMigration[]>([])
	migrationsGenerating = $state(false)
	// Bumped whenever the drafts are (re)generated, to re-key the Monaco editors so
	// they pick up the fresh SQL (Monaco doesn't sync external `code` changes).
	migrationsGeneration = $state(0)

	// Resource type names declared by the workspace, used to tell a real type from
	// an arbitrary `resource-<x>` arg format. Stays undefined until the list loads.
	resourceTypeNames = $state<Set<string> | undefined>(undefined)
	// Resource types the user explicitly opted into publishing. Opt-in, never
	// derived from the selection: exporting a type definition to the Hub is a
	// deliberate act, so the default is to export none.
	exportedResourceTypes = $state<Set<string>>(new Set())

	bundlePreview = $state<ProjectBundle | undefined>(undefined)
	detectingResources = $state(false)
	// Data tables (→ tables) the current selection reads/writes, detected off the
	// same bundle preview. Drives the predeploy "Data table dependencies" summary;
	// the editable migration itself is generated in the bundle drawer.
	datatableUsage = $state<Map<string, Set<string>>>(new Map())
	detectingDatatables = $state(false)

	submitting = $state(false)
	syncing = $state(false)

	// Set from the Hub's answer to the draft request: this push went into an update
	// rather than over the published project.
	#publishedAsUpdate = false

	// Intra-session tokens: latest call wins among competing calls on this session.
	#triggerLoadTok = 0
	#recordRunTok = 0
	#pipelineRunTok = 0
	#migrationsTok = 0
	#schedulePreviewsInFlight = new Set<string>()
	// Preview-only cache: toggling checkboxes re-runs the closure walk, but item
	// contents don't change mid-session. deployAll bypasses this and fetches fresh.
	#previewItemCache = new Map<string, Promise<FetchedItem | undefined>>()
	#previewTypeCache = new Map<string, Promise<string | undefined>>()

	constructor(workspace: string, folder: string, deps: SessionDeps) {
		this.workspace = workspace
		this.folder = folder
		this.selectedFolder = `f/${folder}`
		this.#deps = deps
	}

	dispose() {
		this.#disposed = true
		// Invalidate any in-flight pipeline cascade poll so it stops on the next
		// tick instead of polling to the timeout against a discarded session.
		this.#pipelineRunTok++
	}

	load() {
		void this.#loadWorkspace()
		void this.#loadResourceTypeNames()
		void this.#loadTriggers()
		void this.rehydrateFromHub()
		void this.#loadPipelineGraph()
	}

	// Deliberately not `resourceTypesStore.getResourceTypes()`: its error path
	// resolves to a non-empty `['error_fetching_names']`, which here would read as
	// a real catalog and filter out every legitimate type. A failure must leave the
	// catalog unset so validation stays off.
	async #loadResourceTypeNames() {
		try {
			const names = await ResourceService.listResourceTypeNames({ workspace: this.workspace })
			if (this.#disposed) return
			this.resourceTypeNames = new Set(names)
		} catch (e: any) {
			console.error('failed to load resource type names, resource type validation is off', e)
		}
	}

	filteredWorkspaceItems = $derived(
		this.workspaceItems.filter((i) => i.path.startsWith(this.selectedFolder + '/'))
	)
	// Derived (not merged at load time) so it settles regardless of which of the
	// racing loads (#loadWorkspace / rehydrateFromHub) finishes last.
	draftItemsWithOrigin = $derived(mergeAppTableOrigin(this.draftItems, this.workspaceItems))
	items = $derived(
		this.phase === 'predeploy' ? this.filteredWorkspaceItems : this.draftItemsWithOrigin
	)
	selectedItems = $derived(
		this.phase === 'predeploy'
			? this.filteredWorkspaceItems.filter((i) => !this.manualDeselected.has(i.key))
			: []
	)
	selectedItemKeys = $derived(this.selectedItems.map((i) => i.key))
	allSelected = $derived(
		this.phase === 'predeploy' &&
			this.selectedItemKeys.length === this.filteredWorkspaceItems.length
	)
	// A raw app's recorded session counts towards the project's recordings just as
	// a script's captured run does — both are what a visitor replays.
	recordableItems = $derived(this.items.filter((i) => canRecord(i.kind) || canRecordSession(i)))
	allRecorded = $derived(
		this.recordableItems.length > 0 && this.recordableItems.every((i) => i.rec === 'recorded')
	)
	// Pipeline members of this project's folder (`// pipeline` scripts).
	pipelineScriptPaths = $derived(
		(this.pipelineGraph?.runnables ?? [])
			.filter((r) => r.usage_kind === 'script' && r.in_pipeline)
			.map((r) => r.path)
	)
	// The subset actually in the Hub project — so a member the user deselected from
	// the bundle is neither executed nor embedded (with its code/logs/samples) in
	// the recording. In the draft phase `items` is the project's membership.
	recordablePipelineScriptPaths = $derived(
		this.pipelineScriptPaths.filter((p) =>
			this.items.some((i) => i.kind === 'script' && i.path === p)
		)
	)
	pipelineScriptPathSet = $derived(new Set(this.pipelineScriptPaths))
	isPipelineProject = $derived(this.pipelineScriptPaths.length > 0)
	/** The pipeline replay this update carries came from the published version, and
	 * something it runs has changed since — so it is a recording of another version.
	 * `hubItemsChanged` is the Hub comparing content, not a guess from which items
	 * carry recordings: an item nobody ever recorded has not changed. */
	pipelineReplayMayBeStale = $derived(
		this.liveOnHub &&
			this.isPipelineProject &&
			this.hubHasPipelineRecording &&
			this.hubPipelineRecordingInherited &&
			this.hubItemsChanged &&
			!this.pipelineRecorded
	)
	hubSlug = $derived(this.effectiveSlug || sanitizeSlug(this.hubName))

	relevantTriggers = $derived.by(() => {
		const selectedScripts = new Set(
			this.selectedItems.filter((i) => i.kind === 'script').map((i) => i.path)
		)
		const selectedFlows = new Set(
			this.selectedItems.filter((i) => i.kind === 'flow').map((i) => i.path)
		)
		return this.workspaceTriggers.filter((t) =>
			t.is_flow ? selectedFlows.has(t.script_path) : selectedScripts.has(t.script_path)
		)
	})

	triggersByKind = $derived.by(() => {
		const out = new Map<WorkspaceTriggerKind, WorkspaceTrigger[]>()
		for (const t of this.relevantTriggers) {
			const arr = out.get(t.kind) ?? []
			arr.push(t)
			out.set(t.kind, arr)
		}
		return Array.from(out.entries()).sort((a, b) => a[0].localeCompare(b[0]))
	})

	runnableSummaryByPath = $derived.by(() => {
		const m = new Map<string, string | undefined>()
		for (const it of this.workspaceItems) {
			if (it.kind === 'script' || it.kind === 'flow') {
				m.set(`${it.kind}:${it.path}`, it.summary)
			}
		}
		return m
	})

	// `hasHardcoded` = pinned via $res: path (relocated as a stub); else input-only.
	dependencyTypes = $derived.by(() => {
		const b = this.bundlePreview
		if (!b) return [] as DependencyType[]
		const stubByNewPath = new Map(b.resourceStubs.map((s) => [s.newPath, s]))
		const byType = new Map<string, DependencyType>()
		const ensure = (rt: string) => {
			let e = byType.get(rt)
			if (!e) {
				e = { resource_type: rt, hasHardcoded: false, usages: [] }
				byType.set(rt, e)
			}
			return e
		}
		for (const it of b.items) {
			const label = (it.summary?.trim() || it.path) ?? it.path
			const refs =
				it.kind === 'flow'
					? extractFlowRefs(it.value).filter((r) => r.kind === 'resource')
					: it.kind === 'app'
						? extractAppRefs(it.value)
						: extractScriptRefs(it.content ?? '')
			for (const r of refs) {
				const stub = stubByNewPath.get(r.path)
				if (!stub || HIDDEN_RESOURCE_TYPES.has(stub.resource_type)) continue
				const e = ensure(stub.resource_type)
				e.hasHardcoded = true
				e.usages.push({
					role: 'hardcoded',
					label,
					kind: it.kind,
					path: stub.originalPath,
					itemPath: it.path
				})
			}
			for (const t of inputResourceTypes(it.schema, this.resourceTypeNames)) {
				ensure(t).usages.push({ role: 'input', label, kind: it.kind, itemPath: it.path })
			}
		}
		// Resources referenced only by a trigger (no item uses them in code) —
		// its kind resource field or any `$res:` token in its config.
		const stubByOriginal = new Map(b.resourceStubs.map((s) => [s.originalPath, s]))
		for (const t of this.relevantTriggers) {
			const refs = new Set(
				extractTriggerConfigResourceRefs(portableTriggerConfig(t.kind, t.config))
			)
			const rp = triggerResourcePath(t)
			if (rp) refs.add(rp)
			for (const ref of refs) {
				const stub = stubByOriginal.get(ref)
				if (!stub || HIDDEN_RESOURCE_TYPES.has(stub.resource_type)) continue
				ensure(stub.resource_type).usages.push({
					role: 'trigger',
					label: t.summary?.trim() || t.path,
					triggerKind: t.kind,
					path: stub.originalPath
				})
			}
		}
		return [...byType.values()].sort((a, b) => a.resource_type.localeCompare(b.resource_type))
	})

	// Only the types the user ticked, restricted to what the current selection
	// actually depends on — deselecting the last item that used a type drops it
	// from the export without the user having to untick it.
	exportedDependencyTypes = $derived(
		this.dependencyTypes
			.map((d) => d.resource_type)
			.filter((rt) => this.exportedResourceTypes.has(rt))
	)

	toggleResourceTypeExport = (resource_type: string) => {
		const next = new Set(this.exportedResourceTypes)
		if (next.has(resource_type)) next.delete(resource_type)
		else next.add(resource_type)
		this.exportedResourceTypes = next
	}

	toggleItem = (item: { key: string }) => {
		const next = new Set(this.manualDeselected)
		if (next.has(item.key)) next.delete(item.key)
		else next.add(item.key)
		this.manualDeselected = next
	}
	selectAll = () => {
		this.manualDeselected = new Set()
	}
	deselectAll = () => {
		this.manualDeselected = new Set(this.filteredWorkspaceItems.map((i) => i.key))
	}

	#folderQs(): string {
		return `?folder=${encodeURIComponent(this.folder)}`
	}

	itemUrl(kind: ItemKind, path: string): string | undefined {
		if (!path) return undefined
		return `${base}/${ITEM_KIND_ROUTE[kind]}/${path}?workspace=${this.workspace}`
	}
	triggerListUrl(kind: WorkspaceTriggerKind): string {
		return `${base}/${TRIGGER_KINDS[kind].route}?workspace=${this.workspace}`
	}

	#patchItem(key: string, patch: Partial<DeployItem>) {
		this.workspaceItems = this.workspaceItems.map((i) => (i.key === key ? { ...i, ...patch } : i))
		this.draftItems = this.draftItems.map((i) => (i.key === key ? { ...i, ...patch } : i))
	}

	async #listAllPages<T>(
		fetcher: (params: { perPage: number; page: number }) => Promise<T[]>
	): Promise<T[]> {
		const perPage = 100
		const out: T[] = []
		for (let page = 1; page <= 1000; page++) {
			const batch = await fetcher({ perPage, page })
			out.push(...batch)
			if (batch.length < perPage) return out
		}
		return out
	}

	async #loadWorkspace() {
		const workspace = this.workspace
		this.loading = true
		try {
			const [apps, rawApps, flows, scripts] = await Promise.all([
				this.#listAllPages((p) => AppService.listApps({ workspace, ...p })),
				this.#listAllPages((p) => RawAppService.listRawApps({ workspace, ...p })),
				this.#listAllPages((p) => FlowService.listFlows({ workspace, ...p })),
				this.#listAllPages((p) => ScriptService.listScripts({ workspace, ...p }))
			])
			if (this.#disposed) return

			const next: DeployItem[] = []
			for (const a of apps) {
				// Raw apps live in the `app` table (value = files/runnables) but must be
				// published to the Hub as raw apps, not low-code apps.
				const isRaw = (a as any).raw_app === true
				next.push({
					key: `${isRaw ? 'raw_app' : 'app'}:${a.path}`,
					path: a.path,
					kind: isRaw ? 'raw_app' : 'app',
					appTable: isRaw || undefined,
					summary: a.summary,
					rec: 'none'
				})
			}
			for (const a of rawApps) {
				next.push({
					key: `raw_app:${a.path}`,
					path: a.path,
					kind: 'raw_app',
					summary: a.summary,
					rec: 'none'
				})
			}
			for (const f of flows) {
				next.push({
					key: `flow:${f.path}`,
					path: f.path,
					kind: 'flow',
					summary: f.summary,
					rec: 'none'
				})
			}
			for (const s of scripts) {
				next.push({
					key: `script:${s.path}`,
					path: s.path,
					kind: 'script',
					summary: s.summary,
					rec: 'none'
				})
			}
			if (this.#disposed) return
			this.workspaceItems = next
		} catch (e: any) {
			if (!this.#disposed) {
				sendUserToast(`Failed to load project items: ${e?.message ?? e}`, true)
			}
		} finally {
			if (!this.#disposed) this.loading = false
		}
	}

	/** Re-fetch triggers, e.g. after the EE license hydrates late. */
	reloadTriggers() {
		void this.#loadTriggers()
	}

	async #loadTriggers() {
		const tok = ++this.#triggerLoadTok
		this.triggersLoading = true
		try {
			const { triggers, failedKinds } = await listAllWorkspaceTriggers(this.workspace, {
				includeEeOnly: this.#deps.hasEeLicense(),
				onError: (message) => {
					if (!this.#disposed) sendUserToast(message, true)
				}
			})
			if (this.#disposed || tok !== this.#triggerLoadTok) return
			this.workspaceTriggers = triggers
			this.triggerDiscoveryFailed = failedKinds.length > 0
		} finally {
			if (!this.#disposed && tok === this.#triggerLoadTok) this.triggersLoading = false
		}
	}

	async rehydrateFromHub() {
		try {
			const res = await fetch(`/api/w/${this.workspace}/hub/project${this.#folderQs()}`, {
				credentials: 'include',
				headers: { accept: 'application/json' }
			})
			if (this.#disposed) return
			if (!res.ok) return // 404 = no project published for this folder yet
			const p = JSON.parse(await res.text())
			if (this.#disposed || !p?.slug) return
			this.effectiveSlug = p.slug
			this.hubName = p.name ?? ''
			this.hubSummary = p.summary ?? ''
			this.hubReadme = p.readme ?? ''
			this.hubHasRemoteLogo = p.has_logo === true
			this.hubHasPipelineRecording = p.has_pipeline_recording === true
			this.hubItemsChanged = p.items_changed === true
			this.hubPipelineRecordingInherited = p.pipeline_recording_inherited === true
			this.rejectionReason = p.rejection_reason ?? undefined
			// `live` is a key this Hub always sends — null unless an update is in
			// flight, in which case the fields above describe that update and the
			// project itself is still published. Its absence means a Hub old enough to
			// still take a project offline while it re-publishes, so the wizard must
			// not promise otherwise.
			this.hubSupportsUpdates = 'live' in p
			this.liveOnHub = this.hubSupportsUpdates && (p.live?.approved === true || p.status === 'live')
			this.phase =
				p.status === 'live' ? 'live' : p.status === 'under_review' ? 'under_review' : 'draft'
			const ids: Record<string, number> = {}
			this.draftItems = (p.items ?? []).map((it: any) => {
				const wpath = it.source_path ?? it.path
				const key = `${it.kind}:${wpath}`
				if (typeof it.hub_id === 'number') ids[key] = it.hub_id
				return {
					key,
					path: wpath,
					kind: it.kind as Kind,
					summary: it.summary ?? undefined,
					rec: it.has_recording ? 'recorded' : 'none'
				} satisfies DeployItem
			})
			this.hubItemIds = ids
		} catch {}
	}

	/** Kick off schedule-preview fetches for any relevant schedule trigger missing one. */
	ensureSchedulePreviews() {
		for (const t of this.relevantTriggers) {
			if (t.kind !== 'schedule') continue
			const c = t.config as any
			const key = `${c.schedule}|${c.timezone}`
			if (this.schedulePreviews[key] || this.#schedulePreviewsInFlight.has(key)) continue
			this.#schedulePreviewsInFlight.add(key)
			ScheduleService.previewSchedule({
				requestBody: {
					schedule: c.schedule,
					timezone: c.timezone,
					cron_version: c.cron_version ?? 'v2'
				}
			})
				.then((dates) => {
					this.schedulePreviews = { ...this.schedulePreviews, [key]: dates.slice(0, 3) }
				})
				.catch(() => {})
				.finally(() => this.#schedulePreviewsInFlight.delete(key))
		}
	}

	/**
	 * Rebuild the predeploy bundle preview (resource + data table dependency
	 * summaries), debounced so rapid checkbox toggles coalesce into one walk.
	 * Reads its reactive inputs synchronously and returns a cancel function, so
	 * it can be driven from an `$effect` with proper cleanup.
	 */
	queueBundlePreview(): (() => void) | undefined {
		if (this.phase !== 'predeploy') {
			this.bundlePreview = undefined
			this.datatableUsage = new Map()
			return undefined
		}
		this.detectingResources = true
		this.detectingDatatables = true
		const slug = this.hubSlug
		const seed: ItemRef[] = [
			...this.selectedItems
				.filter((i) => i.kind !== 'resource')
				.map((i) => ({ kind: i.kind as ItemRef['kind'], path: i.path })),
			...this.#triggerHandlerSeed(this.relevantTriggers, slug)
		]
		const triggerResources = this.#triggerResourcePaths(this.relevantTriggers)
		const triggerVars = this.#triggerVarPaths(this.relevantTriggers)
		let cancelled = false
		const timer = setTimeout(() => {
			buildProjectBundle(seed, slug, this.#cachedBundleDeps(), triggerResources, triggerVars)
				.then((b) => {
					if (cancelled) return
					this.bundlePreview = b
					// Detect data table usage off the same fetched items.
					detectDatatableTables(b.items)
						.then((usage) => {
							if (!cancelled) this.datatableUsage = usage
						})
						.finally(() => {
							if (!cancelled) this.detectingDatatables = false
						})
				})
				.finally(() => {
					if (!cancelled) this.detectingResources = false
				})
		}, 250)
		return () => {
			cancelled = true
			clearTimeout(timer)
		}
	}

	#buildBundleDeps(): BundleDeps {
		const workspace = this.workspace
		return {
			fetchItem: async (ref: ItemRef): Promise<FetchedItem | undefined> => {
				try {
					if (ref.kind === 'script') {
						const s = await ScriptService.getScriptByPath({ workspace, path: ref.path })
						return {
							kind: 'script',
							path: ref.path,
							summary: s.summary,
							description: s.description ?? undefined,
							content: s.content,
							language: s.language,
							schema: s.schema,
							lock: s.lock ?? undefined,
							scriptKind: typeof s.kind === 'string' ? s.kind.toLowerCase() : 'script'
						}
					} else if (ref.kind === 'flow') {
						const f = await FlowService.getFlowByPath({ workspace, path: ref.path })
						return {
							kind: 'flow',
							path: ref.path,
							summary: f.summary,
							description: f.description ?? undefined,
							value: f.value,
							schema: f.schema
						}
					} else if (ref.kind === 'app') {
						const a = await AppService.getAppByPath({ workspace, path: ref.path })
						return { kind: 'app', path: ref.path, summary: a.summary, value: a.value }
					} else if (ref.kind === 'raw_app') {
						// Modern raw apps live in the `app` table: fetch source files +
						// runnables + the compiled bundle, and shape them into the `raw`
						// payload the Hub's RawAppView expects (JSON is valid YAML).
						const isModern = this.workspaceItems.some(
							(i) => i.kind === 'raw_app' && i.path === ref.path && i.appTable
						)
						if (isModern) {
							const a = await AppService.getAppByPath({ workspace, path: ref.path })
							const secret = await AppService.getPublicSecretOfLatestVersionOfApp({
								workspace,
								path: ref.path
							})
							// The compiled JS bundle is required; a missing one means the app
							// was never built/deployed, so fail loudly instead of pushing a blank app.
							const [jsRes, cssRes] = await Promise.all([
								fetch(`/api/w/${workspace}/apps/get_data/v/${secret}.js`, {
									credentials: 'include'
								}),
								fetch(`/api/w/${workspace}/apps/get_data/v/${secret}.css`, {
									credentials: 'include'
								})
							])
							if (!jsRes.ok) {
								throw new Error(`raw app ${ref.path} has no compiled bundle — deploy it first`)
							}
							const js = await jsRes.text()
							const css = cssRes.ok ? await cssRes.text() : ''
							const v: any = a.value ?? {}
							const content = JSON.stringify({
								files: { ...(v.files ?? {}), '/bundle.js': js, '/bundle.css': css },
								runnables: v.runnables ?? {},
								// Preserve the full-code app's explicit data table declaration so it
								// survives publish/import and feeds migration detection.
								...(v.data !== undefined ? { data: v.data } : {}),
								...(v.datatables !== undefined ? { datatables: v.datatables } : {})
							})
							return { kind: 'raw_app', path: ref.path, summary: a.summary, content }
						}
						const r = await fetch(`/api/w/${workspace}/raw_apps/get_data/0/${ref.path}`, {
							credentials: 'include'
						})
						if (!r.ok) return undefined
						return { kind: 'raw_app', path: ref.path, content: await r.text() }
					}
				} catch (e: any) {
					return undefined
				}
				return undefined
			},
			resolveResourceType: async (path: string): Promise<string | undefined> => {
				try {
					const r = await ResourceService.getResource({ workspace, path })
					return r.resource_type ?? undefined
				} catch (e: any) {
					return undefined
				}
			}
		}
	}

	#cachedBundleDeps(): BundleDeps {
		const deps = this.#buildBundleDeps()
		// Memoize only successful lookups: a miss (undefined) is likely transient, so
		// evict it once it resolves. Otherwise a fixed/retried dependency can never
		// clear `bundlePreview.unresolved` until the whole session is recreated.
		const memoize = <T>(
			cache: Map<string, Promise<T | undefined>>,
			key: string,
			run: () => Promise<T | undefined>
		) => {
			let p = cache.get(key)
			if (!p) {
				p = run()
				cache.set(key, p)
				void p.then((r) => {
					if (r === undefined && cache.get(key) === p) cache.delete(key)
				})
			}
			return p
		}
		return {
			fetchItem: (ref) =>
				memoize(this.#previewItemCache, `${ref.kind}:${ref.path}`, () => deps.fetchItem(ref)),
			resolveResourceType: (path) =>
				memoize(this.#previewTypeCache, path, () => deps.resolveResourceType(path))
		}
	}

	async #postHub(path: string, body: unknown): Promise<Record<string, any> | undefined> {
		const res = await fetch(`/api/w/${this.workspace}${path}${this.#folderQs()}`, {
			method: 'POST',
			headers: { 'Content-Type': 'application/json' },
			credentials: 'include',
			body: JSON.stringify(body)
		})
		const text = await res.text()
		if (!res.ok) throw new Error(text)
		try {
			return JSON.parse(text)
		} catch {
			return undefined
		}
	}

	async regenerateMigrations() {
		const tok = ++this.#migrationsTok
		this.migrationsGenerating = true
		try {
			// Same handler-augmented seed as deployAll: a data table used only by a
			// bundled trigger handler must still get its migration.
			const seed: ItemRef[] = [
				...this.selectedItems
					.filter((i) => i.kind !== 'resource')
					.map((i) => ({ kind: i.kind as ItemRef['kind'], path: i.path })),
				...this.#triggerHandlerSeed(this.relevantTriggers, this.hubSlug || 'project')
			]
			// Detection is independent of the final slug (data table refs aren't
			// relocated), so any placeholder slug works for this throwaway bundle.
			const bundle = await buildProjectBundle(
				seed,
				this.hubSlug || 'project',
				this.#buildBundleDeps(),
				[]
			)
			const usage = await detectDatatableTables(bundle.items)
			const drafts = await generateDatatableMigrations(this.workspace, usage)
			if (this.#disposed || tok !== this.#migrationsTok) return
			this.migrationDrafts = drafts
			this.migrationsGeneration++
		} catch (e: any) {
			if (!this.#disposed && tok === this.#migrationsTok) {
				this.migrationDrafts = []
				this.migrationsGeneration++
				// Toast so a genuine failure isn't mistaken for "no data table usage".
				sendUserToast(`Could not generate data table migrations: ${e?.message ?? e}`, true)
			}
		} finally {
			if (!this.#disposed && tok === this.#migrationsTok) this.migrationsGenerating = false
		}
	}

	/** Prefill bundle metadata and start migration detection (bundle drawer opening). */
	prepareBundle() {
		this.hubName = this.hubName || this.folder
		void this.regenerateMigrations()
	}

	/**
	 * Create the Hub draft then push the full bundle. `deploying` is set
	 * synchronously before the first request so a double-click cannot start a
	 * second publish, and the whole run is refused while triggers are still
	 * loading — an incomplete `relevantTriggers` snapshot would permanently
	 * omit triggers (and their handlers and migrations) from the draft.
	 * `onDraftCreated` fires once the draft exists (the bundle drawer closes
	 * there while items continue publishing).
	 */
	async publishBundle(onDraftCreated?: () => void): Promise<void> {
		if (this.deploying || this.triggersLoading || this.triggerDiscoveryFailed) return
		this.deploying = true
		try {
			// Captured at click time rather than read in #deployAll, which only runs
			// after the draft request resolves: what gets published must be what was
			// ticked on confirmation, whatever mutates `exportedResourceTypes` after.
			const exportedTypes = new Set(this.exportedResourceTypes)
			if (!(await this.#createDraft())) return
			onDraftCreated?.()
			await this.#deployAll(exportedTypes)
		} finally {
			this.deploying = false
		}
	}

	/**
	 * Create the Hub draft project. Returns true when the draft exists and
	 * publishing can proceed.
	 */
	async #createDraft(): Promise<boolean> {
		this.hubName = this.hubName.trim()
		this.hubSummary = this.hubSummary.trim()
		this.hubReadme = this.hubReadme.trim()
		try {
			const res = await fetch(`/api/w/${this.workspace}/hub/publish_draft${this.#folderQs()}`, {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				credentials: 'include',
				body: JSON.stringify({
					slug: this.hubSlug,
					name: this.hubName,
					summary: this.hubSummary || this.hubName,
					readme: this.hubReadme || undefined
				})
			})
			const text = await res.text()
			if (!res.ok) {
				sendUserToast(`Hub draft creation failed: ${text}`, true)
				return false
			}
			// Abort if Hub didn't echo a slug — guessing here lands items under
			// a folder the Hub never locked.
			let returnedSlug: string | undefined
			try {
				const parsed = JSON.parse(text)
				if (typeof parsed?.slug === 'string') returnedSlug = parsed.slug
				// The Hub decides this: publishing over an approved project goes into a
				// pending update instead, and the project keeps serving meanwhile.
				this.#publishedAsUpdate = parsed?.pending_revision === true
			} catch {}
			if (!returnedSlug) {
				sendUserToast(`Hub did not return a slug. Aborting publish to avoid path drift.`, true)
				return false
			}
			// Session replaced mid-request (workspace/folder switch): publishing now
			// would push another scope's items into this draft. Abort.
			if (this.#disposed) {
				sendUserToast(`Workspace changed during publish — aborted to avoid mixing items.`, true)
				return false
			}
			this.effectiveSlug = returnedSlug
			return true
		} catch (e: any) {
			sendUserToast(`Hub draft creation failed: ${e?.message ?? e}`, true)
			return false
		}
	}

	async #pushBundledItem(slug: string, it: BundledItem): Promise<void> {
		const key = `${it.kind}:${it.path}`
		if (it.kind === 'script') {
			const resp = await this.#postHub('/hub/scripts', {
				summary: it.summary || it.newPath,
				app: slug,
				description: it.description ?? '',
				kind: it.scriptKind ?? 'script',
				content: it.content,
				language: it.language,
				schema: it.schema ?? undefined,
				lockfile: it.lock ?? undefined,
				path: it.newPath,
				source_path: it.path,
				project_slug: slug
			})
			if (typeof resp?.id === 'number') this.hubItemIds = { ...this.hubItemIds, [key]: resp.id }
		} else if (it.kind === 'flow') {
			const resp = await this.#postHub('/hub/flows', {
				flow: {
					summary: it.summary || it.newPath,
					description: it.description ?? undefined,
					value: it.value,
					schema: it.schema ?? undefined
				},
				apps: [],
				path: it.newPath,
				source_path: it.path,
				project_slug: slug
			})
			if (typeof resp?.id === 'number') this.hubItemIds = { ...this.hubItemIds, [key]: resp.id }
		} else if (it.kind === 'app') {
			await this.#postHub('/hub/apps', {
				app: it.value,
				apps: [],
				summary: it.summary || it.newPath,
				description: undefined,
				path: it.newPath,
				source_path: it.path,
				project_slug: slug
			})
		} else if (it.kind === 'raw_app') {
			const resp = await this.#postHub('/hub/raw_apps', {
				raw: it.content ?? '',
				apps: [],
				summary: it.summary || it.newPath,
				path: it.newPath,
				source_path: it.path,
				description: undefined,
				project_slug: slug
			})
			if (typeof resp?.id === 'number') this.hubItemIds = { ...this.hubItemIds, [key]: resp.id }
		}
	}

	// Handler runnables (trigger error handlers, schedule on_* handlers) ship
	// with the bundle like the primary runnables do; hub refs stay external.
	#triggerHandlerSeed(triggers: WorkspaceTrigger[], slug: string): ItemRef[] {
		return triggers.flatMap(triggerHandlerRefs).filter((r) => classifyPath(r.path, slug) !== 'hub')
	}

	// Every resource a trigger's exported config references: the kind-specific
	// broker/auth field plus any `$res:` token nested in it (schedule args,
	// handler extra args, …) — all must enter the bundle path map.
	#triggerResourcePaths(triggers: WorkspaceTrigger[]): string[] {
		const out = new Set<string>()
		for (const t of triggers) {
			const rp = triggerResourcePath(t)
			if (rp) out.add(rp)
			for (const p of extractTriggerConfigResourceRefs(portableTriggerConfig(t.kind, t.config))) {
				out.add(p)
			}
		}
		return [...out]
	}

	// Every whole-string `$var:`/`$jsonvar:` value a trigger's config resolves (SQS
	// queue_url, schedule args, …) — relocated through the bundle map like item vars.
	#triggerVarPaths(triggers: WorkspaceTrigger[]): string[] {
		const out = new Set<string>()
		for (const t of triggers) {
			for (const p of extractVarRefsFromValue(portableTriggerConfig(t.kind, t.config))) out.add(p)
		}
		return [...out]
	}

	async #pushTriggers(
		slug: string,
		resourcePathMap: Map<string, string>,
		relevant: WorkspaceTrigger[]
	): Promise<void> {
		const pathMap = buildPathMap(
			relevant.map((t) => t.path),
			slug
		)
		const triggers: Array<Record<string, unknown>> = []
		const skipped: string[] = []
		for (const t of relevant) {
			const itemKind: ItemKind = t.is_flow ? 'flow' : 'script'
			const runnableKey = `${itemKind}:${t.script_path}`
			const hubId = this.hubItemIds[runnableKey]
			if (!hubId) {
				skipped.push(t.path)
				continue
			}
			// Full-config remap: resource paths, error-handler paths, schedule on_*
			// handler refs and whole-string `$var:` values all relocate through the map.
			const config = rewriteVarRefsInValue(
				rewriteTriggerConfig(portableTriggerConfig(t.kind, t.config), resourcePathMap),
				resourcePathMap
			)
			triggers.push({
				path: pathMap.get(t.path) ?? t.path,
				kind: t.kind,
				summary: t.summary ?? null,
				description: (t.config as any)?.description ?? null,
				config,
				script_ask_id: t.is_flow ? null : hubId,
				flow_id: t.is_flow ? hubId : null
			})
		}
		if (skipped.length > 0) {
			sendUserToast(
				`Skipped ${skipped.length} trigger(s) whose runnable did not publish: ${skipped.join(', ')}`,
				true
			)
		}
		// Full-set sync: always push (an empty list clears the Hub's triggers on a
		// re-deploy), so removing every trigger doesn't leave stale ones on the Hub.
		await this.#postHub('/hub/triggers', { triggers, project_slug: slug })
	}

	// Builtin types (git_repository, ...) aren't in resource_type — push with empty schema.
	async #pushResourceTypes(slug: string, types: string[]): Promise<number> {
		const results = await Promise.all(
			types.map(async (name) => {
				let schema: unknown = undefined
				let description: string | undefined = undefined
				try {
					const rt = await ResourceService.getResourceType({
						workspace: this.workspace,
						path: name
					})
					schema = rt.schema ?? undefined
					description = rt.description ?? undefined
				} catch (e: any) {}
				try {
					await this.#postHub('/hub/resource_types', {
						name,
						schema,
						description,
						project_slug: slug
					})
					return 0
				} catch (e: any) {
					sendUserToast(`Resource type ${name} push failed: ${e?.message ?? e}`, true)
					return 1
				}
			})
		)
		return results.reduce((a: number, b) => a + b, 0)
	}

	async #deployAll(exportedTypes: Set<string>) {
		const slug = this.hubSlug
		// Snapshot the selection up-front: `selectedItems`/`relevantTriggers` are
		// derived from live workspace data and `migrationDrafts` is edited in the
		// drawer — the deploy must publish exactly what the user confirmed.
		const itemsSnapshot = this.selectedItems.slice()
		const triggersSnapshot = this.relevantTriggers.slice()
		const migrationsSnapshot = this.migrationDrafts.slice()
		this.hubItemIds = {}
		this.deploymentStatus = {}
		let failures = 0
		try {
			const seed: ItemRef[] = [
				...itemsSnapshot
					.filter((i) => i.kind !== 'resource')
					.map((i) => ({ kind: i.kind as ItemRef['kind'], path: i.path })),
				...this.#triggerHandlerSeed(triggersSnapshot, slug)
			]
			const triggerResources = this.#triggerResourcePaths(triggersSnapshot)
			const triggerVars = this.#triggerVarPaths(triggersSnapshot)
			const bundle = await buildProjectBundle(
				seed,
				slug,
				this.#buildBundleDeps(),
				triggerResources,
				triggerVars
			)
			// Full path map (incl. unresolved) so a trigger's resource path is always
			// relocated — never leaks the publisher's original private path to the Hub.
			const resourcePathMap = bundle.pathMap

			// A dangling reference (a selected root or transitive runnable that failed
			// to fetch, or a resource whose type can't be resolved) means the bundle
			// doesn't close: the root would silently vanish, or a published item would
			// still point at the publisher's private source-workspace path. Refuse to
			// publish until every reference resolves rather than ship a broken project.
			if (bundle.unresolved.length > 0) {
				sendUserToast(
					`Cannot publish: ${bundle.unresolved.length} unresolved reference(s): ${bundle.unresolved.join(', ')}. Deselect or fix them, then retry.`,
					true
				)
				return
			}

			// Bundle building is slow — bail before the first Hub write if the session
			// was replaced (workspace/folder switch) in the meantime.
			if (this.#disposed) return

			// Types come from $res: stubs AND schema inputs (resource-<type>). A stub's
			// type is declared by an existing resource, so it needs no validation; an
			// input's is a free-form format string, so it does.
			const inputTypes = bundle.items.flatMap((i) =>
				inputResourceTypes(i.schema, this.resourceTypeNames)
			)
			const types = [
				...new Set([...bundle.resourceStubs.map((s) => s.resource_type), ...inputTypes])
			]
			// Only the types the user ticked are published. The others still get their
			// stub below, so a fork knows which credential to fill; only the type
			// definition itself stays out of the Hub.
			const depFailures = await this.#pushResourceTypes(
				slug,
				types.filter((t) => exportedTypes.has(t))
			)

			// Input-type deps with no path get a conventional f/<slug>/<type> stub.
			const stubsByPath = new Map<string, { path: string; resource_type: string }>()
			for (const s of bundle.resourceStubs)
				stubsByPath.set(s.newPath, { path: s.newPath, resource_type: s.resource_type })
			for (const t of inputTypes) {
				const path = `f/${slug}/${t}`
				if (!stubsByPath.has(path)) stubsByPath.set(path, { path, resource_type: t })
			}
			const stubs = [...stubsByPath.values()]
			if (stubs.length > 0) {
				try {
					await this.#postHub('/hub/resources', { resources: stubs, project_slug: slug })
				} catch (e: any) {
					sendUserToast(`Resource sync failed: ${e?.message ?? e}`, true)
					failures++
				}
			}
			failures += depFailures
			if (failures > 0) {
				sendUserToast(
					`Resource dependency sync failed — items not published to avoid broken references.`,
					true
				)
				return
			}

			for (const it of bundle.items) {
				// Stop writing item status / Hub IDs once the session is replaced —
				// continuing would publish into a project the user has moved away from.
				if (this.#disposed) return
				const key = `${it.kind}:${it.path}`
				this.deploymentStatus = { ...this.deploymentStatus, [key]: { status: 'loading' } }
				try {
					await this.#pushBundledItem(slug, it)
					this.deploymentStatus = { ...this.deploymentStatus, [key]: { status: 'deployed' } }
				} catch (e: any) {
					failures++
					this.deploymentStatus = {
						...this.deploymentStatus,
						[key]: { status: 'failed', error: e?.message ?? String(e) }
					}
				}
			}
			if (this.#disposed) return
			try {
				await this.#pushTriggers(slug, resourcePathMap, triggersSnapshot)
			} catch (e: any) {
				sendUserToast(`Trigger sync failed: ${e?.message ?? e}`, true)
				failures++
			}

			// Full-set sync: always push (an empty list clears the Hub's migrations on
			// a re-deploy). The Hub drops empty-SQL entries, so disabled placeholders
			// don't persist.
			try {
				await this.#postHub('/hub/migrations', {
					migrations: migrationsSnapshot.map((m) => ({
						datatable_name: m.datatable_name,
						sql: m.sql,
						sql_down: m.sql_down,
						enabled: m.enabled
					})),
					project_slug: slug
				})
			} catch (e: any) {
				sendUserToast(`Data table migration sync failed: ${e?.message ?? e}`, true)
				failures++
			}

			// Push the logo only when touched this session: an object uploads it,
			// null clears the Hub's current logo, undefined leaves it alone
			// (re-publishing a bundle must not clear it).
			if (this.hubLogo !== undefined) {
				try {
					await this.#postHub(`/hub/projects/${encodeURIComponent(slug)}/logo`, {
						logo: this.hubLogo ? { b64: this.hubLogo.b64, mime: this.hubLogo.mime } : null
					})
					this.hubHasRemoteLogo = this.hubLogo !== null
					this.hubLogo = undefined
				} catch (e: any) {
					sendUserToast(
						`Logo ${this.hubLogo ? 'upload' : 'removal'} failed: ${e?.message ?? e}`,
						true
					)
					failures++
				}
			}

			await sleep(150)
			if (this.#disposed) return
			// An incomplete push must never become submittable: a failed transitive item
			// can leave a pushed runnable pointing at content that never landed. Stay in
			// predeploy (deploymentStatus keeps the failed items visible) so re-publishing
			// retries every write — createDraft and the item pushes are idempotent.
			if (failures > 0) {
				sendUserToast(
					`Publish incomplete: ${failures} write(s) failed. Nothing was submitted — fix them and re-publish.`,
					true
				)
				return
			}
			this.deploymentStatus = {}
			this.recordings = {}
			// Deterministic baseline so a transient Hub read failure can't leave the
			// UI stuck in `predeploy`; rehydrate then upgrades to authoritative state.
			this.draftItems = itemsSnapshot.map((i) => ({ ...i, rec: 'none' }))
			this.phase = 'draft'
			const asUpdate = this.#publishedAsUpdate
			await this.rehydrateFromHub()
			sendUserToast(
				asUpdate
					? `Update ready on the Hub. Your published project stays live until it is approved.`
					: `Draft created on the Hub. Add recordings before submitting for review.`
			)
		} finally {
			this.deploying = false
		}
	}

	submitForReview = async () => {
		const slug = this.hubSlug
		if (!slug) return
		this.submitting = true
		try {
			const res = await fetch(
				`/api/w/${this.workspace}/hub/projects/${encodeURIComponent(slug)}/submit${this.#folderQs()}`,
				{
					method: 'POST',
					credentials: 'include',
					headers: { 'Content-Type': 'application/json' },
					body: '{}'
				}
			)
			if (!res.ok) {
				sendUserToast(`Submit for review failed: ${await res.text()}`, true)
				return
			}
			this.phase = 'under_review'
			sendUserToast('Submitted for review by the Windmill team.')
		} finally {
			this.submitting = false
		}
	}

	syncWithHub = async () => {
		this.syncing = true
		try {
			if (this.phase === 'draft') {
				await this.#loadWorkspace()
				const prev = new Map(this.draftItems.map((i) => [i.key, { rec: i.rec }]))
				this.draftItems = this.workspaceItems
					.filter((i) => prev.has(i.key))
					.map((i) => ({ ...i, rec: prev.get(i.key)?.rec ?? 'none' }))
			} else {
				// under_review / live: re-fetch the Hub project to pick up an
				// admin status change (under_review -> live).
				const before = this.phase
				await this.rehydrateFromHub()
				sendUserToast(
					this.phase === before
						? 'Still waiting for review.'
						: this.phase === 'live'
							? 'Approved — your project is now live.'
							: `Status updated: ${this.phase}.`
				)
			}
		} catch (e: any) {
			sendUserToast(`Sync failed: ${e?.message ?? e}`, true)
		} finally {
			this.syncing = false
		}
	}

	/** Go back to picking items, to publish again. Local only — nothing reaches the
	 * Hub until the bundle is confirmed, and where the Hub supports updates the
	 * published version keeps serving even then. */
	startNewDraft = () => {
		this.draftItems = []
		this.recordings = {}
		this.rejectionReason = undefined
		// All of it belongs to the update just finished, not the one starting. The
		// captured cascade especially: left in place, the next update could save a
		// replay of the version it replaces. Bumping the token first abandons a run
		// still in flight, which would otherwise write its result back over this.
		this.#pipelineRunTok++
		this.pipelineRecorded = false
		this.pipelineRecordingResult = undefined
		this.pipelineRunState = 'idle'
		this.pipelineRunError = undefined
		this.phase = 'predeploy'
	}

	/** Take the submission back out of review. Everything pushed for it is kept, so
	 * it can be fixed and submitted again. */
	cancelSubmission = async () => {
		if (this.withdrawing) return
		const slug = this.effectiveSlug
		if (!slug) return
		this.withdrawing = true
		try {
			const res = await fetch(
				`/api/w/${this.workspace}/hub/projects/${encodeURIComponent(slug)}/withdraw${this.#folderQs()}`,
				{ method: 'POST', credentials: 'include' }
			)
			if (!res.ok) {
				sendUserToast(`Could not cancel the submission: ${await res.text()}`, true)
				return
			}
			if (this.#disposed) return
			this.phase = 'draft'
			await this.rehydrateFromHub()
			sendUserToast(`Submission cancelled. Everything you pushed is still here.`)
		} catch (e: any) {
			sendUserToast(`Could not cancel the submission: ${e?.message ?? e}`, true)
		} finally {
			this.withdrawing = false
		}
	}

	/** Throw away an update in progress and go back to what is published. */
	discardUpdate = async () => {
		if (this.discardingUpdate) return
		const slug = this.effectiveSlug
		if (!slug) return
		this.discardingUpdate = true
		try {
			const res = await fetch(
				`/api/w/${this.workspace}/hub/projects/${encodeURIComponent(slug)}/discard_update${this.#folderQs()}`,
				{ method: 'POST', credentials: 'include' }
			)
			if (!res.ok) {
				sendUserToast(`Could not discard the update: ${await res.text()}`, true)
				return
			}
			if (this.#disposed) return
			this.draftItems = []
			this.recordings = {}
			this.deploymentStatus = {}
			this.rejectionReason = undefined
			this.phase = 'live'
			await this.rehydrateFromHub()
			sendUserToast(`Update discarded. The published project is unchanged.`)
		} catch (e: any) {
			sendUserToast(`Could not discard the update: ${e?.message ?? e}`, true)
		} finally {
			this.discardingUpdate = false
		}
	}

	/** Reset record-drawer state and load the target's schema. */
	async openRecord(it: DeployItem) {
		const tok = ++this.#recordRunTok
		this.recordTarget = it
		this.recordArgs = {}
		this.recordValid = true
		this.recordSchema = emptySchema()
		this.recordSchemaLoading = true
		this.runState = 'idle'
		this.runJobId = undefined
		this.runResult = undefined
		this.runError = undefined
		this.pastRuns = []
		this.#loadPastRuns(it, tok)
		try {
			if (it.kind === 'script') {
				const s = await ScriptService.getScriptByPath({
					workspace: this.workspace,
					path: it.path
				})
				if (tok !== this.#recordRunTok) return
				this.recordSchema = (s.schema as Record<string, any>) ?? emptySchema()
			} else if (it.kind === 'flow') {
				const f = await FlowService.getFlowByPath({ workspace: this.workspace, path: it.path })
				if (tok !== this.#recordRunTok) return
				this.recordSchema = (f.schema as Record<string, any>) ?? emptySchema()
			}
		} catch (e: any) {
			if (tok !== this.#recordRunTok) return
			sendUserToast(`Failed to load schema: ${e?.message ?? e}`, true)
		} finally {
			if (tok === this.#recordRunTok) this.recordSchemaLoading = false
		}
	}

	/** Invalidate any in-flight record run/poll (record drawer closed). */
	cancelRecordRun = () => {
		this.#recordRunTok++
	}

	/** Recent successful runs of the target, so one can be recorded as-is.
	 * Best-effort — an empty list just means the drawer only offers a fresh run. */
	async #loadPastRuns(it: DeployItem, tok: number) {
		if (it.kind !== 'script' && it.kind !== 'flow') return
		try {
			const runs = await JobService.listCompletedJobs({
				workspace: this.workspace,
				jobKinds: it.kind === 'script' ? 'script' : 'flow',
				scriptPathExact: it.path,
				status: 'success',
				// Standalone runs only — a script that also runs as a flow step would
				// otherwise list its child jobs.
				hasNullParent: true,
				orderDesc: true,
				perPage: 5
			})
			if (tok !== this.#recordRunTok) return
			this.pastRuns = runs.map((j) => ({
				id: j.id,
				started_at: j.started_at,
				duration_ms: j.duration_ms
			}))
		} catch {
			// best-effort
		}
	}

	/** Adopt an existing successful run as the one to save: recordings are built
	 * from completed jobs, so it goes through the exact same save path as a
	 * fresh run. No token bump — there is no poll to cancel (the picker is
	 * hidden while a run is in flight) and bumping would strand `openRecord`'s
	 * still-loading schema fetch on "Loading schema…" forever. */
	useExistingRun = (jobId: string) => {
		this.runJobId = jobId
		this.runState = 'success'
		this.runResult = undefined
		this.runError = undefined
	}

	runJob = async () => {
		const it = this.recordTarget
		if (!it) return
		const tok = ++this.#recordRunTok
		this.runState = 'running'
		this.runJobId = undefined
		this.runResult = undefined
		this.runError = undefined
		try {
			let jobId: string
			if (it.kind === 'script') {
				jobId = await JobService.runScriptByPath({
					workspace: this.workspace,
					path: it.path,
					requestBody: this.recordArgs
				})
			} else if (it.kind === 'flow') {
				jobId = await JobService.runFlowByPath({
					workspace: this.workspace,
					path: it.path,
					requestBody: this.recordArgs
				})
			} else {
				if (tok === this.#recordRunTok) this.runState = 'idle'
				return
			}
			if (tok !== this.#recordRunTok) return
			this.runJobId = jobId
			await this.#pollJobUntilComplete(jobId, tok)
		} catch (e: any) {
			if (tok !== this.#recordRunTok) return
			this.runState = 'failed'
			this.runError = `Failed to start: ${e?.message ?? e}`
		}
	}

	async #pollJobUntilComplete(jobId: string, tok: number) {
		// First check immediately (fast scripts complete in ms), then back off to 2s.
		const deadline = Date.now() + 5 * 60_000
		let interval = 250
		while (Date.now() < deadline) {
			if (tok !== this.#recordRunTok) return
			try {
				const r = await JobService.getCompletedJobResultMaybe({
					workspace: this.workspace,
					id: jobId
				})
				if (tok !== this.#recordRunTok) return
				if (r.completed) {
					this.runResult = r.result
					if (r.success) {
						this.runState = 'success'
					} else {
						this.runState = 'failed'
						this.runError = typeof r.result === 'string' ? r.result : JSON.stringify(r.result)
					}
					return
				}
			} catch (e: any) {
				if (tok !== this.#recordRunTok) return
				this.runState = 'failed'
				this.runError = `Polling failed: ${e?.message ?? e}`
				return
			}
			await sleep(interval)
			interval = Math.min(interval * 2, 2000)
		}
		if (tok !== this.#recordRunTok) return
		this.runState = 'failed'
		this.runError = 'Timed out after 5 minutes'
	}

	async #buildScriptRecording(it: DeployItem, jobId: string) {
		const workspace = this.workspace
		// Pin the code to the version the run executed — the picker can select a
		// run older than the currently deployed script, and publishing current
		// code with an old run's logs/result would misrepresent both.
		const job = (await JobService.getJob({ workspace, id: jobId })) as any
		let s = job.script_hash
			? await ScriptService.getScriptByHash({ workspace, hash: job.script_hash }).catch(
					() => undefined
				)
			: undefined
		if (!s) {
			s = await ScriptService.getScriptByPath({ workspace, path: it.path })
			sendUserToast(
				"The run's script version could not be resolved — the recording pairs it with the current code, which may not match what ran.",
				true
			)
		}
		return await buildScriptRecording(workspace, jobId, {
			scriptPath: it.path,
			code: s.content,
			language: s.language,
			schema: s.schema as Record<string, any> | undefined
		})
	}

	async #buildFlowRecording(it: DeployItem, jobId: string) {
		const workspace = this.workspace
		// Pin the definition (value, schema) to the version the run executed —
		// the recorded statuses reference its module ids and the recorded args
		// its input schema, so the current flow may not match. The job's
		// script_hash is the flow version id.
		const root = (await JobService.getJob({ workspace, id: jobId })) as any
		let flow: { value: unknown; schema?: unknown; summary?: string } | undefined
		if (root.script_hash) {
			flow = await FlowService.getFlowVersion({
				workspace,
				version: parseInt(root.script_hash, 16)
			}).catch(() => undefined)
		}
		if (!flow && root.raw_flow) {
			// The API materialized the executed value on the job; the input schema
			// has to come from the current flow, which may have drifted.
			const f = await FlowService.getFlowByPath({ workspace, path: it.path })
			flow = { value: root.raw_flow, schema: f.schema, summary: f.summary }
			sendUserToast(
				"The run's flow version could not be resolved — the recording uses the executed graph but the current input schema, which may not match the recorded arguments.",
				true
			)
		}
		if (!flow) {
			const f = await FlowService.getFlowByPath({ workspace, path: it.path })
			flow = f
			sendUserToast(
				"The run's flow version could not be resolved — the recording pairs it with the current definition, which may not match what ran.",
				true
			)
		}
		return await buildFlowRecording(workspace, jobId, it.path, {
			value: flow.value as any,
			schema: (flow.schema as Record<string, unknown> | undefined) ?? {
				type: 'object',
				properties: {},
				required: []
			},
			summary: flow.summary ?? ''
		})
	}

	/** Save a recorded raw-app session as that app's Hub recording. */
	async saveAppRecording(it: DeployItem, recording: RawAppRecording): Promise<boolean> {
		const hubId = this.hubItemIds[it.key]
		if (!hubId) {
			sendUserToast(`Push the bundle to the Hub first before saving recordings`, true)
			return false
		}
		try {
			await this.#postHub(`/hub/raw_apps/${hubId}/recording`, {
				recording,
				project_slug: this.hubSlug
			})
			this.#patchItem(it.key, { rec: 'recorded' })
			sendUserToast(`Recording saved — ${recording.steps.length} steps`)
			return true
		} catch (e: any) {
			sendUserToast(`Failed to save recording: ${e?.message ?? e}`, true)
			return false
		}
	}

	/** Save the current successful run as the Hub recording. Returns true on success. */
	async saveRecording(): Promise<boolean> {
		const it = this.recordTarget
		if (!it || !this.runJobId || this.runState !== 'success') return false
		const hubId = this.hubItemIds[it.key]
		if (!hubId) {
			sendUserToast(`Push the bundle to the Hub first before saving recordings`, true)
			return false
		}
		if (it.kind !== 'script' && it.kind !== 'flow') {
			sendUserToast(`Recordings only supported for script/flow`, true)
			return false
		}
		try {
			const recording =
				it.kind === 'script'
					? await this.#buildScriptRecording(it, this.runJobId)
					: await this.#buildFlowRecording(it, this.runJobId)
			const path = it.kind === 'script' ? 'scripts' : 'flows'
			await this.#postHub(`/hub/${path}/${hubId}/recording`, {
				recording,
				project_slug: this.hubSlug
			})
			this.recordings = { ...this.recordings, [it.key]: this.runJobId }
			this.#patchItem(it.key, { rec: 'recorded' })
			sendUserToast(`Recording saved — job ${this.runJobId}`)
			return true
		} catch (e: any) {
			sendUserToast(`Failed to save recording: ${e?.message ?? e}`, true)
			return false
		}
	}

	/** Resolve the project folder's asset graph so a data-pipeline project can be
	 * detected and its whole-folder cascade recorded. Best-effort — a project
	 * with no pipeline just never shows the pipeline record card. */
	async #loadPipelineGraph() {
		try {
			const params = new URLSearchParams({
				folder: this.folder,
				asset_kinds: DATA_ASSET_KINDS.join(',')
			})
			const res = await fetch(`/api/w/${this.workspace}/assets/graph?${params}`, {
				credentials: 'include'
			})
			if (!res.ok) throw new Error(`GET /assets/graph → ${res.status}`)
			const graph = (await res.json()) as AssetGraphResponse
			if (this.#disposed) return
			this.pipelineGraph = graph
		} catch {
			// No pipeline graph — the pipeline record card simply stays hidden.
		}
	}

	/** Run the whole-folder cascade and capture it into a single PipelineRecording.
	 * Deployed-only (no drafts) and arg-less — unlike the editor it seeds no
	 * per-node input, so a root that needs uploaded data or a schedule's static
	 * payload records a failure the user can see and fix rather than a green run. */
	runPipelineRecording = async () => {
		const fullGraph = this.pipelineGraph
		const scripts = this.recordablePipelineScriptPaths
		if (!fullGraph || scripts.length === 0) return
		const scriptSet = new Set(scripts)
		// Scope the graph to the project's members so the run, the recorded graph
		// (rendered by the player) and the asset samples all exclude deselected
		// branches.
		const graph = pruneGraphToScripts(fullGraph, scriptSet)
		const tok = ++this.#pipelineRunTok
		this.pipelineRunState = 'running'
		this.pipelineRecordingResult = undefined
		this.pipelineRunError = undefined
		// A previous save's badge must not linger over a fresh, unsaved re-run.
		this.pipelineRecorded = false
		const workspace = this.workspace
		try {
			const { recording, result } = await capturePipelineRecording({
				workspace,
				folder: this.folder,
				graph,
				scriptPaths: scriptSet,
				launch: (path) =>
					JobService.runScriptByPath({
						workspace,
						path,
						// Skip the backend asset-trigger dispatcher: the cascade engine owns
						// the whole closure (parity with the pipeline editor's bounded run).
						requestBody: { _wmill_skip_asset_dispatch: true }
					}),
				waitTerminal: (jobId) => this.#waitJobTerminal(jobId, tok)
			})
			if (tok !== this.#pipelineRunTok) return
			this.pipelineRecordingResult = recording
			// A dependency cycle drops its members from the schedule, so an all- or
			// partially-cyclic run leaves the recording missing steps (and an empty
			// schedule reports `ok`). Treat any dropped cyclic member as a failure so
			// an incomplete pipeline can't be saved as a successful recording.
			if (result.cyclic.length > 0) {
				this.pipelineRunState = 'failed'
				this.pipelineRunError = `Cannot record — ${result.cyclic.length} script(s) on a dependency cycle: ${result.cyclic.join(', ')}`
			} else if (result.ok) {
				this.pipelineRunState = 'success'
			} else {
				this.pipelineRunState = 'failed'
				const failed = [...result.statuses.entries()]
					.filter(([, s]) => s.status === 'failure')
					.map(([p]) => p)
				this.pipelineRunError =
					failed.length > 0 ? `Failed at ${failed.join(', ')}` : 'Cascade did not complete'
			}
		} catch (e: any) {
			if (tok !== this.#pipelineRunTok) return
			this.pipelineRunState = 'failed'
			this.pipelineRunError = `Failed to run pipeline: ${e?.message ?? e}`
		}
	}

	// Poll a launched step to terminal, matching the pipeline editor's cascade
	// timeout (DuckLake/DuckDB steps routinely exceed a few minutes). Adds the
	// `#pipelineRunTok` cancellation the shared `makeWaitJobTerminal` lacks.
	async #waitJobTerminal(jobId: string, tok: number): Promise<'success' | 'failure'> {
		const deadline = Date.now() + CASCADE_JOB_TIMEOUT_MS
		while (Date.now() < deadline) {
			if (tok !== this.#pipelineRunTok) throw new Error('cancelled')
			try {
				const r = await JobService.getCompletedJobResultMaybe({
					workspace: this.workspace,
					id: jobId,
					getStarted: false
				})
				if (r.completed) return r.success ? 'success' : 'failure'
			} catch {
				// transient — retry on the next tick
			}
			await sleep(CASCADE_POLL_INTERVAL_MS)
		}
		throw new Error(`Timed out waiting for job ${jobId}`)
	}

	/** Save the captured pipeline recording to the Hub, scoped to the project
	 * (a pipeline is the whole folder, not a single Hub item). Returns true on
	 * success. */
	async savePipelineRecording(): Promise<boolean> {
		const recording = this.pipelineRecordingResult
		if (!recording || this.pipelineRunState !== 'success') return false
		if (this.phase === 'predeploy') {
			sendUserToast(`Push the project to the Hub first before saving its pipeline recording`, true)
			return false
		}
		try {
			await this.#postHub(`/hub/projects/${this.hubSlug}/pipeline_recording`, { recording })
			this.pipelineRecorded = true
			sendUserToast(`Pipeline recording saved`)
			return true
		} catch (e: any) {
			sendUserToast(`Failed to save pipeline recording: ${e?.message ?? e}`, true)
			return false
		}
	}
}

/**
 * Owns the session lifecycle: a new `DeployToHubSession` is created whenever the
 * (workspace, folder) identity actually changes — a spurious same-value store
 * emit reuses the live session — and the previous one is disposed, which is the
 * single mechanism invalidating its in-flight work. Also hosts the reactive
 * plumbing the session itself can't (license-hydration reload, schedule
 * previews, debounced bundle preview).
 */
export function useDeployToHubSession(args: {
	workspace: () => string | undefined
	folder: () => string
	hasEeLicense: () => boolean
}) {
	let session = $state<DeployToHubSession | undefined>()

	$effect(() => {
		const workspace = args.workspace()
		const folder = args.folder()
		if (!workspace) return
		untrack(() => {
			if (session && session.workspace === workspace && session.folder === folder) return
			session?.dispose()
			const next = new DeployToHubSession(workspace, folder, {
				hasEeLicense: args.hasEeLicense
			})
			session = next
			next.load()
		})
	})

	// The EE license hydrates async; if it lands after a license-less trigger load,
	// EE kinds stay empty. Re-fetch on false→true (the session reads the license
	// getter at call time).
	let prevHadLicense: boolean | undefined = undefined
	$effect(() => {
		const hasLicense = args.hasEeLicense()
		untrack(() => {
			if (hasLicense && prevHadLicense === false) session?.reloadTriggers()
			prevHadLicense = hasLicense
		})
	})

	// Leaving/entering predeploy invalidates manual selection tweaks.
	$effect(() => {
		const s = session
		if (!s) return
		s.phase
		untrack(() => {
			s.manualDeselected = new Set()
		})
	})

	// Schedule previews for relevant schedule triggers (deduped in the session).
	$effect(() => {
		session?.ensureSchedulePreviews()
	})

	// Debounced predeploy bundle preview; the session reads its reactive inputs
	// synchronously and returns the cancel function used as effect cleanup.
	$effect(() => {
		const s = session
		if (!s) return
		return s.queueBundlePreview()
	})

	return {
		get session() {
			return session
		}
	}
}
