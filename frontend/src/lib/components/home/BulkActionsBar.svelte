<script lang="ts">
	/**
	 * Contextual bar for the Home page's multi-selection: what can be done to the
	 * current selection, why the rest can't, and the confirmation + per-item
	 * outcome of the run.
	 */
	import { Alert, Button } from '$lib/components/common'
	import ConfirmationModal from '$lib/components/common/confirmationModal/ConfirmationModal.svelte'
	import DropdownV2 from '$lib/components/DropdownV2.svelte'
	import Select from '$lib/components/select/Select.svelte'
	import Label from '$lib/components/Label.svelte'
	import { sendUserToast } from '$lib/toast'
	import { invalidateWorkspaceDrafts } from '$lib/workspaceDrafts.svelte'
	import { Archive, ArchiveRestore, FolderInput, MoreHorizontal, Trash, X } from 'lucide-svelte'
	import {
		blockedReason,
		eligible,
		movedPath,
		runBulk,
		type BulkAction,
		type BulkContext,
		type BulkOutcome
	} from './bulkActions'
	import type { BulkItem, HomeSelection } from './homeSelection.svelte'

	interface Props {
		selection: HomeSelection
		workspace: string
		isAdmin: boolean
		/** Owner prefixes (`f/<folder>` / `u/<user>`) the user may move items into. */
		moveTargets: string[]
		/** Refresh the list and its counts once the batch is done. */
		onDone: () => Promise<void> | void
	}

	let { selection, workspace, isAdmin, moveTargets, onDone }: Props = $props()

	let ctx: BulkContext = $derived({ workspace, isAdmin })
	let items = $derived(selection.items)

	const plural = (n: number) => (n === 1 ? '' : 's')
	const ACTION_LABEL: Record<BulkAction, string> = {
		move: 'Move',
		archive: 'Archive',
		unarchive: 'Unarchive',
		delete: 'Delete',
		discard: 'Discard drafts'
	}
	const MODAL_TITLE: Record<BulkAction, (n: number) => string> = {
		move: (n) => `Move ${n} item${plural(n)}`,
		archive: (n) => `Archive ${n} item${plural(n)}`,
		unarchive: (n) => `Unarchive ${n} item${plural(n)}`,
		delete: (n) => `Delete ${n} item${plural(n)}`,
		discard: (n) => `Discard ${n} draft${plural(n)}`
	}

	function targets(action: BulkAction): BulkItem[] {
		return eligible(action, items, ctx)
	}

	/** Every distinct reason the ineligible rows are out, so both the disabled
	 * action and the modal can say whether the limit comes from permissions, the
	 * item's kind, or its state. Empty when the whole selection is eligible. */
	function blockedSummary(action: BulkAction): string {
		return [
			...new Set(
				items.map((i) => blockedReason(action, i, ctx)).filter((r): r is string => r != undefined)
			)
		].join('; ')
	}

	function actionTitle(action: BulkAction): string {
		const n = targets(action).length
		// Selection mode is entered from the toolbar with nothing picked yet, so this
		// is the state the primary entry point lands on — it has no blocked reason.
		if (items.length === 0) return `Select items to ${ACTION_LABEL[action].toLowerCase()}`
		if (n === 0) return `Cannot ${ACTION_LABEL[action].toLowerCase()}: ${blockedSummary(action)}`
		if (n < items.length) return `${ACTION_LABEL[action]} ${n} of the ${items.length} selected`
		return `${ACTION_LABEL[action]} ${n} item${plural(n)}`
	}

	function countSuffix(action: BulkAction): string {
		const n = targets(action).length
		return n > 0 && n < items.length ? ` (${n})` : ''
	}

	let pending = $state<BulkAction | undefined>(undefined)
	let moveTarget = $state<string | undefined>(undefined)
	let running = $state(false)
	let progress = $state(0)
	let outcomes = $state<BulkOutcome[] | undefined>(undefined)

	let pendingItems = $derived(pending ? targets(pending) : [])
	let failures = $derived(outcomes?.filter((o) => o.error != undefined) ?? [])
	// A discard removes a draft-only item outright but only the draft of a
	// deployed one — the modal has to name which paths go for good.
	let discardDeletes = $derived(pendingItems.filter((i) => i.draftOnly))
	let discardReverts = $derived(pendingItems.filter((i) => !i.draftOnly))

	function open(action: BulkAction) {
		if (targets(action).length === 0) return
		pending = action
		outcomes = undefined
		progress = 0
		moveTarget = action === 'move' ? moveTargets[0] : undefined
	}

	// The dialog's Escape fires this even while its Cancel button is disabled by
	// `loading`. A batch in flight cannot be called off, and closing over it would
	// hide the progress and drop the per-item failures it is about to report.
	function close() {
		if (running) return
		pending = undefined
		outcomes = undefined
	}

	async function confirm() {
		const action = pending
		if (!action || running) return
		if (action === 'move' && !moveTarget) return
		running = true
		progress = 0
		outcomes = undefined
		const batch = pendingItems
		let finished = false
		try {
			const res = await runBulk(action, batch, ctx, {
				target: moveTarget,
				onProgress: (done) => (progress = done)
			})
			if (action === 'discard') invalidateWorkspaceDrafts(workspace)
			const failed = res.filter((o) => o.error != undefined)
			// `running` is still set across the refetch: it is what keeps the dialog's
			// Enter from starting the same batch again over an untrimmed selection.
			await onDone()
			// Only what the batch actually changed leaves the selection. The rows it
			// skipped as ineligible stay ticked so the next action can still address
			// them — a partial failure must not drop them along with the successes.
			const untouched = items.filter((i) => !batch.some((b) => b.key === i.key)).map((i) => i.key)
			if (failed.length === 0) {
				sendUserToast(`${ACTION_LABEL[action]}: ${res.length} item${plural(res.length)}`)
				selection.keepOnly(untouched)
				finished = true
				return
			}
			// Keep the failures ticked too (and the modal open, listing them) so they
			// can be retried without rebuilding the selection.
			outcomes = res
			selection.keepOnly([...untouched, ...failed.map((o) => o.item.key)])
		} finally {
			// Released here and nowhere else: while it is set, Escape can't dismiss the
			// dialog and both its buttons are disabled, so a throw would wedge it shut.
			running = false
			if (finished) close()
		}
	}
