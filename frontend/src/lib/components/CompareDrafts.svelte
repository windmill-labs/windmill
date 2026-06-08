<script lang="ts">
	import WorkspaceDeployLayout from './WorkspaceDeployLayout.svelte'
	import DiffDrawer from './DiffDrawer.svelte'
	import WorkspaceDeployItemSummary from './WorkspaceDeployItemSummary.svelte'
	import { Badge } from './common'
	import Button from './common/button/Button.svelte'
	import ConfirmationModal from './common/confirmationModal/ConfirmationModal.svelte'
	import { ArrowRight, DiffIcon, GitFork, Pencil, Undo2 } from 'lucide-svelte'
	import { untrack } from 'svelte'
	import CompareModeToggle, { type CompareMode } from './CompareModeToggle.svelte'
	import { editUrlFor } from './sessions/forkEditUrl'
	import { AppService, FlowService, ScriptService, type WorkspaceItemDiff } from '$lib/gen'
	import { sendUserToast } from '$lib/toast'
	import { getDraftDiffValues, deployDraft, discardDraft } from '$lib/utils_draft_deploy'
	import { type DraftItem } from '$lib/workspaceDrafts.svelte'

	interface Props {
		currentWorkspaceId: string
		/** The Draft Items to review, owned by the page's Workspace Drafts resource
		 * and passed down so we don't mount a second resource (which would double
		 * the list fetches). Deploy/discard here invalidate that resource, so the
		 * list refetches upstream and the new `draftItems` flow back in. */
		draftItems: DraftItem[]
		/** True while the page's Workspace Drafts resource is loading. */
		draftsLoading?: boolean
		/** Fork context drives the merged toggle: only a fork offers the
		 * deploy_to/update directions, so the toggle is hidden otherwise. */
		isFork?: boolean
		parentWorkspaceId?: string
		deployCount?: number
		updateCount?: number
		draftCount?: number
		/** Selecting deploy_to/update asks the page to swap to CompareWorkspaces. */
		onModeSelected?: (v: CompareMode) => void
		/** Fired after a deploy/discard so the page can refresh the *fork*
		 * comparison (ahead/behind). The Draft Count refreshes itself — deploy/
		 * discard invalidate the Workspace Drafts resource. */
		onChanged?: () => void
	}

	let {
		currentWorkspaceId,
		draftItems,
		draftsLoading = false,
		isFork = false,
		parentWorkspaceId,
		deployCount = 0,
		updateCount = 0,
		draftCount = 0,
		onModeSelected,
		onChanged
	}: Props = $props()

	type Row = {
		kind: DraftItem['kind']
		path: string
		summary?: string
		draft_only: boolean
		raw_app: boolean
		key: string
	}
	function getItemKey(kind: string, path: string): string {
		return `${kind}:${path}`
	}

	// The list (and the Draft Count) come from the shared Workspace Drafts module,
	// owned by the page and passed in via `draftItems`; deploy/discard invalidate
	// that resource, so the list refetches and deployed items drop off without a
	// manual reload here.
	const items: Row[] = $derived(draftItems.map((d) => ({ ...d, key: getItemKey(d.kind, d.path) })))

	// The Draft Items list only carries the *deployed* summary, so the draft's
	// (new) display name isn't known yet. Fetch each item's draft blob once and
	// cache both names — mirrors CompareWorkspaces' fetchSummaries (eager on load,
	// keyed by row key) so the rename rendering is shared and consistent. Only
	// non-`draft_only` items can show a rename: a `draft_only` item has no deployed
	// side to diff the name against. Raw apps live on a separate route and aren't
	// fetchable here, so they're skipped (no rename shown, same as before).
	const summaryCache = $state<
		Record<string, { deployed?: string; draft?: string; loading?: boolean }>
	>({})

	async function fetchDraftSummary(item: Row) {
		if (summaryCache[item.key]) return
		summaryCache[item.key] = { loading: true }
		try {
			const r = (await (item.kind === 'script'
				? ScriptService.getScriptByPathWithDraft({ workspace: currentWorkspaceId, path: item.path })
				: item.kind === 'flow'
					? FlowService.getFlowByPathWithDraft({ workspace: currentWorkspaceId, path: item.path })
					: AppService.getAppByPathWithDraft({
							workspace: currentWorkspaceId,
							path: item.path
						}))) as any
			summaryCache[item.key] = {
				deployed: r.summary,
				draft: (r.draft as any)?.summary,
				loading: false
			}
		} catch (error) {
			console.error(`Failed to fetch draft summary for ${item.kind}:${item.path}`, error)
			summaryCache[item.key] = { loading: false }
		}
	}

	$effect(() => {
		const current = items
		untrack(() => {
			for (const item of current) {
				if (!item.draft_only && !item.raw_app && !summaryCache[item.key]) {
					void fetchDraftSummary(item)
				}
			}
		})
	})

	let selectedItems = $state<string[]>([])
	let deploying = $state(false)
	// Select all on the first non-empty load (deploy-all is the common intent);
	// only once, so a refetch after a deploy doesn't re-select the leftovers.
	let hasAutoSelected = $state(false)

	const deploymentStatus: Record<
		string,
		{ status: 'loading' | 'deployed' | 'failed'; error?: string }
	> = $state({})

	// Prune transient deploy status for items no longer in the list (a deployed
	// item drops off after the resource refetches). Keeps the map from growing
	// unbounded and avoids a stale 'deployed' entry suppressing a row if the same
	// kind:path is re-drafted within this mount.
	$effect(() => {
		const live = new Set(items.map((i) => i.key))
		untrack(() => {
			for (const key of Object.keys(deploymentStatus)) {
				if (!live.has(key)) delete deploymentStatus[key]
			}
		})
	})

	$effect(() => {
		if (!hasAutoSelected && items.length > 0) {
			selectedItems = items
				.filter((i) => deploymentStatus[i.key]?.status !== 'deployed')
				.map((i) => i.key)
			hasAutoSelected = true
		}
	})

	// Selected items still in the live list and deployable. Derived (not a pruning
	// effect) so the "Deploy N drafts" button stays reactive to the Workspace
	// Drafts resource: deploy/discard drop items, and stale keys left in
	// selectedItems are simply ignored here (and by deploySelected).
	let selectedCount = $derived(
		items.filter(
			(i) => selectedItems.includes(i.key) && deploymentStatus[i.key]?.status !== 'deployed'
		).length
	)

	let allSelected = $derived(
		items.length > 0 &&
			items
				.filter((i) => deploymentStatus[i.key]?.status !== 'deployed')
				.every((i) => selectedItems.includes(i.key))
	)

	function toggleItem(item: { key: string }) {
		if (selectedItems.includes(item.key)) {
			selectedItems = selectedItems.filter((k) => k !== item.key)
		} else {
			selectedItems = [...selectedItems, item.key]
		}
	}

	function selectAll() {
		selectedItems = items
			.filter((i) => deploymentStatus[i.key]?.status !== 'deployed')
			.map((i) => i.key)
	}

	function deselectAll() {
		selectedItems = []
	}

	// --- Diff ---
	let diffDrawer: DiffDrawer | undefined = $state(undefined)
	let isFlow = $state(false)
	// Monotonic token so that two quick "Show diff" clicks don't race: a slower
	// earlier fetch must not overwrite a faster later one in the (single) drawer.
	let diffRequestId = 0

	async function showDiff(item: Row) {
		if (!diffDrawer) return
		const reqId = ++diffRequestId
		isFlow = item.kind === 'flow'
		diffDrawer.openDrawer()
		const { deployed, draft } = await getDraftDiffValues(
			item.kind,
			item.path,
			currentWorkspaceId,
			item.draft_only
		)
		// A newer Show-diff click superseded this one — drop the stale result.
		if (reqId !== diffRequestId) return
		diffDrawer.setDiff({
			mode: 'simple',
			original: deployed as any,
			current: draft as any,
			title: 'Deployed → Draft'
		})
	}

	// --- Deploy ---
	async function deploySelected() {
		deploying = true
		// Snapshot the items to deploy: deployDraft invalidates the Workspace Drafts
		// resource, so `items` can change mid-loop — iterate a stable copy.
		const toDeploy = items.filter((i) => selectedItems.includes(i.key))
		let deployedAny = false
		for (const item of toDeploy) {
			deploymentStatus[item.key] = { status: 'loading' }
			const res = await deployDraft(
				item.kind,
				item.path,
				currentWorkspaceId,
				item.draft_only,
				item.raw_app
			)
			if (res.success) {
				deploymentStatus[item.key] = { status: 'deployed' }
				deployedAny = true
			} else {
				deploymentStatus[item.key] = { status: 'failed', error: res.error }
				sendUserToast(`Failed to deploy ${item.path}: ${res.error}`, true)
			}
		}
		deploying = false
		selectedItems = []
		// The Draft list refetches itself (deployDraft invalidated it). Deploying
		// also changes the fork comparison (ahead/behind) — ask the page to refresh
		// that.
		if (deployedAny) onChanged?.()
	}

	// --- Discard ---
	let discardTarget = $state<Row | undefined>(undefined)

	async function confirmDiscard() {
		const item = discardTarget
		discardTarget = undefined
		if (!item) return
		const res = await discardDraft(item.kind, item.path, currentWorkspaceId, item.draft_only)
		if (res.success) {
			sendUserToast(item.draft_only ? `Deleted ${item.path}` : `Discarded draft of ${item.path}`)
			// discardDraft invalidated the Draft list; refresh the fork comparison.
			onChanged?.()
		} else {
			sendUserToast(`Failed to discard ${item.path}: ${res.error}`, true)
		}
	}

	// Editor URL for a draft item, scoped to the current workspace. Raw apps live
	// under a different editor route, so map their kind accordingly.
	function draftEditUrl(d: Row): string | undefined {
		return editUrlFor(
			{ kind: d.raw_app ? 'raw_app' : d.kind, path: d.path } as unknown as WorkspaceItemDiff,
			currentWorkspaceId
		)
	}
