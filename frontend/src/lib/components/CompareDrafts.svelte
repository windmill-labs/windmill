<script lang="ts">
	import { ScriptService, FlowService, AppService } from '$lib/gen'
	import WorkspaceDeployLayout from './WorkspaceDeployLayout.svelte'
	import DiffDrawer from './DiffDrawer.svelte'
	import { Alert, Badge } from './common'
	import Button from './common/button/Button.svelte'
	import ConfirmationModal from './common/confirmationModal/ConfirmationModal.svelte'
	import { DiffIcon, Undo2 } from 'lucide-svelte'
	import CompareModeToggle, { type CompareMode } from './CompareModeToggle.svelte'
	import { untrack } from 'svelte'
	import { sendUserToast } from '$lib/toast'
	import {
		getDraftDiffValues,
		deployDraft,
		discardDraft,
		type DraftKind
	} from '$lib/utils_draft_deploy'

	interface Props {
		currentWorkspaceId: string
		/** Reports the current number of drafts after each (re)load, so the page
		 * can keep the "Draft (N)" toggle badge in sync without a separate call. */
		onCountChange?: (count: number) => void
		/** Fork context drives the merged toggle: only a fork offers the
		 * deploy_to/update directions, so the toggle is hidden otherwise. */
		isFork?: boolean
		parentWorkspaceId?: string
		draftCount?: number
		/** Selecting deploy_to/update asks the page to swap to CompareWorkspaces. */
		onModeSelected?: (v: CompareMode) => void
	}

	let {
		currentWorkspaceId,
		onCountChange,
		isFork = false,
		parentWorkspaceId,
		draftCount = 0,
		onModeSelected
	}: Props = $props()

	type DraftItem = {
		key: string
		path: string
		kind: DraftKind
		summary?: string
		draft_only: boolean
		raw_app: boolean
	}

	let items = $state<DraftItem[]>([])
	let loading = $state(true)
	let selectedItems = $state<string[]>([])
	let deploying = $state(false)

	const deploymentStatus: Record<
		string,
		{ status: 'loading' | 'deployed' | 'failed'; error?: string }
	> = $state({})

	function getItemKey(kind: string, path: string): string {
		return `${kind}:${path}`
	}

	async function loadDrafts() {
		loading = true
		try {
			const [scripts, flows, apps] = await Promise.all([
				ScriptService.listScripts({ workspace: currentWorkspaceId, includeDraftOnly: true }),
				FlowService.listFlows({ workspace: currentWorkspaceId, includeDraftOnly: true }),
				AppService.listApps({ workspace: currentWorkspaceId, includeDraftOnly: true })
			])
			const collected: DraftItem[] = []
			const push = (kind: DraftKind, list: Array<any>) => {
				for (const it of list) {
					if (it.has_draft || it.draft_only) {
						collected.push({
							key: getItemKey(kind, it.path),
							path: it.path,
							kind,
							summary: it.summary,
							draft_only: !!it.draft_only,
							raw_app: !!it.raw_app
						})
					}
				}
			}
			push('script', scripts)
			push('flow', flows)
			push('app', apps)
			collected.sort((a, b) => a.path.localeCompare(b.path))
			items = collected
			onCountChange?.(items.length)
		} catch (e) {
			console.error('Failed to load drafts', e)
			sendUserToast(`Failed to load drafts: ${e}`, true)
		} finally {
			loading = false
		}
	}

	$effect(() => {
		currentWorkspaceId
		untrack(() => loadDrafts())
	})

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

	async function showDiff(item: DraftItem) {
		if (!diffDrawer) return
		isFlow = item.kind === 'flow'
		diffDrawer.openDrawer()
		const { deployed, draft } = await getDraftDiffValues(
			item.kind,
			item.path,
			currentWorkspaceId,
			item.draft_only
		)
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
		for (const key of [...selectedItems]) {
			const item = items.find((i) => i.key === key)
			if (!item) continue
			deploymentStatus[key] = { status: 'loading' }
			const res = await deployDraft(
				item.kind,
				item.path,
				currentWorkspaceId,
				item.draft_only,
				item.raw_app
			)
			if (res.success) {
				deploymentStatus[key] = { status: 'deployed' }
			} else {
				deploymentStatus[key] = { status: 'failed', error: res.error }
				sendUserToast(`Failed to deploy ${item.path}: ${res.error}`, true)
			}
		}
		deploying = false
		selectedItems = []
		// Reload: deployed items had their draft deleted server-side and drop off
		// the list; failed items keep their draft (and red status, preserved since
		// we don't clear deploymentStatus).
		await loadDrafts()
	}

	// --- Discard ---
	let discardTarget = $state<DraftItem | undefined>(undefined)

	async function confirmDiscard() {
		const item = discardTarget
		discardTarget = undefined
		if (!item) return
		const res = await discardDraft(item.kind, item.path, currentWorkspaceId, item.draft_only)
		if (res.success) {
			sendUserToast(item.draft_only ? `Deleted ${item.path}` : `Discarded draft of ${item.path}`)
			await loadDrafts()
		} else {
			sendUserToast(`Failed to discard ${item.path}: ${res.error}`, true)
		}
	}

	const KIND_LABEL: Record<DraftKind, string> = { script: 'Script', flow: 'Flow', app: 'App' }
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
			emptyMessage={loading ? 'Loading drafts…' : 'No drafts in this workspace'}
		>
			{#snippet header()}
				{#if isFork}
					<div class="flex items-center bg-surface-tertiary pb-4 border-b">
						<CompareModeToggle
							selected="draft"
							{isFork}
							{parentWorkspaceId}
							{draftCount}
							disabled={deploying}
							onSelected={(v) => onModeSelected?.(v)}
						/>
					</div>
				{/if}
			{/snippet}

			{#snippet alerts()}
				{#if items.length > 0}
					<Alert title="Pending drafts" type="info" size="xs" class="my-2">
						These are unsaved drafts for the current workspace. Deploying promotes a draft to the
						deployed version (and removes the draft); discarding removes the draft — for items that
						exist only as a draft, discarding deletes the item.
					</Alert>
				{/if}
			{/snippet}

			{#snippet itemSummary(item)}
				{@const draftItem = item as unknown as DraftItem}
				<span class="text-secondary">{KIND_LABEL[draftItem.kind]}</span>
				<span class="text-tertiary mx-1">→</span>
				<span class="text-emphasis">{draftItem.summary || draftItem.path}</span>
			{/snippet}

			{#snippet itemActions(item)}
				{@const draftItem = item as unknown as DraftItem}
				{#if draftItem.draft_only}
					<Badge color="indigo" size="xs">New</Badge>
				{/if}
				{#if deploymentStatus[draftItem.key]?.status !== 'deployed'}
					<Button unifiedSize="xs" variant="subtle" onClick={() => showDiff(draftItem)}>
						<DiffIcon class="w-3 h-3 mr-1" /> Show diff
					</Button>
					<Button
						unifiedSize="xs"
						variant="subtle"
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
						disabled={selectedItems.length === 0 || deploying}
						loading={deploying}
						onClick={deploySelected}
					>
						Deploy {selectedItems.length} draft{selectedItems.length !== 1 ? 's' : ''}
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
