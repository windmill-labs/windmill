<script lang="ts">
	import WorkspaceDiffDrawer, { type DiffRow } from './WorkspaceDiffDrawer.svelte'
	import { Pencil } from 'lucide-svelte'
	import { type WorkspaceItemDiff } from '$lib/gen'
	import { userWorkspaces } from '$lib/stores'
	import { editUrlFor as buildEditUrl } from './forkEditUrl'
	import { getDraftDiffValues, type DraftKind } from '$lib/utils_draft_deploy'
	import { getDraftItems } from '$lib/workspaceDrafts.svelte'

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
				const kind = it.raw_app ? 'raw_app' : it.kind
				donly[`${kind}/${it.path}`] = it.draft_only
				return { kind, path: it.path, status: it.draft_only ? 'added' : 'modified' }
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
		// getDraftDiffValues works on the draft_type kind ('app' for raw apps too).
		const kind: DraftKind = d.kind === 'raw_app' ? 'app' : (d.kind as DraftKind)
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
