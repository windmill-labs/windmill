<script lang="ts">
	import ConfirmationModal from '$lib/components/common/confirmationModal/ConfirmationModal.svelte'
	import { Alert, RadioCard } from '$lib/components/common'
	import type { PerpetualRunsAtPath } from './perpetualRuns'

	interface Props {
		/** Open while set. */
		runs: PerpetualRunsAtPath | undefined
		onConfirmed: (applyToPerpetualRuns: boolean) => void
		onCanceled: () => void
	}

	let { runs, onConfirmed, onCanceled }: Props = $props()

	let choice: 'switch' | 'keep' | undefined = $state(undefined)

	const runsText = $derived.by(() => {
		if (!runs) return ''
		return runs.count === 1 ? '1 run of this script is' : `${runs.count} runs of this script are`
	})
</script>

<ConfirmationModal
	open={!!runs}
	title="Perpetual runs on an earlier version"
	confirmationText="Deploy"
	type="reload"
	showIcon={false}
	confirmDisabled={!choice}
	onConfirmed={() => {
		const apply = choice === 'switch'
		choice = undefined
		onConfirmed(apply)
	}}
	onCanceled={() => {
		choice = undefined
		onCanceled()
	}}
>
	{#if runs}
		<div class="flex flex-col gap-3">
			<p>{runsText} queued or running on an earlier version.</p>
			<div class="flex flex-col gap-2" role="radiogroup" aria-label="What happens to these runs">
				<RadioCard
					label="Switch to this version"
					description="Each run finishes on its current version, then restarts on this one."
					selected={choice === 'switch'}
					onSelect={() => (choice = 'switch')}
				/>
				<RadioCard
					label="Keep their current version"
					description="Runs keep restarting on the version they use now."
					selected={choice === 'keep'}
					onSelect={() => (choice = 'keep')}
				/>
			</div>
			{#if choice === 'switch' && runs.mismatchedArgs.length > 0}
				<Alert type="warning" size="xs" title="Arguments no longer match">
					Each next run reuses the arguments of the run before it, and this version changes
					{runs.mismatchedArgs.length === 1 ? 'this argument' : 'these arguments'}:
					{#each runs.mismatchedArgs as arg, i (arg)}
						<code>{arg}</code>{i < runs.mismatchedArgs.length - 1 ? ', ' : '.'}
					{/each}
				</Alert>
			{/if}
		</div>
	{/if}
</ConfirmationModal>
