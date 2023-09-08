<script lang="ts">
	import { getContext } from 'svelte'
	import ManualPopover from '../ManualPopover.svelte'
	import Button from '../common/button/Button.svelte'
	import { WindmillIcon } from '../icons'
	import type { FlowCopilotContext } from './flow'
	import { faMagicWandSparkles } from '@fortawesome/free-solid-svg-icons'
	import { charsToNumber } from '../flows/idUtils'
	import { existsOpenaiResourcePath } from '$lib/stores'
	import Popup from '../common/popup/Popup.svelte'

	export let copilotLoading: boolean
	export let copilotStatus: string
	export let abortController: AbortController | undefined
	export let genFlow: (index: number) => void
	export let handleFlowGenInputs: () => void

	let copilotPopover: ManualPopover | undefined = undefined

	const { modulesStore, drawerStore, currentStepStore } =
		getContext<FlowCopilotContext>('FlowCopilotContext')

	$: copilotStatus.length > 0 ? copilotPopover?.open() : copilotPopover?.close()
</script>

{#if $existsOpenaiResourcePath}
	<ManualPopover bind:this={copilotPopover}>
		<Button
			size="xs"
			btnClasses={'mr-2 ' + ($currentStepStore !== undefined ? 'z-[901]' : '')}
			on:click={() => {
				if (copilotLoading || ($currentStepStore !== undefined && $currentStepStore !== 'Input')) {
					abortController?.abort()
					copilotStatus = ''
					if (!copilotLoading) {
						$currentStepStore = undefined
					}
				} else {
					$drawerStore?.openDrawer()
				}
			}}
			startIcon={copilotLoading ||
			($currentStepStore !== undefined && $currentStepStore !== 'Input')
				? undefined
				: {
						icon: faMagicWandSparkles
				  }}
			color={copilotLoading || ($currentStepStore !== undefined && $currentStepStore !== 'Input')
				? 'red'
				: 'light'}
			variant={copilotLoading || ($currentStepStore !== undefined && $currentStepStore !== 'Input')
				? 'contained'
				: 'border'}
		>
			{#if copilotLoading}
				<WindmillIcon white class="mr-1 text-white" height="16px" width="20px" spin="veryfast" />
			{/if}

			{copilotLoading
				? 'Stop'
				: $currentStepStore !== undefined && $currentStepStore !== 'Input'
				? 'Exit'
				: 'AI Flow Builder'}
		</Button>
		<div slot="content" class="text-sm flex flex-row items-center z-[901]"
			><span class="font-semibold">
				{copilotStatus}
			</span>
			{#if !copilotLoading && $currentStepStore !== undefined && $currentStepStore !== 'Input'}
				<Button
					size="xs"
					btnClasses="ml-2"
					color="red"
					on:click={() => {
						$drawerStore?.openDrawer()
					}}>Edit prompts</Button
				>
				<Button
					btnClasses="ml-2"
					color="green"
					size="xs"
					on:click={() => {
						if ($currentStepStore === undefined) {
							return
						}
						const stepNb = charsToNumber($currentStepStore)
						if (stepNb >= $modulesStore.length - 1) {
							handleFlowGenInputs()
						} else {
							genFlow(stepNb + 1)
						}
					}}
				>
					{charsToNumber($currentStepStore) >= $modulesStore.length - 1
						? 'Flow inputs'
						: 'Next step'}
				</Button>
			{/if}</div
		>
	</ManualPopover>
{:else}
	<Popup>
		<svelte:fragment slot="button">
			<Button
				size="xs"
				btnClasses="mr-2"
				startIcon={{
					icon: faMagicWandSparkles
				}}
				color={'light'}
				variant={'border'}
				nonCaptureEvent
			>
				AI Flow Builder
			</Button>
		</svelte:fragment>
		<div class="block text-primary">
			<p class="text-sm"
				>Enable Windmill AI in the <a href="/workspace_settings?tab=openai">workspace settings.</a
				></p
			>
		</div>
	</Popup>
{/if}
