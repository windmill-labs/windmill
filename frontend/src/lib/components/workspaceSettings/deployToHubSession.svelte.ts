import { untrack } from 'svelte'
import { base } from '$lib/base'
import {
	AppService,
	FlowService,
	JobService,
	RawAppService,
	ResourceService,
	ScriptService,
	WorkspaceService,
	ScheduleService
} from '$lib/gen'
import { sendUserToast } from '$lib/toast'
import { sleep, emptySchema } from '$lib/utils'
import { computeSecretUrl } from '$lib/components/apps/editor/appDeploy.svelte'
import {
	buildProjectBundle,
	buildPathMap,
	classifyPath,
	extractScriptRefs,
	extractFlowRefs,
	extractAppRefs,
	rewriteTriggerConfig,
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
	TRIGGER_KINDS,
	listAllWorkspaceTriggers,
	triggerResourcePath,
	triggerHandlerRefs,
	stripTriggerConfig,
	type WorkspaceTrigger,
	type WorkspaceTriggerKind
} from '../triggers/workspaceTriggersList'

export type Phase = 'predeploy' | 'draft' | 'under_review' | 'live'
export type RecStatus = 'none' | 'recorded'
export interface DeployItem {
	key: string
	path: string
	kind: Kind
	summary?: string
	rec: RecStatus
	published?: boolean
	publicUrl?: string
	[k: string]: unknown
}

export const canRecord = (k: Kind) => k === 'script' || k === 'flow'
export const canPublishApp = (k: Kind) => k === 'app' || k === 'raw_app'

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

const HIDDEN_RESOURCE_TYPES = new Set(['app_theme', 'state', 'cache'])

