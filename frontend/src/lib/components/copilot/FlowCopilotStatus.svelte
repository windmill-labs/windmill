<script lang="ts">
	import { getContext } from 'svelte'
	import ManualPopover from '../ManualPopover.svelte'
	import Button from '../common/button/Button.svelte'
	import { WindmillIcon } from '../icons'
	import type { FlowCopilotContext } from './flow'
	import { charsToNumber } from '../flows/idUtils'
	import { copilotInfo } from '$lib/stores'
	import Popup from '../common/popup/Popup.svelte'
	import type { FlowModule } from '$lib/gen'
	import type { FlowEditorContext } from '../flows/types'
	import { ExternalLink, Wand2 } from 'lucide-svelte'

	export let copilotLoading: boolean
	export let copilotStatus: string
	export let abortController: AbortController | undefined
	export let genFlow: (index: number, modules: FlowModule[], stepOnly?: boolean) => void
	export let finishCopilotFlowBuilder: () => void

	let copilotPopover: ManualPopover | undefined = undefined

	const { flowStore } = getContext<FlowEditorContext>('FlowEditorContext')

	const { modulesStore, drawerStore, currentStepStore } =
		getContext<FlowCopilotContext>('FlowCopilotContext')

	$: copilotStatus.length > 0 ? copilotPopover?.open() : copilotPopover?.close()

	$: copilotStatus && copilotPopover?.refresh()
</script>

{#if $copilotInfo.exists_openai_resource_path}
	<ManualPopover bind:this={copilotPopover}>
		<Button
			size="xs"
			btnClasses={$currentStepStore !== undefined ? 'z-[901]' : ''}
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
						icon: Wand2
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
		<div slot="content" class="text-sm flex flex-row items-center z-[901]">
			<span class="font-semibold">
				{copilotStatus}
			</span>
			{#if !copilotLoading && $currentStepStore !== undefined && $currentStepStore !== 'Input'}
				<Button
					size="xs"
					btnClasses="ml-2"
					color="red"
					on:click={() => {
						$drawerStore?.openDrawer()
					}}
				>
					Edit prompts
				</Button>
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
							finishCopilotFlowBuilder()
						} else {
							genFlow(stepNb + 1, $flowStore.value.modules)
						}
					}}
				>
					{charsToNumber($currentStepStore) >= $modulesStore.length - 1
						? 'Flow inputs'
						: 'Next step'}
				</Button>
			{/if}
		</div>
	</ManualPopover>
{:else}
	<Popup
		floatingConfig={{
			strategy: 'absolute',
			placement: 'bottom'
		}}
	>
		<svelte:fragment slot="button">
			<Button
				size="xs"
				startIcon={{
					icon: Wand2
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
				>Enable Windmill AI in the <a
					href="/workspace_settings?tab=openai"
					target="_blank"
					class="inline-flex flex-row items-center gap-1"
					>workspace settings <ExternalLink size={16} /></a
				></p
			>
		</div>
	</Popup>
{/if}
