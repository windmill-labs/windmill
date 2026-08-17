<script lang="ts">
	import ConfirmationModal from '$lib/components/common/confirmationModal/ConfirmationModal.svelte'
	import { forkConflictModal } from '$lib/stores'

	const state = $derived(forkConflictModal.val)

	// Three failure shapes we want to describe accurately:
	// - split-events: shared upstream subscription/queue, two listeners halve each other's traffic
	// - duplicate-firing: independent triggers on shared input, every event fires twice
	// - slot-takeover: PG replication slot is exclusive AND its position is destructively shared
	const SPLIT_KINDS = new Set(['kafka', 'nats', 'mqtt', 'sqs', 'gcp', 'azure'])
	const DUPLICATE_KINDS = new Set(['websocket', 'schedule'])

	const family = $derived(
		!state
			? 'unknown'
			: state.kind === 'postgres'
				? 'slot'
				: SPLIT_KINDS.has(state.kind)
					? 'split'
					: DUPLICATE_KINDS.has(state.kind)
						? 'duplicate'
						: 'unknown'
	)

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
			The parent workspace (<span class="font-mono">{state.parentWorkspaceId}</span>) has the same
			{state.kindLabel} configured at this path. Because this fork's row was cloned from it, the upstream
			identifier is shared.
		</p>
		<p class="mt-2">
			{#if family === 'split'}
				If both are enabled, the two listeners will compete on the same upstream and each side will
				receive only a fraction of its events.
			{:else if family === 'duplicate'}
				If both are enabled, every event will fire the script twice — once in the fork and once in
				the parent.
			{:else if family === 'slot'}
				The cloned <span class="font-mono">replication_slot_name</span> points at the same Postgres slot,
				which only allows one consumer at a time. Enabling here will either fail with "slot already active"
				if the parent is enabled, or hijack the slot's WAL position if it isn't — causing the parent
				to lose events when re-enabled.
			{:else}
				Enabling it here may compete for the same upstream events or duplicate side effects.
			{/if}
		</p>
		<p class="mt-2">Enable in this fork anyway?</p>
	{/if}
</ConfirmationModal>
