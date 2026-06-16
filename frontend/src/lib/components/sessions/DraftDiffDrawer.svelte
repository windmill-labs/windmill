<script lang="ts">
	import WorkspaceDiffDrawer, { type DiffRow } from './WorkspaceDiffDrawer.svelte'
	import { Pencil } from 'lucide-svelte'
	import { type WorkspaceItemDiff } from '$lib/gen'
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

	const ws = $derived($userWorkspaces.find((w) => w.id === workspaceId))

	export function open() {
		void fetchDrafts()
		inner?.open()
	}

	async function fetchDrafts() {
		loading = true
		error = undefined
		try {
			// One source of truth (Workspace Drafts) — same list the compare page and
			// the count use.
			const items = await getDraftItems(workspaceId)
			const donly: Record<string, boolean> = {}
			rows = items.map((it) => {
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
				return {
					kind,
					path: it.path,
					displayPath: it.draft_path ?? it.path,
					summary: it.summary,
					status: it.draft_only ? 'added' : 'modified'
				}
			})
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
