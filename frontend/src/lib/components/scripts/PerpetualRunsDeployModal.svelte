<script lang="ts">
	import ConfirmationModal from '$lib/components/common/confirmationModal/ConfirmationModal.svelte'
	import { Alert } from '$lib/components/common'
	import type { PerpetualRunsAtPath } from './perpetualRuns'

	interface Props {
		/** Open while set. */
		runs: PerpetualRunsAtPath | undefined
		onConfirmed: () => void
		onCanceled: () => void
	}

	let { runs, onConfirmed, onCanceled }: Props = $props()

	const single = $derived(runs?.count === 1)
</script>

<ConfirmationModal
	open={!!runs}
	title="Perpetual runs restart on this version"
	confirmationText="Deploy"
	type="reload"
	{onConfirmed}
	{onCanceled}
>
	{#if runs}
		<div class="flex flex-col gap-3">
			<p>
				{single ? '1 run of this script is' : `${runs.count} runs of this script are`} queued or running
				on an earlier version. Deploying stops {single ? 'it' : 'them'} and starts {single
					? 'it'
					: 'them'} again on this version, with the same arguments.
			</p>
			{#if runs.mismatchedArgs.length > 0}
				<Alert type="warning" size="xs" title="Arguments no longer match">
					Each run keeps the arguments it has now, and this version changes
					{runs.mismatchedArgs.length === 1 ? 'this argument' : 'these arguments'}:
					{#each runs.mismatchedArgs as arg, i (arg)}
						<code>{arg}</code>{i < runs.mismatchedArgs.length - 1 ? ', ' : '.'}
					{/each}
				</Alert>
			{/if}
		</div>
	{/if}
</ConfirmationModal>
