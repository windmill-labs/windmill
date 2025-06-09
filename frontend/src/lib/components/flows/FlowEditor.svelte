<script lang="ts">
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import FlowEditorPanel from './content/FlowEditorPanel.svelte'
	import FlowModuleSchemaMap from './map/FlowModuleSchemaMap.svelte'
	import WindmillIcon from '../icons/WindmillIcon.svelte'
	import { Skeleton } from '../common'
	import { getContext, setContext } from 'svelte'
	import type { FlowEditorContext } from './types'

	import { writable } from 'svelte/store'
	import type { PropPickerContext, FlowPropPickerConfig } from '$lib/components/prop_picker'
	import type { PickableProperties } from '$lib/components/flows/previousResults'
	import type { Flow } from '$lib/gen'
	import type { Trigger } from '$lib/components/triggers/utils'
	import FlowAIChat from '../copilot/chat/flow/FlowAIChat.svelte'
	import HideButton from '../apps/editor/settingsPanel/HideButton.svelte'
	import { chatMode, copilotInfo } from '$lib/stores'
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

	const DEFAULT_AI_PANEL_SIZE = 25
	const DEFAULT_MODULE_PANEL_SIZE = 35
	const DEFAULT_MAP_PANEL_SIZE = 40

	let aiPanelSize =
		!$copilotInfo.enabled || localStorage.getItem('aiPanelOpen') === 'false'
			? 0
			: DEFAULT_AI_PANEL_SIZE
	let modulePanelSize = DEFAULT_MODULE_PANEL_SIZE + (DEFAULT_AI_PANEL_SIZE - aiPanelSize)
	let storedAiPanelSize = aiPanelSize > 0 ? aiPanelSize : DEFAULT_AI_PANEL_SIZE
	let mapPanelSize = DEFAULT_MAP_PANEL_SIZE

	export function toggleAiPanel(mode?: 'script' | 'flow') {
		if (!$copilotInfo.enabled) return
		if (aiPanelSize > 0) {
			storedAiPanelSize = aiPanelSize
			modulePanelSize += aiPanelSize
			aiPanelSize = 0
			localStorage.setItem('aiPanelOpen', 'false')
		} else {
			modulePanelSize -= storedAiPanelSize
			aiPanelSize = storedAiPanelSize
			localStorage.setItem('aiPanelOpen', 'true')
			if (mode) {
				chatMode.set(mode)
			}
		}
	}

	export function addSelectedLinesToAiChat(lines: string, startLine: number, endLine: number) {
		flowAIChat?.addSelectedLinesToContext(lines, startLine, endLine)
		if (getIsAiPanelClosed()) {
			toggleAiPanel('script')
		}
	}

	export function getIsAiPanelClosed() {
		return aiPanelSize === 0
	}

	let flowAIChat: FlowAIChat | undefined = undefined
</script>

<svelte:window
	on:keydown={(e) => {
		if ((e.ctrlKey || e.metaKey) && e.key === 'l') {
			e.preventDefault()
			toggleAiPanel()
		}
	}}
/>

<div
	id="flow-editor"
	class={'h-full overflow-hidden transition-colors duration-[400ms] ease-linear border-t'}
>
	<Splitpanes>
		<Pane bind:size={mapPanelSize} minSize={15} class="h-full relative z-0">
			<div class="grow overflow-hidden bg-gray h-full bg-surface-secondary relative">
				{#if loading}
					<div class="p-2 pt-10">
						{#each new Array(6) as _}
							<Skeleton layout={[[2], 1.5]} />
						{/each}
					</div>
				{:else if flowStore.val.value.modules}
					<FlowModuleSchemaMap
						bind:this={flowModuleSchemaMap}
						{disableStaticInputs}
						{disableTutorials}
						{disableAi}
						{disableSettings}
						{smallErrorHandler}
						{newFlow}
						bind:modules={flowStore.val.value.modules}
						on:reload
						on:generateStep={({ detail }) => {
							if (getIsAiPanelClosed()) {
								toggleAiPanel()
							}
							flowAIChat?.generateStep(detail.moduleId, detail.lang, detail.instructions)
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
		{#if !disableAi && aiPanelSize > 0}
			{#snippet aiChatHeaderLeft()}
				<HideButton
					hidden={false}
					direction="right"
					panelName="AI"
					shortcut="L"
					size="md"
					on:click={() => {
						toggleAiPanel()
					}}
				/>
			{/snippet}
			<Pane bind:size={aiPanelSize}>
				<FlowAIChat bind:this={flowAIChat} {flowModuleSchemaMap} headerLeft={aiChatHeaderLeft} />
			</Pane>
		{/if}
	</Splitpanes>
</div>
