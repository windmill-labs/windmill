<script lang="ts">
	import ConfirmationModal from '$lib/components/common/confirmationModal/ConfirmationModal.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import type { DraftItemRef } from '$lib/gen'

	type Item = DraftItemRef & { label: string }

	let {
		open,
		parentName,
		targetName,
		newFork = false,
		items,
		conflicts = [],
		error,
		loading = false,
		removeFromParent = $bindable(),
		onConfirmed,
		onCanceled
	}: {
		open: boolean
		parentName: string
		targetName: string
		// A new fork is cloned with every draft of the user, not only this session's.
		newFork?: boolean
		items: Item[]
		conflicts?: DraftItemRef[]
		error?: string
		loading?: boolean
		removeFromParent: boolean
		onConfirmed: () => void
		onCanceled: () => void
	} = $props()

	const conflictKeys = $derived(new Set(conflicts.map((c) => `${c.kind}:${c.path}`)))
</script>

<ConfirmationModal
	{open}
	title="Move session to {targetName}"
	confirmationText={error && conflicts.length === 0 ? 'Retry' : 'Move'}
	type="info"
	alwaysPortal
	{loading}
	confirmDisabled={conflicts.length > 0}
	{onConfirmed}
	{onCanceled}
>
	<div class="flex flex-col gap-3">
		<p>
			The session will act on <span class="font-medium text-primary">{targetName}</span>.
			{#if newFork}
				The new fork starts with a copy of all your drafts in {parentName}, including these from
				this session:
			{:else}
				These drafts are copied there:
			{/if}
		</p>
		<ul class="flex flex-col gap-0.5 max-h-48 overflow-y-auto text-xs">
			{#each items as item (`${item.kind}:${item.path}`)}
				<li class="flex items-center gap-2 min-w-0">
					<span class="text-2xs text-tertiary shrink-0 w-14">{item.kind.replace('_', ' ')}</span>
					<span
						class="truncate font-mono {conflictKeys.has(`${item.kind}:${item.path}`)
							? 'text-red-600 dark:text-red-400'
							: 'text-primary'}">{item.label}</span
					>
				</li>
			{/each}
		</ul>
		<div class="flex items-start gap-2 border rounded-md p-3 bg-surface-secondary">
			<Toggle size="xs" id="move-remove-from-parent" bind:checked={removeFromParent} />
			<label for="move-remove-from-parent" class="flex flex-col cursor-pointer">
				<span class="text-xs font-medium text-primary">Remove these drafts from {parentName}</span>
				<span class="text-3xs text-tertiary"
					>Leaves {parentName} at its deployed versions. The work then exists only in the fork.</span
				>
			</label>
		</div>
		{#if conflicts.length > 0}
			<p class="text-xs text-red-600 dark:text-red-400">
				You already have drafts of the highlighted items in {targetName}. Discard them there first.
			</p>
		{:else if error}
			<p class="text-xs text-red-600 dark:text-red-400">{error}</p>
		{/if}
	</div>
</ConfirmationModal>