</script>

<div
	class="fixed bottom-6 left-1/2 -translate-x-1/2 z-40 flex items-center gap-2 rounded-md border bg-surface px-3 py-2 shadow-lg"
>
	<span class="text-xs font-semibold text-emphasis whitespace-nowrap">
		{selection.size} selected
	</span>
	<div class="h-4 border-l"></div>
	<Button
		variant="subtle"
		unifiedSize="sm"
		startIcon={{ icon: FolderInput }}
		disabled={targets('move').length === 0}
		title={actionTitle('move')}
		on:click={() => open('move')}
	>
		Move{countSuffix('move')}
	</Button>
	<!-- Both render when both have targets: a selection mixing archived and active
	     rows would otherwise be a dead end for whichever action lost the toss. With
	     no targets either way, Archive stands alone and disabled, explaining why. -->
	{#if targets('archive').length > 0 || targets('unarchive').length === 0}
		<Button
			variant="subtle"
			unifiedSize="sm"
			startIcon={{ icon: Archive }}
			disabled={targets('archive').length === 0}
			title={actionTitle('archive')}
			on:click={() => open('archive')}
		>
			Archive{countSuffix('archive')}
		</Button>
	{/if}
	{#if targets('unarchive').length > 0}
		<Button
			variant="subtle"
			unifiedSize="sm"
			startIcon={{ icon: ArchiveRestore }}
			title={actionTitle('unarchive')}
			on:click={() => open('unarchive')}
		>
			Unarchive{countSuffix('unarchive')}
		</Button>
	{/if}
	<DropdownV2
		placement="top-end"
		items={[
			{
				displayName: `Discard drafts${countSuffix('discard')}`,
				icon: Trash,
				action: () => open('discard'),
				disabled: targets('discard').length === 0,
				// Only while blocked: a disabled entry can't open its modal, so the reason
				// has to live here — and an enabled one would render a pointless ⓘ.
				tooltip: targets('discard').length === 0 ? actionTitle('discard') : undefined
			},
			{
				displayName: `Delete${countSuffix('delete')}`,
				icon: Trash,
				type: 'delete' as const,
				action: () => open('delete'),
				disabled: targets('delete').length === 0,
				tooltip: targets('delete').length === 0 ? actionTitle('delete') : undefined
			}
		]}
	>
		{#snippet buttonReplacement()}
			<Button
				nonCaptureEvent
				variant="subtle"
				unifiedSize="sm"
				startIcon={{ icon: MoreHorizontal }}
				title="More actions"
			>
				More
			</Button>
		{/snippet}
	</DropdownV2>
	<div class="h-4 border-l"></div>
	<Button
		variant="subtle"
		unifiedSize="sm"
		iconOnly
		startIcon={{ icon: X }}
		title="Cancel selection (Esc)"
		on:click={() => selection.exit()}
	/>
</div>

<ConfirmationModal
	open={pending != undefined}
	title={pending ? MODAL_TITLE[pending](pendingItems.length) : ''}
	confirmationText={failures.length > 0 ? 'Retry' : pending ? ACTION_LABEL[pending] : ''}
	type={pending === 'delete' || pending === 'discard' ? 'danger' : 'info'}
	loading={running}
	onCanceled={close}
	onConfirmed={confirm}
>
	<div class="flex flex-col w-full gap-4 text-sm">
		{#if pending === 'move'}
			<Label label="Destination folder">
				<Select
					items={moveTargets.map((t) => ({ label: t, value: t }))}
					bind:value={moveTarget}
					placeholder="Select a folder"
				/>
			</Label>
			{#if moveTarget}
				{@const target = moveTarget}
				{@render pathList(
					'Will be moved to',
					pendingItems.map((i) => `${i.path} → ${movedPath(i, target)}`)
				)}
			{/if}
		{:else if pending === 'discard'}
			{#if discardDeletes.length > 0}
				<Alert type="warning" title="Permanently removed">
					These exist only as your draft, so discarding deletes them outright.
				</Alert>
				{@render pathList(
					'',
					discardDeletes.map((i) => i.displayPath)
				)}
			{/if}
			{#if discardReverts.length > 0}
				{@render pathList(
					'Draft removed, deployed version kept',
					discardReverts.map((i) => i.displayPath)
				)}
			{/if}
		{:else if pending === 'delete'}
			<Alert type="warning" title="Permanently deleted">
				Deleted items are gone from the workspace; only a workspace admin can recover them from the
				trash bin.
			</Alert>
			{@render pathList(
				'',
				pendingItems.map((i) => i.displayPath)
			)}
		{:else if pending != undefined}
			{@render pathList(
				'',
				pendingItems.map((i) => i.displayPath)
			)}
		{/if}

		{#if pending && pendingItems.length < items.length}
			{@const skipped = items.length - pendingItems.length}
			<div class="text-xs text-hint">
				{skipped} selected item{plural(skipped)} left untouched: {blockedSummary(pending)}
			</div>
		{/if}

		{#if running}
			<div class="text-xs text-secondary">{progress} / {pendingItems.length} done…</div>
		{/if}

		{#if failures.length > 0}
			<Alert type="error" title="{failures.length} failed">
				<div class="flex flex-col gap-1 mt-1 max-h-48 overflow-y-auto">
					{#each failures as f (f.item.key)}
						<div class="text-2xs">
							<span class="font-mono">{f.item.displayPath}</span>: {f.error}
						</div>
					{/each}
				</div>
			</Alert>
		{/if}
	</div>
</ConfirmationModal>

{#snippet pathList(title: string, paths: string[])}
	<div class="flex flex-col gap-1">
		{#if title}
			<span class="text-xs font-semibold text-secondary">{title}</span>
		{/if}
		<div class="flex flex-col gap-0.5 max-h-48 overflow-y-auto border rounded-md p-2">
			{#each paths as p, i (i)}
				<span class="text-2xs font-mono text-primary truncate" title={p}>{p}</span>
			{/each}
		</div>
	</div>
{/snippet}
