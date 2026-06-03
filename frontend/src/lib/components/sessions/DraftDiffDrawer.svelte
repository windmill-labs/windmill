<script lang="ts">
	import WorkspaceDiffDrawer, { type DiffRow } from './WorkspaceDiffDrawer.svelte'
	import { Pencil } from 'lucide-svelte'
	import { ScriptService, FlowService, AppService, type WorkspaceItemDiff } from '$lib/gen'
	import { userWorkspaces } from '$lib/stores'
	import { editUrlFor as buildEditUrl } from './forkEditUrl'
	import { getDraftDiffValues, type DraftKind } from '$lib/utils_draft_deploy'

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
			const [scripts, flows, apps] = await Promise.all([
				ScriptService.listScripts({ workspace: workspaceId, includeDraftOnly: true }),
				FlowService.listFlows({ workspace: workspaceId, includeDraftOnly: true }),
				AppService.listApps({ workspace: workspaceId, includeDraftOnly: true })
			])
			const collected: DiffRow[] = []
			const donly: Record<string, boolean> = {}
			const push = (kind: DraftKind, list: Array<any>) => {
				for (const it of list) {
					if (it.has_draft || it.draft_only) {
						const status: DiffRow['status'] = it.draft_only ? 'added' : 'modified'
						collected.push({ kind, path: it.path, status })
						donly[`${kind}/${it.path}`] = !!it.draft_only
					}
				}
			}
			push('script', scripts)
			push('flow', flows)
			push('app', apps)
			collected.sort((a, b) => a.path.localeCompare(b.path))
			rows = collected
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
		const { deployed, draft } = await getDraftDiffValues(
			d.kind as DraftKind,
			d.path,
			workspaceId,
			draftOnly
		)
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
