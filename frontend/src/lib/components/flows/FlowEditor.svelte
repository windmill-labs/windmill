<script lang="ts">
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import FlowEditorPanel from './content/FlowEditorPanel.svelte'
	import FlowModuleSchemaMap from './map/FlowModuleSchemaMap.svelte'
	import WindmillIcon from '../icons/WindmillIcon.svelte'
	import { Skeleton } from '../common'
	import { getContext, onDestroy, onMount, setContext } from 'svelte'
	import type { FlowEditorContext } from './types'

	import { writable, type Writable } from 'svelte/store'
	import type { PropPickerContext, FlowPropPickerConfig } from '$lib/components/prop_picker'
	import type { PickableProperties } from '$lib/components/flows/previousResults'
	import type { Flow, Job } from '$lib/gen'
	import type { Trigger } from '$lib/components/triggers/utils'
	import FlowAIChat from '../copilot/chat/flow/FlowAIChat.svelte'
	import { aiChatManager, AIMode } from '../copilot/chat/AIChatManager.svelte'
	import type { DurationStatus, GraphModuleState } from '../graph'
	import { triggerableByAI } from '$lib/actions/triggerableByAI.svelte'
	const { flowStore } = getContext<FlowEditorContext>('FlowEditorContext')

	interface Props {
		loading: boolean
		disableStaticInputs?: boolean
		disableTutorials?: boolean
		disableAi?: boolean
		disableSettings?: boolean
		disabledFlowInputs?: boolean
		smallErrorHandler?: boolean
		newFlow?: boolean
		savedFlow?:
			| (Flow & {
					draft?: Flow | undefined
			  })
			| undefined
		onDeployTrigger?: (trigger: Trigger) => void
		onTestUpTo?: ((id: string) => void) | undefined
		onEditInput?: ((moduleId: string, key: string) => void) | undefined
		forceTestTab?: Record<string, boolean>
		highlightArg?: Record<string, string | undefined>
		aiChatOpen?: boolean
		showFlowAiButton?: boolean
		toggleAiChat?: () => void
		localModuleStates?: Writable<Record<string, GraphModuleState>>
		isOwner?: boolean
		onTestFlow?: () => void
		isRunning?: boolean
		onCancelTestFlow?: () => void
		onOpenPreview?: () => void
		onHideJobStatus?: () => void
		individualStepTests?: boolean
		localDurationStatuses?: Writable<Record<string, DurationStatus>>
		suspendStatus?: Writable<Record<string, { job: Job; nb: number }>>
	}

	let {
		loading,
		disableStaticInputs = false,
		disableTutorials = false,
		disableAi = false,
		disableSettings = false,
		disabledFlowInputs = false,
		smallErrorHandler = false,
		newFlow = false,
		savedFlow = undefined,
		onDeployTrigger = () => {},
		onTestUpTo = undefined,
		onEditInput = undefined,
		forceTestTab,
		highlightArg,
		localModuleStates = writable({}),
		aiChatOpen,
		showFlowAiButton,
		toggleAiChat,
		isOwner,
		onTestFlow,
		isRunning,
		onCancelTestFlow,
		onOpenPreview,
		onHideJobStatus,
		individualStepTests = false,
		localDurationStatuses,
		suspendStatus
	}: Props = $props()

	let flowModuleSchemaMap: FlowModuleSchemaMap | undefined = $state()

	export function isNodeVisible(nodeId: string): boolean {
		return flowModuleSchemaMap?.isNodeVisible(nodeId) ?? false
	}

	setContext<PropPickerContext>('PropPickerContext', {
		flowPropPickerConfig: writable<FlowPropPickerConfig | undefined>(undefined),
		pickablePropertiesFiltered: writable<PickableProperties | undefined>(undefined)
	})

	onMount(() => {
		aiChatManager.changeMode(AIMode.FLOW)
	})

	onDestroy(() => {
		aiChatManager.changeMode(AIMode.NAVIGATOR)
	})
</script>

<div
	id="flow-editor"
	class={'h-full overflow-hidden transition-colors duration-[400ms] ease-linear border-t'}
	use:triggerableByAI={{
		id: 'flow-editor',
		description: 'Component to edit a flow'
	}}
>
	<Splitpanes>
		<Pane size={50} minSize={15} class="h-full relative z-0">
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
						on:reload
						on:generateStep={({ detail }) => {
							if (!aiChatManager.open) {
								aiChatManager.openChat()
							}
							aiChatManager.generateStep(detail.moduleId, detail.lang, detail.instructions)
						}}
						{onTestUpTo}
						{onEditInput}
						{localModuleStates}
						{aiChatOpen}
						{showFlowAiButton}
						{toggleAiChat}
						{isOwner}
						{onTestFlow}
						{isRunning}
						{onCancelTestFlow}
						{onOpenPreview}
						{onHideJobStatus}
						{individualStepTests}
						{suspendStatus}
					/>
				{/if}
			</div>
		</Pane>
		<Pane class="relative z-10" size={50} minSize={20}>
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
					{forceTestTab}
					{highlightArg}
					{onTestFlow}
					{isOwner}
					{localDurationStatuses}
					{suspendStatus}
					onOpenDetails={onOpenPreview}
				/>
			{/if}
		</Pane>
		{#if !disableAi}
			<FlowAIChat {flowModuleSchemaMap} />
		{/if}
	</Splitpanes>
</div>
