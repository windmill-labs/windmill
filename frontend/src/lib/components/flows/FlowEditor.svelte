<script lang="ts">
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import FlowEditorPanel from './content/FlowEditorPanel.svelte'
	import FlowModuleSchemaMap from './map/FlowModuleSchemaMap.svelte'
	import WindmillIcon from '../icons/WindmillIcon.svelte'
	import { Skeleton } from '../common'
	import { getContext, setContext } from 'svelte'
	import type { FlowEditorContext } from './types'
	import type { FlowCopilotContext } from '../copilot/flow'
	import { classNames } from '$lib/utils'

	import { writable } from 'svelte/store'
	import type { PropPickerContext, FlowPropPickerConfig } from '$lib/components/prop_picker'
	import type { PickableProperties } from '$lib/components/flows/previousResults'
	import type { Flow } from '$lib/gen'
	import type { Trigger } from '$lib/components/triggers/utils'
	const { flowStore } = getContext<FlowEditorContext>('FlowEditorContext')

	export let loading: boolean
	export let disableStaticInputs = false
	export let disableTutorials = false
	export let disableAi = false
	export let disableSettings = false
	export let disabledFlowInputs = false
	export let smallErrorHandler = false
	export let newFlow: boolean = false
	export let savedFlow:
		| (Flow & {
				draft?: Flow | undefined
		  })
		| undefined = undefined
	export let onDeployTrigger: (trigger: Trigger) => void = () => {}

	let size = 50

	const { currentStepStore: copilotCurrentStepStore } =
		getContext<FlowCopilotContext>('FlowCopilotContext')

	setContext<PropPickerContext>('PropPickerContext', {
		flowPropPickerConfig: writable<FlowPropPickerConfig | undefined>(undefined),
		pickablePropertiesFiltered: writable<PickableProperties | undefined>(undefined)
	})
</script>

<div
	id="flow-editor"
	class={classNames(
		'h-full overflow-hidden transition-colors duration-[400ms] ease-linear border-t',
		$copilotCurrentStepStore !== undefined ? 'border-gray-500/75' : ''
	)}
>
	<Splitpanes>
		<Pane {size} minSize={15} class="h-full relative z-0">
			<div class="grow overflow-hidden bg-gray h-full bg-surface-secondary relative">
				{#if loading}
					<div class="p-2 pt-10">
						{#each new Array(6) as _}
							<Skeleton layout={[[2], 1.5]} />
						{/each}
					</div>
				{:else if $flowStore.value.modules}
					<FlowModuleSchemaMap
						{disableStaticInputs}
						{disableTutorials}
						{disableAi}
						{disableSettings}
						{smallErrorHandler}
						{newFlow}
						bind:modules={$flowStore.value.modules}
						on:reload
					/>
				{/if}
			</div>
		</Pane>
		<Pane class="relative z-10" size={100 - size} minSize={40}>
			{#if loading}
				<div class="w-full h-full">
					<div class="block m-auto pt-40 w-10">
						<WindmillIcon height="40px" width="40px" spin="fast" />
					</div>
				</div>
			{:else}
				<FlowEditorPanel
					{disabledFlowInputs}
					{newFlow}
					{savedFlow}
					enableAi={!disableAi}
					on:applyArgs
					on:testWithArgs
					{onDeployTrigger}
				/>
			{/if}
		</Pane>
	</Splitpanes>
</div>
