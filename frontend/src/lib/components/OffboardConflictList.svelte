<script lang="ts">
	import { Alert, Button } from '$lib/components/common'
	import ConfirmationModal from '$lib/components/common/confirmationModal/ConfirmationModal.svelte'
	import type { OffboardPathConflict } from '$lib/gen'
	import { apiErrorMessage } from '$lib/utils'
	import { sendUserToast } from '$lib/toast'
	import {
		canDeleteKind,
		conflictsGoneAfterDelete,
		deleteItemOfKind,
		isTrashedKind,
		itemHref,
		kindLabel
	} from './offboarding-utils'

	type Props = {
		conflicts: OffboardPathConflict[]
		/** Prefix each line with its workspace (instance-level offboarding). */
		showWorkspace?: boolean
		onResolved: (conflict: OffboardPathConflict) => void
	}

	let { conflicts, showWorkspace = false, onResolved }: Props = $props()

	type Action = 'overwrite' | 'discard'
	let pending: { conflict: OffboardPathConflict; action: Action } | undefined = $state(undefined)
	let deleting = $state(false)

	function pathToDelete(p: NonNullable<typeof pending>): string {
		return p.action === 'overwrite' ? p.conflict.target_path : p.conflict.source_path
	}

	async function confirm() {
		if (!pending) return
		const { conflict } = pending
		const path = pathToDelete(pending)
		deleting = true
		try {
			await deleteItemOfKind(conflict.kind, conflict.workspace_id, path)
		} catch (e) {
			sendUserToast(`Could not delete ${path}: ${apiErrorMessage(e)}`, true)
			deleting = false
			return
		}
		sendUserToast(`Deleted ${kindLabel(conflict.kind)} ${path}`)
		const remaining = conflicts.filter((c) => c !== conflict)
		onResolved(conflict)
		pending = undefined
		try {
			for (const gone of await conflictsGoneAfterDelete(conflict.kind, remaining)) {
				onResolved(gone)
			}
		} catch (e) {
			sendUserToast(`Could not refresh the remaining conflicts: ${apiErrorMessage(e)}`, true)
		} finally {
			deleting = false
		}
	}
</script>

<Alert type="error" title="Path conflicts detected">
	<p class="text-xs mb-2">
		These items already exist at the target. For each one, overwrite the existing item with the
		reassigned one, or discard the reassigned one and keep the existing item. You can also choose a
		different user/folder.
	</p>
	<ul class="flex flex-col gap-1 max-h-48 overflow-y-auto">
		{#each conflicts as conflict (`${conflict.workspace_id}:${conflict.kind}:${conflict.source_path}`)}
			{@const href = itemHref(conflict.kind, conflict.target_path)}
			<li class="flex items-center gap-2 text-xs">
				<span class="flex-1 min-w-0 truncate" title={conflict.target_path}>
					{#if showWorkspace}
						<span class="text-secondary">[{conflict.workspace_id}]</span>
					{/if}
					<span class="text-secondary">{kindLabel(conflict.kind)}:</span>
					{#if href && !showWorkspace}
						<a {href} target="_blank" rel="noopener noreferrer">{conflict.target_path}</a>
					{:else}
						{conflict.target_path}
					{/if}
				</span>
				{#if canDeleteKind(conflict.kind)}
					<Button
						unifiedSize="xs"
						variant="default"
						disabled={deleting}
						destructive
						title="Delete the existing {conflict.target_path} so {conflict.source_path} can take its place"
						onclick={() => (pending = { conflict, action: 'overwrite' })}
					>
						Overwrite target
					</Button>
					<Button
						unifiedSize="xs"
						variant="default"
						disabled={deleting}
						title="Delete {conflict.source_path} and keep the existing {conflict.target_path}"
						onclick={() => (pending = { conflict, action: 'discard' })}
					>
						Discard
					</Button>
				{/if}
			</li>
		{/each}
	</ul>
</Alert>

<ConfirmationModal
	open={pending != undefined}
	title={pending?.action === 'overwrite' ? 'Overwrite target' : 'Discard reassigned item'}
	confirmationText="Delete"
	loading={deleting}
	trashbin={pending != undefined && isTrashedKind(pending.conflict.kind)}
	zIndexClass="z-[6000]"
	alwaysPortal
	onConfirmed={confirm}
	onCanceled={() => (pending = undefined)}
>
	{#if pending}
		<p class="text-sm">
			{#if pending.action === 'overwrite'}
				The existing {kindLabel(pending.conflict.kind)}
				<span class="font-mono">{pending.conflict.target_path}</span> will be deleted, and
				<span class="font-mono">{pending.conflict.source_path}</span> will be moved to that path on reassignment.
			{:else}
				The {kindLabel(pending.conflict.kind)}
				<span class="font-mono">{pending.conflict.source_path}</span> will be deleted instead of
				being reassigned. The existing
				<span class="font-mono">{pending.conflict.target_path}</span> is kept.
			{/if}
		</p>
	{/if}
</ConfirmationModal>
