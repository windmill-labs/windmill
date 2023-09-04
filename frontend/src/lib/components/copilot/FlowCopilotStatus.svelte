<script lang="ts">
	import { getContext } from 'svelte'
	import ManualPopover from '../ManualPopover.svelte'
	import Button from '../common/button/Button.svelte'
	import { WindmillIcon } from '../icons'
	import type { FlowCopilotContext } from './flow'
	import { faMagicWandSparkles } from '@fortawesome/free-solid-svg-icons'

	export let copilotLoading: boolean
	export let waitingStep: number | undefined
	export let copilotStatus: string
	export let abortController: AbortController | undefined
	export let genFlow: (index: number) => void
	export let handleFlowGenInputs: () => void

	let copilotPopover: ManualPopover | undefined = undefined

	const { modulesStore, drawerStore } = getContext<FlowCopilotContext>('FlowCopilotContext')

	$: copilotStatus.length > 0 ? copilotPopover?.open() : copilotPopover?.close()
</script>

<ManualPopover bind:this={copilotPopover}>
	<Button
		size="xs"
		btnClasses="mr-2"
		on:click={() => {
			if (copilotLoading || waitingStep !== undefined) {
				abortController?.abort()
				waitingStep = undefined
				copilotStatus = ''
			} else {
				$drawerStore?.openDrawer()
			}
		}}
		startIcon={copilotLoading
			? undefined
			: {
					icon: faMagicWandSparkles
			  }}
		color={copilotLoading || waitingStep !== undefined ? 'red' : 'light'}
		variant="border"
	>
		{#if copilotLoading}
			<WindmillIcon class="mr-1 text-white" height="16px" width="20px" spin="veryfast" />
		{/if}

		{copilotLoading || waitingStep !== undefined ? 'Cancel' : 'AI Flow Builder'}
	</Button>
	<div slot="content" class="text-sm flex flex-row items-center"
		><span class="font-semibold">{copilotStatus}</span>
		{#if waitingStep !== undefined}
			<Button
				btnClasses="ml-2"
				color="green"
				on:click={() => {
					if (waitingStep === undefined) {
						return
					}
					if (waitingStep >= $modulesStore.length - 1) {
						handleFlowGenInputs()
					} else {
						genFlow(waitingStep + 1)
					}
				}}
			>
				Validate and continue
			</Button>
		{/if}</div
	>
</ManualPopover>
