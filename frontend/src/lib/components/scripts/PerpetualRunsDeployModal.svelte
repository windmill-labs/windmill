<script lang="ts">
	import ConfirmationModal from '$lib/components/common/confirmationModal/ConfirmationModal.svelte'
	import { Alert, Button } from '$lib/components/common'
	import type { PerpetualRunsAtPath } from './perpetualRuns'

	interface Props {
		/** Open while set. */
		runs: PerpetualRunsAtPath | undefined
		/** Stops the runs without deploying, so the deploy that follows starts nothing in their place. */
		onStop: () => Promise<boolean>
		onConfirmed: () => void
		onCanceled: () => void
	}

	let { runs, onStop, onConfirmed, onCanceled }: Props = $props()

	let stopping = $state(false)
	let stopped = $state(false)

	const single = $derived(runs?.count === 1)
	const subject = $derived(single ? 'it' : 'them')
	const runsText = $derived(
		runs?.count === undefined
			? 'Runs of this script could not be listed. Any that are'
			: single
				? '1 run of this script is'
				: `${runs?.count} runs of this script are`
	)

	function reset() {
		stopping = false
		stopped = false
	}
</script>

<ConfirmationModal
	open={!!runs}
	title="Perpetual runs on an earlier version"
	confirmationText="Deploy"
	type="reload"
	loading={stopping}
	onConfirmed={() => {
		reset()
		onConfirmed()
	}}
	onCanceled={() => {
		reset()
		onCanceled()
	}}
>
	{#if runs}
		<div class="flex flex-col gap-3">
			{#if stopped}
				<p>
					{single ? 'The run was' : 'The runs were'} scaled down. Deploying starts nothing in {single
						? 'its'
						: 'their'} place.
				</p>
			{:else}
				<p>
					{runsText} queued or running on an earlier version. Deploying stops {subject} and starts {subject}
					again on this version, with the arguments {single ? 'it has' : 'they have'} now.
				</p>
				{#if runs.count === undefined || runs.mismatchedArgs.length > 0}
					<Alert
						type="warning"
						size="xs"
						title={runs.mismatchedArgs.length > 0
							? 'Arguments no longer match'
							: 'Their arguments could not be checked'}
					>
						<div class="flex flex-col items-start gap-2">
							<p>
								{#if runs.mismatchedArgs.length > 0}
									This version changes
									{runs.mismatchedArgs.length === 1 ? 'this argument' : 'these arguments'}:
									{#each runs.mismatchedArgs as arg, i (arg)}
										<code>{arg}</code>{i < runs.mismatchedArgs.length - 1 ? ', ' : '.'}
									{/each}
								{:else}
									The versions the runs are on could not be read, so arguments this version changes
									would still be carried over.
								{/if}
								{single ? 'The restarted run keeps' : 'Restarted runs keep'} the old ones. To run this
								version with arguments that fit it, scale down to 0 here and start a run yourself once
								the deploy is through.
							</p>
							<Button
								variant="default"
								unifiedSize="xs"
								btnClasses="bg-surface"
								disabled={stopping}
								onclick={async () => {
									stopping = true
									stopped = await onStop()
									stopping = false
								}}
							>
								{stopping ? 'Scaling down to 0…' : 'Scale down to 0'}
							</Button>
						</div>
					</Alert>
				{/if}
			{/if}
		</div>
	{/if}
</ConfirmationModal>
