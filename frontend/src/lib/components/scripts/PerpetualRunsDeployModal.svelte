<script lang="ts">
	import ConfirmationModal from '$lib/components/common/confirmationModal/ConfirmationModal.svelte'
	import { Alert, RadioCard } from '$lib/components/common'
	import type { PerpetualRunsAtPath } from './perpetualRuns'

	interface Props {
		/** Open while set. */
		runs: PerpetualRunsAtPath | undefined
		onConfirmed: (choice: 'restart' | 'stop') => void
		onCanceled: () => void
	}

	let { runs, onConfirmed, onCanceled }: Props = $props()

	let choice: 'restart' | 'stop' = $state('restart')

	const single = $derived(runs?.count === 1)
	const subject = $derived(single ? 'it' : 'them')
	const runsText = $derived(
		runs?.count === undefined
			? 'Runs of this script could not be listed. Any that are'
			: single
				? '1 run of this script is'
				: `${runs?.count} runs of this script are`
	)
</script>

<ConfirmationModal
	open={!!runs}
	title="Perpetual runs on an earlier version"
	confirmationText="Deploy"
	type="reload"
	onConfirmed={() => {
		const made = choice
		choice = 'restart'
		onConfirmed(made)
	}}
	onCanceled={() => {
		choice = 'restart'
		onCanceled()
	}}
>
	{#if runs}
		<div class="flex flex-col gap-3">
			<p>{runsText} queued or running on an earlier version.</p>
			<div class="flex flex-col gap-2" role="radiogroup" aria-label="What happens to these runs">
				<RadioCard
					label="Restart on this version"
					description="Each run stops and starts again on this version, with the arguments it has now."
					selected={choice === 'restart'}
					onSelect={() => (choice = 'restart')}
				/>
				<RadioCard
					label="Stop {subject}"
					description="Each run stops and nothing takes its place."
					selected={choice === 'stop'}
					onSelect={() => (choice = 'stop')}
				/>
			</div>
			{#if choice === 'restart' && runs.mismatchedArgs.length > 0}
				<Alert type="warning" size="xs" title="Arguments no longer match">
					Each run keeps the arguments it has now, and this version changes
					{runs.mismatchedArgs.length === 1 ? 'this argument' : 'these arguments'}:
					{#each runs.mismatchedArgs as arg, i (arg)}
						<code>{arg}</code>{i < runs.mismatchedArgs.length - 1 ? ', ' : '.'}
					{/each}
					Stopping {subject} instead leaves the new version to be started with arguments that match.
				</Alert>
			{/if}
		</div>
	{/if}
</ConfirmationModal>
