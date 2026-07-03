<script lang="ts">
	import WorkspaceDiffDrawer, { type DiffRow } from './WorkspaceDiffDrawer.svelte'
	import Badge from '$lib/components/common/badge/Badge.svelte'
	import { AlertTriangle, ArrowRight, GitFork } from 'lucide-svelte'
	import { WorkspaceService, type WorkspaceComparison, type WorkspaceItemDiff } from '$lib/gen'
	import { getItemValue } from '$lib/utils_workspace_deploy'
	import { userWorkspaces } from '$lib/stores'
	import { editUrlFor as buildEditUrl } from './forkEditUrl'

	// Thin wrapper: supplies the deployed-parent ↔ deployed-fork data source to
	// the generic WorkspaceDiffDrawer. Display/behavior unchanged from before.
	let {
		forkWorkspaceId,
		parentWorkspaceId
	}: { forkWorkspaceId: string; parentWorkspaceId: string } = $props()

	let inner: WorkspaceDiffDrawer | undefined = $state(undefined)
	let comparison: WorkspaceComparison | undefined = $state(undefined)
	let loading = $state(false)
	let error: string | undefined = $state(undefined)

	const forkWs = $derived($userWorkspaces.find((w) => w.id === forkWorkspaceId))
	const parentWs = $derived($userWorkspaces.find((w) => w.id === parentWorkspaceId))

	function statusOf(d: WorkspaceItemDiff): DiffRow['status'] {
		if (d.exists_in_fork && !d.exists_in_source) return 'added'
		if (!d.exists_in_fork && d.exists_in_source) return 'removed'
		if (d.ahead > 0 && d.behind > 0) return 'conflict'
		return 'modified'
	}

	// Keep the original diffs by key so loadValues can honor exists flags
	// (skip the non-existent side for added/removed items).
	let diffByKey: Record<string, WorkspaceItemDiff> = $state({})

	const rows = $derived.by<DiffRow[]>(() =>
		(comparison?.diffs ?? []).map((d) => ({
			kind: d.kind,
			path: d.path,
			status: statusOf(d),
			ahead: d.ahead,
			behind: d.behind
		}))
	)

	function skipNotice(c: WorkspaceComparison | undefined): string | undefined {
		return c?.skipped_comparison
			? 'This fork was created before change tracking was added — diffs are not available.'
			: undefined
	}
	const notice = $derived(skipNotice(comparison))

	export function open() {
		void fetchComparison()
		inner?.open()
	}

	async function fetchComparison() {
		loading = true
		error = undefined
		try {
			comparison = await WorkspaceService.compareWorkspaces({
				workspace: parentWorkspaceId,
				targetWorkspaceId: forkWorkspaceId
			})
			diffByKey = Object.fromEntries(
				(comparison?.diffs ?? []).map((d) => [`${d.kind}/${d.path}`, d])
			)
		} catch (e) {
			console.error('Fork diff: comparison failed', e)
			error = `Failed to load comparison: ${e}`
			comparison = undefined
		} finally {
			loading = false
		}
	}

	async function loadValues(d: DiffRow): Promise<{ before: unknown; after: unknown }> {
		const od = diffByKey[`${d.kind}/${d.path}`]
		const existsSource = od ? od.exists_in_source !== false : true
		const existsFork = od ? od.exists_in_fork !== false : true
		const [before, after] = await Promise.all([
			existsSource
				? getItemValue(d.kind as any, d.path, parentWorkspaceId).catch(() => undefined)
				: Promise.resolve(undefined),
			existsFork
				? getItemValue(d.kind as any, d.path, forkWorkspaceId).catch(() => undefined)
				: Promise.resolve(undefined)
		])
		return { before, after }
	}
</script>

<WorkspaceDiffDrawer
	bind:this={inner}
	diffs={rows}
	{loadValues}
	{loading}
	{error}
	{notice}
	emptyMessage="No changes between this fork and its parent."
	title="Fork changes"
	reviewHref={`/forks/compare?workspace_id=${encodeURIComponent(forkWorkspaceId)}`}
	editUrlFor={(d) => buildEditUrl(d as unknown as WorkspaceItemDiff, forkWorkspaceId)}
>
	{#snippet titleExtra()}
		<div class="flex items-center gap-2 text-xs text-secondary">
			<GitFork class="w-3.5 h-3.5 shrink-0" />
			<span class="font-medium truncate">{forkWs?.name ?? forkWorkspaceId}</span>
			<ArrowRight class="w-3 h-3 shrink-0 text-tertiary" />
			<span class="font-medium truncate">{parentWs?.name ?? parentWorkspaceId}</span>
			{#if comparison}
				<Badge color="transparent" class="ml-2">
					{comparison.summary.total_diffs} item{comparison.summary.total_diffs !== 1 ? 's' : ''}
				</Badge>
				{#if comparison.summary.conflicts > 0}
					<Badge color="orange">
						<AlertTriangle class="w-3 h-3 inline mr-1" />
						{comparison.summary.conflicts} conflict{comparison.summary.conflicts !== 1 ? 's' : ''}
					</Badge>
				{/if}
			{/if}
		</div>
	{/snippet}
</WorkspaceDiffDrawer>