</script>

<div class="flex flex-col gap-4">
	<div class="bg-surface-tertiary p-4 rounded-md border">
		<WorkspaceDeployLayout
			{items}
			{selectedItems}
			{deploymentStatus}
			{allSelected}
			selectablePredicate={(item) => deploymentStatus[item.key]?.status !== 'deployed'}
			onToggleItem={toggleItem}
			onSelectAll={selectAll}
			onDeselectAll={deselectAll}
			emptyMessage={draftsLoading ? 'Loading drafts…' : 'No drafts in this workspace'}
		>
			{#snippet header()}
				{#if isFork}
					<div class="flex flex-wrap gap-1 items-center bg-surface-tertiary pb-4">
						<CompareModeToggle
							selected="draft"
							{isFork}
							{parentWorkspaceId}
							{deployCount}
							{updateCount}
							{draftCount}
							disabled={deploying}
							onSelected={(v) => onModeSelected?.(v)}
						/>
						<!-- Direction badge, mirroring the fork compare header: make it explicit
						     that deploying a draft promotes it *within this fork* (deployed↔draft),
						     not up to the parent. -->
						<div class="flex-1 flex gap-1 items-center">
							<Badge color="transparent" class="ml-5 font-semibold">
								<span class="text-secondary">deploy:</span>
								<Pencil size={14} />
								<span class="text-emphasis">draft</span>
							</Badge>
							<ArrowRight size={16} />
							<Badge color="transparent" class="font-semibold" title={currentWorkspaceId}>
								<span class="text-secondary">into:</span>
								<GitFork size={14} />
								<span class="text-emphasis">{currentWorkspaceId}</span>
							</Badge>
						</div>
					</div>
				{/if}
			{/snippet}

			{#snippet itemSummary(item)}
				{@const draftItem = item as unknown as Row}
				{@const editUrl = draftEditUrl(draftItem)}
				{@const cache = summaryCache[draftItem.key]}
				{@const oldSummary = cache?.deployed ?? draftItem.summary}
				{@const newSummary = cache?.draft ?? draftItem.summary}
				<WorkspaceDeployItemSummary
					path={draftItem.path}
					{editUrl}
					{oldSummary}
					{newSummary}
					renamed={!draftItem.draft_only &&
						oldSummary != null &&
						newSummary != null &&
						oldSummary !== newSummary}
				/>
			{/snippet}

			{#snippet itemActions(item)}
				{@const draftItem = item as unknown as Row}
				{#if draftItem.draft_only}
					<Badge color="indigo" size="xs">New</Badge>
				{/if}
				{#if deploymentStatus[draftItem.key]?.status !== 'deployed'}
					<Button
						unifiedSize="xs"
						variant="subtle"
						startIcon={{ icon: DiffIcon }}
						onClick={() => showDiff(draftItem)}
					>
						Show diff
					</Button>
					<Button
						unifiedSize="xs"
						variant="subtle"
						destructive
						startIcon={{ icon: Undo2 }}
						onClick={() => (discardTarget = draftItem)}
					>
						Discard draft
					</Button>
				{/if}
			{/snippet}

			{#snippet footer()}
				<div class="flex items-center justify-end">
					<Button
						variant="accent"
						disabled={selectedCount === 0 || deploying}
						loading={deploying}
						onClick={deploySelected}
					>
						Deploy {selectedCount} draft{selectedCount !== 1 ? 's' : ''}
					</Button>
				</div>
			{/snippet}
		</WorkspaceDeployLayout>
	</div>

	<DiffDrawer bind:this={diffDrawer} {isFlow} />
</div>

<ConfirmationModal
	open={discardTarget !== undefined}
	title={discardTarget?.draft_only ? 'Delete item' : 'Discard draft'}
	confirmationText={discardTarget?.draft_only ? 'Delete' : 'Discard'}
	onConfirmed={confirmDiscard}
	onCanceled={() => (discardTarget = undefined)}
>
	{#if discardTarget?.draft_only}
		<p>
			<span class="font-mono font-medium text-primary">{discardTarget?.path}</span> exists only as a
			draft. Discarding it will permanently delete the item. This cannot be undone.
		</p>
	{:else}
		<p>
			Discard the draft of
			<span class="font-mono font-medium text-primary">{discardTarget?.path}</span>? The deployed
			version is unaffected.
		</p>
	{/if}
</ConfirmationModal>
