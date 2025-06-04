<script lang="ts">
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import FlowEditorPanel from './content/FlowEditorPanel.svelte'
	import FlowModuleSchemaMap from './map/FlowModuleSchemaMap.svelte'
	import WindmillIcon from '../icons/WindmillIcon.svelte'
	import { Skeleton } from '../common'
	import { getContext, onDestroy, onMount, setContext } from 'svelte'
	import type { FlowEditorContext } from './types'

	import { writable } from 'svelte/store'
	import type { PropPickerContext, FlowPropPickerConfig } from '$lib/components/prop_picker'
	import type { PickableProperties } from '$lib/components/flows/previousResults'
	import type { Flow } from '$lib/gen'
	import type { Trigger } from '$lib/components/triggers/utils'
	import FlowAIChat from '../copilot/chat/flow/FlowAIChat.svelte'
	import { aiChatManager } from '../copilot/chat/AIChatManager.svelte'
	import TriggerableByAI from '../TriggerableByAI.svelte'
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

	let flowModuleSchemaMap: FlowModuleSchemaMap | undefined

	setContext<PropPickerContext>('PropPickerContext', {
		flowPropPickerConfig: writable<FlowPropPickerConfig | undefined>(undefined),
		pickablePropertiesFiltered: writable<PickableProperties | undefined>(undefined)
	})

	const DEFAULT_MODULE_PANEL_SIZE = 60
	const DEFAULT_MAP_PANEL_SIZE = 40

	let modulePanelSize = DEFAULT_MODULE_PANEL_SIZE
	let mapPanelSize = DEFAULT_MAP_PANEL_SIZE

	export function addSelectedLinesToAiChat(lines: string, startLine: number, endLine: number) {
		aiChatManager.addSelectedLinesToContext(lines, startLine, endLine)
		if (!aiChatManager.open) {
			aiChatManager.openChat()
			aiChatManager.changeMode('script')
		}
	}

	onMount(() => {
		aiChatManager.changeMode('flow')
	})

	onDestroy(() => {
		aiChatManager.changeMode('navigator')
	})
</script>

<div
	id="flow-editor"
	class={'h-full overflow-hidden transition-colors duration-[400ms] ease-linear border-t'}
>
	<TriggerableByAI id="flow-editor" description="Component to edit a flow" />
	<Splitpanes>
		<Pane bind:size={mapPanelSize} minSize={15} class="h-full relative z-0">
			<div class="grow overflow-hidden bg-gray h-full bg-surface-secondary relative">
				{#if loading}
					<div class="p-2 pt-10">
						{#each new Array(6) as _}
							<Skeleton layout={[[2], 1.5]} />
						{/each}
					</div>
				{:else if $flowStore.value.modules}
					<FlowModuleSchemaMap
						bind:this={flowModuleSchemaMap}
						{disableStaticInputs}
						{disableTutorials}
						{disableAi}
						{disableSettings}
						{smallErrorHandler}
						{newFlow}
						bind:modules={$flowStore.value.modules}
						on:reload
						on:generateStep={({ detail }) => {
							if (!aiChatManager.open) {
								aiChatManager.openChat()
							}
							aiChatManager.generateStep(detail.moduleId, detail.lang, detail.instructions)
						}}
					/>
				{/if}
			</div>
		</Pane>
		<Pane class="relative z-10" bind:size={modulePanelSize} minSize={20}>
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
		<FlowAIChat {flowModuleSchemaMap} />
	</Splitpanes>
</div>
