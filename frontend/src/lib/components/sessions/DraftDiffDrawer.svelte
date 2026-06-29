<script lang="ts">
	import WorkspaceDiffDrawer, { type DiffRow } from './WorkspaceDiffDrawer.svelte'
	import { Pencil } from 'lucide-svelte'
	import { DraftService, ScriptService, type WorkspaceItemDiff } from '$lib/gen'
	import { userWorkspaces } from '$lib/stores'
	import { editUrlFor as buildEditUrl } from './forkEditUrl'
	import { getDraftDiffValues, type DraftKind } from '$lib/utils_draft_deploy'
	import { getDraftItems } from '$lib/workspaceDrafts.svelte'

	// Draft rows carry the user-draft itemKind (`trigger_schedule`, `trigger_http`…),
	// but the shared row icon/label and the edit-link builder speak the deploy-style
	// kinds (`schedule`, `http_trigger`…) the fork drawer and compare page use. Map
	// to deploy-style for display, and back for the draft-value getter.
	const DEPLOY_KIND_BY_DRAFT_KIND: Partial<Record<DraftKind, string>> = {
		trigger_schedule: 'schedule',
		trigger_http: 'http_trigger',
		trigger_websocket: 'websocket_trigger',
		trigger_kafka: 'kafka_trigger',
		trigger_nats: 'nats_trigger',
		trigger_postgres: 'postgres_trigger',
		trigger_mqtt: 'mqtt_trigger',
		trigger_sqs: 'sqs_trigger',
		trigger_gcp: 'gcp_trigger',
		trigger_azure: 'azure_trigger',
		trigger_email: 'email_trigger'
	}
	const DRAFT_KIND_BY_DEPLOY_KIND = Object.fromEntries(
		Object.entries(DEPLOY_KIND_BY_DRAFT_KIND).map(([d, p]) => [p, d])
	) as Record<string, DraftKind>

	// Thin wrapper: supplies the deployed ↔ draft data source (server `draft`
	// table, same as the compare page) to the generic WorkspaceDiffDrawer.
	// Read-only, mirroring ForkDiffDrawer; deploy/discard live on the Review page.
	let { workspaceId }: { workspaceId: string } = $props()

	let inner: WorkspaceDiffDrawer | undefined = $state(undefined)
	let rows: DiffRow[] = $state([])
	let loading = $state(false)
	let error: string | undefined = $state(undefined)
	// draft_only per item — drives the "added" rendering (empty before).
	let draftOnlyByKey: Record<string, boolean> = $state({})
	// Pre-resolved before/after for exploded pipeline-node rows (see below),
	// keyed by `${kind}/${path}`. Their content lives in the bundle, not a
	// per-item draft endpoint, so `loadValues` reads it from here instead of
	// calling `getDraftDiffValues`.
	let pipelineNodeValues: Record<string, { before?: unknown; after: unknown }> = $state({})

	const ws = $derived($userWorkspaces.find((w) => w.id === workspaceId))

	export function open() {
		void fetchDrafts()
		inner?.open()
	}

	type ScriptBody = { language?: string; content?: string }
	type PipelineBundle = {
		drafts?: Array<[string, { script?: { language?: string; content?: string; summary?: string } }]>
	}

	// Explode a `data_pipeline` bundle draft into one row per node, nested under
	// the bundle's folder so they read as the pipeline's subitems. Each node is a
	// script with its own draft body; the deployed body (if the node is already
	// deployed) is the "before" so its line changes show. Returns undefined to let
	// the caller fall back to a single bundle row if the bundle can't be read.
	async function explodePipelineBundle(bundlePath: string): Promise<DiffRow[] | undefined> {
		let bundle: PipelineBundle
		try {
			const row = (await DraftService.getOwnDraft({
				workspace: workspaceId,
				kind: 'data_pipeline',
				path: bundlePath
			})) as { value?: PipelineBundle }
			bundle = row?.value ?? {}
		} catch (e) {
			console.warn('Draft diff: pipeline bundle load failed', bundlePath, e)
			return undefined
		}
		const entries = Array.isArray(bundle.drafts) ? bundle.drafts : []
		// The bundle is keyed at `f/<folder>/data_pipeline`; nodes live in `f/<folder>`.
		const folder = bundlePath.replace(/\/data_pipeline$/, '')
		const out: DiffRow[] = []
		await Promise.all(
			entries.map(async ([nodePath, d]) => {
				const after: ScriptBody = {
					language: d?.script?.language,
					content: d?.script?.content ?? ''
				}
				let before: ScriptBody | undefined
				try {
					const dep = await ScriptService.getScriptByPath({
						workspace: workspaceId,
						path: nodePath
					})
					before = { language: dep.language, content: dep.content }
				} catch {
					// Node not deployed yet → no "before" (renders as added).
				}
				const rel = nodePath.startsWith(`${folder}/`) ? nodePath.slice(folder.length + 1) : nodePath
				// Nest under the bundle folder (`…/data_pipeline/<node>`) so the nodes
				// group as the pipeline's subitems in the tree.
				const displayPath = `${bundlePath}/${rel}`
				// A pipeline node shares `script/<nodePath>` with a standalone script
				// draft at the same path, so identify the row by its distinct
				// bundle-nested path — `path` stays the real node for the edit link.
				const key = `pipeline:${displayPath}`
				pipelineNodeValues[key] = { before, after }
				out.push({
					kind: 'script',
					path: nodePath,
					displayPath,
					key,
					summary: d?.script?.summary || undefined,
					status: before ? 'modified' : 'added'
				})
			})
		)
		return out
	}

	async function fetchDrafts() {
		loading = true
		error = undefined
		try {
			// One source of truth (Workspace Drafts) — same list the compare page and
			// the count use.
			const items = await getDraftItems(workspaceId)
			const donly: Record<string, boolean> = {}
			pipelineNodeValues = {}
			const out: DiffRow[] = []
			for (const it of items) {
				if (it.kind === 'data_pipeline') {
					// A pipeline bundle isn't a single diffable item — explode it into its
					// node-script subitems. An unreadable bundle is skipped (it has no
					// single-item diff to show).
					const nodes = await explodePipelineBundle(it.path)
					if (nodes) out.push(...nodes)
					continue
				}
				// Raw apps must surface as `raw_app` so the row's edit link points at the
				// raw-app editor (mirrors CompareDrafts); `getDraftItems` carries the flag.
				const baseKind = it.raw_app ? 'raw_app' : it.kind
				const kind = DEPLOY_KIND_BY_DRAFT_KIND[baseKind] ?? baseKind
				donly[`${kind}/${it.path}`] = it.draft_only
				// A never-deployed app/raw_app is parked at a synthetic `…/draft_<uuid>`
				// storage path with the user's typed name in `draft_path`; show that
				// (matches the home list) while `path` stays the storage key for loading.
				// `summary` comes straight from the draft row, so it shows for every kind
				// up front instead of only after the diff value loads.
				out.push({
					kind,
					path: it.path,
					displayPath: it.draft_path ?? it.path,
					summary: it.summary,
					status: it.draft_only ? 'added' : 'modified'
				})
			}
			rows = out
			draftOnlyByKey = donly
		} catch (e) {
			console.error('Draft diff: list failed', e)
			error = `Failed to load drafts: ${e}`
			rows = []
		} finally {
			loading = false
		}
	}

	async function loadValues(d: DiffRow): Promise<{ before: unknown; after: unknown }> {
		// Exploded pipeline-node rows carry their content from the bundle, not a
		// per-item draft endpoint — keyed by the row's unique `key` so a standalone
		// script draft at the same path can't read a node's cached value.
		const pipelineNode = d.key ? pipelineNodeValues[d.key] : undefined
		if (pipelineNode) return { before: pipelineNode.before, after: pipelineNode.after }
		const draftOnly = draftOnlyByKey[`${d.kind}/${d.path}`] ?? false
		// getDraftDiffValues keys on the draft itemKind: `raw_app` must stay
		// `raw_app` (the helper sends rawApp:true only for that exact kind, which a
		// never-deployed raw app needs, else it hits the normal app endpoint and
		// 404s). Only the trigger display kinds map back from their deploy-style names.
		const kind = (DRAFT_KIND_BY_DEPLOY_KIND[d.kind] ?? d.kind) as DraftKind
		const { deployed, draft } = await getDraftDiffValues(kind, d.path, workspaceId, draftOnly)
		// draft_only items have never been deployed → render as "added" (empty
		// before), matching how the fork drawer renders added items.
		return { before: draftOnly ? undefined : deployed, after: draft }
	}
</script>

<WorkspaceDiffDrawer
	bind:this={inner}
	diffs={rows}
	{loadValues}
	{loading}
	{error}
	emptyMessage="No drafts in this workspace."
	title="Drafts"
	reviewHref={`/forks/compare?workspace_id=${encodeURIComponent(workspaceId)}&mode=draft`}
	editUrlFor={(d) => buildEditUrl(d as unknown as WorkspaceItemDiff, workspaceId)}
>
	{#snippet titleExtra()}
		<div class="flex items-center gap-2 text-xs text-secondary">
			<Pencil class="w-3.5 h-3.5 shrink-0" />
			<span class="font-medium truncate">{ws?.name ?? workspaceId}</span>
		</div>
	{/snippet}
</WorkspaceDiffDrawer>
