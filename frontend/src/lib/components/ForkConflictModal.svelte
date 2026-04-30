<script lang="ts">
	import ConfirmationModal from '$lib/components/common/confirmationModal/ConfirmationModal.svelte'
	import { forkConflictModal } from '$lib/stores'

	const state = $derived(forkConflictModal.val)

	function close(confirmed: boolean) {
		const resolve = state?.resolve
		forkConflictModal.val = undefined
		resolve?.(confirmed)
	}
</script>

<ConfirmationModal
	open={!!state}
	title="Enable in fork conflicts with parent"
	confirmationText="Enable anyway"
	onConfirmed={() => close(true)}
	onCanceled={() => close(false)}
>
	{#if state}
		<p>
			This {state.kindLabel} is also enabled in the parent workspace (<span class="font-mono"
				>{state.parentWorkspaceId}</span
			>). Enabling it here means both will run at the same time and may compete for the same
			upstream events or duplicate side effects.
		</p>
		<p class="mt-2">Enable in this fork anyway?</p>
	{/if}
</ConfirmationModal>