function typesFromSchema(schema: any): string[] {
	const out = new Set<string>()
	const props = schema?.properties
	if (props && typeof props === 'object') {
		for (const key of Object.keys(props)) {
			const fmt = props[key]?.format
			if (typeof fmt === 'string' && fmt.startsWith('resource-')) {
				out.add(fmt.slice('resource-'.length))
			}
		}
	}
	return [...out]
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
	workspaceRateLimit = $state<number | undefined>(undefined)
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

	publishTarget = $state<DeployItem | undefined>()
	publishing = $state(false)

	hubName = $state('')
	hubSummary = $state('')
	hubReadme = $state('')
	effectiveSlug = $state('')
	hubItemIds = $state<Record<string, number>>({})

	// Best-effort data table migrations for the bundle, editable in the drawer and
	// pushed on deploy. Regenerated when the bundle drawer opens.
	migrationDrafts = $state<GeneratedMigration[]>([])
	migrationsGenerating = $state(false)
	// Bumped whenever the drafts are (re)generated, to re-key the Monaco editors so
	// they pick up the fresh SQL (Monaco doesn't sync external `code` changes).
	migrationsGeneration = $state(0)

	bundlePreview = $state<ProjectBundle | undefined>(undefined)
	detectingResources = $state(false)
	// Data tables (→ tables) the current selection reads/writes, detected off the
	// same bundle preview. Drives the predeploy "Data table dependencies" summary;
	// the editable migration itself is generated in the bundle drawer.
	datatableUsage = $state<Map<string, Set<string>>>(new Map())
	detectingDatatables = $state(false)

	submitting = $state(false)
	syncing = $state(false)

	// Intra-session tokens: latest call wins among competing calls on this session.
	#triggerLoadTok = 0
	#recordRunTok = 0
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
	}

	load() {
		void this.#loadWorkspace()
		void this.#loadTriggers()
		void this.rehydrateFromHub()
	}

	filteredWorkspaceItems = $derived(
		this.workspaceItems.filter((i) => i.path.startsWith(this.selectedFolder + '/'))
	)
	items = $derived(this.phase === 'predeploy' ? this.filteredWorkspaceItems : this.draftItems)
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
	recordableItems = $derived(this.items.filter((i) => canRecord(i.kind)))
	allRecorded = $derived(
		this.recordableItems.length > 0 && this.recordableItems.every((i) => i.rec === 'recorded')
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
			for (const t of typesFromSchema(it.schema)) {
				if (HIDDEN_RESOURCE_TYPES.has(t)) continue
				ensure(t).usages.push({ role: 'input', label, kind: it.kind, itemPath: it.path })
			}
		}
		// Resources referenced only by a trigger (no item uses them in code).
		const stubByOriginal = new Map(b.resourceStubs.map((s) => [s.originalPath, s]))
		for (const t of this.relevantTriggers) {
			const rp = triggerResourcePath(t)
			const stub = rp ? stubByOriginal.get(rp) : undefined
			if (!stub || HIDDEN_RESOURCE_TYPES.has(stub.resource_type)) continue
			ensure(stub.resource_type).usages.push({
				role: 'trigger',
				label: t.summary?.trim() || t.path,
				triggerKind: t.kind,
				path: stub.originalPath
			})
		}
		return [...byType.values()].sort((a, b) => a.resource_type.localeCompare(b.resource_type))
	})

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
			const [apps, rawApps, flows, scripts, settings] = await Promise.all([
				this.#listAllPages((p) => AppService.listApps({ workspace, ...p })),
				this.#listAllPages((p) => RawAppService.listRawApps({ workspace, ...p })),
				this.#listAllPages((p) => FlowService.listFlows({ workspace, ...p })),
				this.#listAllPages((p) => ScriptService.listScripts({ workspace, ...p })),
				WorkspaceService.getSettings({ workspace }).catch(() => undefined)
			])
			if (this.#disposed) return

			this.workspaceRateLimit = settings?.public_app_execution_limit_per_minute

			const next: DeployItem[] = []
			const publicApps = apps.filter((a) => a.execution_mode === 'anonymous')
			const publicUrls = await Promise.all(publicApps.map((a) => this.#resolvePublicUrl(a.path)))
			const publicUrlByPath = new Map(publicApps.map((a, i) => [a.path, publicUrls[i]]))
			for (const a of apps) {
				const isPublic = a.execution_mode === 'anonymous'
				// Raw apps live in the `app` table (value = files/runnables) but must be
				// published to the Hub as raw apps, not low-code apps.
				const isRaw = (a as any).raw_app === true
				next.push({
					key: `${isRaw ? 'raw_app' : 'app'}:${a.path}`,
					path: a.path,
					kind: isRaw ? 'raw_app' : 'app',
					appTable: isRaw || undefined,
					summary: a.summary,
					rec: 'none',
					published: isPublic,
					publicUrl: isPublic ? publicUrlByPath.get(a.path) : undefined
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

	async #resolvePublicUrl(path: string): Promise<string | undefined> {
		try {
			const secret = await AppService.getPublicSecretOfApp({ workspace: this.workspace, path })
			return computeSecretUrl(secret)
		} catch {
			return undefined
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
		const triggerResources = this.relevantTriggers
			.map(triggerResourcePath)
			.filter((p): p is string => !!p)
		let cancelled = false
		const timer = setTimeout(() => {
			buildProjectBundle(seed, slug, this.#cachedBundleDeps(), triggerResources)
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
		return {
			fetchItem: (ref) => {
				const key = `${ref.kind}:${ref.path}`
				let p = this.#previewItemCache.get(key)
				if (!p) {
					p = deps.fetchItem(ref)
					this.#previewItemCache.set(key, p)
				}
				return p
			},
			resolveResourceType: (path) => {
				let p = this.#previewTypeCache.get(path)
				if (!p) {
					p = deps.resolveResourceType(path)
					this.#previewTypeCache.set(path, p)
				}
				return p
			}
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
			if (!(await this.#createDraft())) return
			onDraftCreated?.()
			await this.#deployAll()
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

	async #pushTriggers(
		slug: string,
		resourcePathMap: Map<string, string>,
		relevant: WorkspaceTrigger[]
	): Promise<void> {
		if (relevant.length === 0) return
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
			// Full-config remap: resource paths, error-handler paths and schedule
			// on_* handler refs all relocate through the bundle's path map.
			const config = rewriteTriggerConfig(stripTriggerConfig(t.config), resourcePathMap)
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
		if (triggers.length === 0) return
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

	async #deployAll() {
		const slug = this.hubSlug
		// Snapshot the selection up-front: `selectedItems`/`relevantTriggers` are
		// derived from live workspace data and `migrationDrafts` is edited in the
		// drawer — the deploy must publish exactly what the user confirmed.
		const itemsSnapshot = this.selectedItems.slice()
		const triggersSnapshot = this.relevantTriggers.slice()
		const migrationsSnapshot = this.migrationDrafts.slice()
		this.hubItemIds = {}
		let failures = 0
		try {
			const seed: ItemRef[] = [
				...itemsSnapshot
					.filter((i) => i.kind !== 'resource')
					.map((i) => ({ kind: i.kind as ItemRef['kind'], path: i.path })),
				...this.#triggerHandlerSeed(triggersSnapshot, slug)
			]
			const triggerResources = triggersSnapshot
				.map(triggerResourcePath)
				.filter((p): p is string => !!p)
			const bundle = await buildProjectBundle(seed, slug, this.#buildBundleDeps(), triggerResources)
			// Full path map (incl. unresolved) so a trigger's resource path is always
			// relocated — never leaks the publisher's original private path to the Hub.
			const resourcePathMap = bundle.pathMap

			if (bundle.unresolved.length > 0) {
				sendUserToast(
					`Skipped ${bundle.unresolved.length} unresolved reference(s): ${bundle.unresolved.join(', ')}`,
					true
				)
			}

			// Bundle building is slow — bail before the first Hub write if the session
			// was replaced (workspace/folder switch) in the meantime.
			if (this.#disposed) return

			// Types come from $res: stubs AND schema inputs (resource-<type>).
			const inputTypes = bundle.items
				.flatMap((i) => typesFromSchema(i.schema))
				.filter((t) => !HIDDEN_RESOURCE_TYPES.has(t))
			const types = [
				...new Set([...bundle.resourceStubs.map((s) => s.resource_type), ...inputTypes])
			]
			const depFailures = await this.#pushResourceTypes(slug, types)

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
			// A re-bundle clears the Hub-side embed (idempotent replace), so re-push it
			// for any raw app that is already public — keeps the live iframe in sync
			// without forcing an unpublish/share round-trip. Updates by hub id, safe in parallel.
			await Promise.all(
				bundle.items
					.filter((it) => it.kind === 'raw_app')
					.map(async (it) => {
						const hubId = this.hubItemIds[`${it.kind}:${it.path}`]
						const src = itemsSnapshot.find((i) => i.kind === 'raw_app' && i.path === it.path)
						if (hubId && src?.published && src?.publicUrl) {
							try {
								await this.#pushRawAppEmbed(hubId, src.publicUrl)
							} catch (e: any) {
								sendUserToast(`Failed to sync iframe for ${it.path}: ${e?.message ?? e}`, true)
							}
						}
					})
			)
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

			await sleep(150)
			if (this.#disposed) return
			this.deploymentStatus = {}
			this.recordings = {}
			// Deterministic baseline so a transient Hub read failure can't leave the
			// UI stuck in `predeploy`; rehydrate then upgrades to authoritative state.
			this.draftItems = itemsSnapshot.map((i) => ({ ...i, rec: 'none' }))
			this.phase = 'draft'
			await this.rehydrateFromHub()
			if (failures > 0) {
				sendUserToast(`Draft pushed with ${failures} failed item(s).`, true)
			} else {
				sendUserToast(`Draft created on the Hub. Add recordings before submitting for review.`)
			}
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

	startNewDraft = () => {
		this.draftItems = []
		this.recordings = {}
		this.phase = 'predeploy'
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
		const s = await ScriptService.getScriptByPath({ workspace, path: it.path })
		const job = await JobService.getCompletedJob({ workspace, id: jobId })
		const initial_job = { ...(job as any), type: 'CompletedJob' }
		const events = [{ t: 0, data: { completed: true, job: initial_job } }]
		const duration = (initial_job.duration_ms as number) ?? 0
		return {
			version: 1,
			type: 'script' as const,
			recorded_at: new Date().toISOString(),
			script_path: it.path,
			total_duration_ms: duration,
			code: s.content,
			language: s.language,
			args: (job.args ?? {}) as Record<string, any>,
			schema: s.schema,
			job: { initial_job, events }
		}
	}

	async #buildFlowRecording(it: DeployItem, jobId: string) {
		const workspace = this.workspace
		const f = await FlowService.getFlowByPath({ workspace, path: it.path })
		const root = (await JobService.getCompletedJob({ workspace, id: jobId })) as any
		const jobs: Record<string, { initial_job: any; events: any[] }> = {}
		const collect = async (j: any) => {
			const stamped = { ...j, type: 'CompletedJob' }
			jobs[j.id] = {
				initial_job: stamped,
				events: [{ t: 0, data: { completed: true, job: stamped } }]
			}
			const modules = (j.flow_status?.modules ?? []).filter(
				(m: any) => m.job && typeof m.job === 'string'
			)
			// Sub-jobs at the same level are independent reads.
			await Promise.all(
				modules.map(async (m: any) => {
					try {
						const sub = (await JobService.getCompletedJob({ workspace, id: m.job })) as any
						await collect(sub)
					} catch {
						/* sub-job missing — skip */
					}
				})
			)
		}
		await collect(root)
		return {
			version: 1,
			recorded_at: new Date().toISOString(),
			flow_path: it.path,
			total_duration_ms: (root.duration_ms as number) ?? 0,
			flow: {
				path: it.path,
				value: f.value,
				schema: f.schema ?? { type: 'object', properties: {}, required: [] },
				summary: f.summary ?? '',
				archived: false,
				edited_at: '',
				edited_by: '',
				extra_perms: {}
			},
			jobs
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

	// Set the Hub raw app's live-iframe URL (or clear it with null). The Hub renders
	// from external_embed_url; project_slug scopes ownership.
	async #pushRawAppEmbed(hubId: number, url: string | null) {
		await this.#postHub(`/hub/raw_apps/${hubId}/embed`, {
			external_embed_url: url,
			project_slug: this.hubSlug
		})
	}

	// Flip an app/raw app between public (anonymous) and private (publisher) and keep
	// the Hub raw-app iframe in sync. Returns the resolved public URL when shared.
	async #setAppShared(it: DeployItem, shared: boolean): Promise<string | null> {
		const workspace = this.workspace
		const app = await AppService.getAppByPath({ workspace, path: it.path })
		const policy = {
			...(app.policy ?? {}),
			execution_mode: (shared ? 'anonymous' : 'publisher') as 'anonymous' | 'publisher'
		}
		await AppService.updateApp({
			workspace,
			path: it.path,
			requestBody: { policy, deployment_message: shared ? 'Share as iframe' : 'Unshare iframe' }
		})
		const url = shared ? ((await this.#resolvePublicUrl(it.path)) ?? null) : null
		if (it.kind === 'raw_app') {
			const hubId = this.hubItemIds[it.key]
			if (!hubId) {
				if (shared) sendUserToast('Push the bundle to the Hub first to share the live iframe', true)
			} else if (!shared) {
				await this.#pushRawAppEmbed(hubId, null)
			} else if (url) {
				await this.#pushRawAppEmbed(hubId, url)
			}
		}
		return url
	}

	/** Make the publish target public. Returns true on success. */
	async confirmPublish(): Promise<boolean> {
		const it = this.publishTarget
		if (!it || !canPublishApp(it.kind)) return false
		this.publishing = true
		try {
			const url = await this.#setAppShared(it, true)
			this.#patchItem(it.key, { published: true, publicUrl: url ?? undefined })
			sendUserToast(`${it.path} is now public`)
			return true
		} catch (e: any) {
			sendUserToast(`Failed to publish: ${e?.message ?? e}`, true)
			return false
		} finally {
			this.publishing = false
		}
	}

	unpublishApp = async (it: DeployItem) => {
		if (!canPublishApp(it.kind)) return
		try {
			await this.#setAppShared(it, false)
			this.#patchItem(it.key, { published: false, publicUrl: undefined })
			sendUserToast('App unpublished')
		} catch (e: any) {
			sendUserToast(`Failed to unpublish: ${e?.message ?? e}`, true)
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
