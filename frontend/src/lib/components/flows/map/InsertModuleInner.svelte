<script lang="ts" module>
	let refreshCount = $state({ val: 0 })
</script>

<script lang="ts">
	import { createEventDispatcher, getContext } from 'svelte'
	import StepGenQuick from '$lib/components/copilot/StepGenQuick.svelte'
	import FlowInputsQuick from '../content/FlowInputsQuick.svelte'
	import type { FlowBuilderWhitelabelCustomUi } from '$lib/components/custom_ui'
	import ToggleHubWorkspaceQuick from '$lib/components/ToggleHubWorkspaceQuick.svelte'
	import TopLevelNode from '../pickers/TopLevelNode.svelte'
	import RefreshButton from '$lib/components/common/button/RefreshButton.svelte'

	const dispatch = createEventDispatcher()
	interface Props {
		stop?: boolean
		funcDesc?: string
		disableAi?: boolean
		kind?: 'script' | 'trigger' | 'preprocessor' | 'failure'
		allowTrigger?: boolean
		toolMode?: boolean
	}

	let {
		stop = false,
		funcDesc = $bindable(''),
		disableAi = false,
		kind = 'script',
		allowTrigger = true,
		toolMode = false
	}: Props = $props()

	let customUi: undefined | FlowBuilderWhitelabelCustomUi = getContext('customUi')
	let selectedKind: 'script' | 'trigger' | 'preprocessor' | 'approval' | 'flow' | 'failure' =
		$state(kind)
	let preFilter: 'all' | 'workspace' | 'hub' = $state('all')
	let loading = $state(false)
	let small = $derived(kind === 'preprocessor' || kind === 'failure')

	let width = $state(0)
	let height = $state(0)
	let owners = $state([])
	let displayPath = $derived(width > 650 || height > 400)
</script>

<div
	id="flow-editor-insert-module"
	class="flex flex-col h-full {small ? 'w-[450px]' : 'w-[650px]'} gap-2 {small
		? 'min-w-[450px]'
		: 'min-w-[650px]'}"
	onwheel={(e) => {
		e.stopPropagation()
	}}
	role="none"
	bind:clientWidth={width}
	bind:clientHeight={height}
>
	<div class="flex flex-row items-center gap-2">
		<StepGenQuick
			on:escape={() => dispatch('close')}
			{disableAi}
			on:insert
			bind:funcDesc
			{preFilter}
			{loading}
		/>
		{#if selectedKind != 'preprocessor' && selectedKind != 'flow'}
			<ToggleHubWorkspaceQuick bind:selected={preFilter} />
		{/if}
		<RefreshButton
			size="sm"
			light
			{loading}
			onClick={() => {
				refreshCount.val += 1
			}}
		/>
	</div>

	<div class="flex flex-row grow min-h-0 gap-2">
		{#if kind === 'script'}
			<div class="flex-none flex flex-col text-xs text-primary overflow-auto gap-1">
				<TopLevelNode
					label="Action"
					selected={selectedKind === 'script'}
					onSelect={() => {
						selectedKind = 'script'
					}}
				/>
				{#if toolMode}
					<TopLevelNode
						label="MCP"
						onSelect={() => {
							dispatch('pickMcpTool')
							dispatch('close')
						}}
					/>
					<TopLevelNode
						label="Web Search"
						onSelect={() => {
							dispatch('pickWebsearchTool')
							dispatch('close')
						}}
					/>
					<TopLevelNode
						label="AI Agent"
						onSelect={() => {
							dispatch('pickAiAgentTool')
							dispatch('close')
						}}
					/>
				{:else}
					{#if customUi?.triggers != false && allowTrigger}
						<TopLevelNode
							label="Trigger"
							selected={selectedKind === 'trigger'}
							onSelect={() => {
								selectedKind = 'trigger'
							}}
						/>
					{/if}
					<TopLevelNode
						label="Approval/Prompt"
						selected={selectedKind === 'approval'}
						onSelect={() => {
							selectedKind = 'approval'
						}}
					/>
					{#if customUi?.flowNode != false}
						<TopLevelNode
							label="Flow"
							selected={selectedKind === 'flow'}
							onSelect={() => {
								selectedKind = 'flow'
							}}
						/>
					{/if}
					{#if stop}
						<TopLevelNode
							label="End flow"
							selected={selectedKind === 'script'}
							onSelect={() => {
								selectedKind = 'script'
							}}
						/>
					{/if}

					<TopLevelNode
						label="For loop"
						onSelect={() => {
							dispatch('close')
							dispatch('new', { kind: 'forloop' })
						}}
					/>
					<TopLevelNode
						label="While loop"
						onSelect={() => {
							dispatch('close')
							dispatch('new', { kind: 'whileloop' })
						}}
					/>
					<TopLevelNode
						label="Branch to one"
						onSelect={() => {
							dispatch('close')
							dispatch('new', { kind: 'branchone' })
						}}
					/>
					<TopLevelNode
						label="Branch to all"
						onSelect={() => {
							dispatch('close')
							dispatch('new', { kind: 'branchall' })
						}}
					/>
					{#if customUi?.aiAgent != false}
						<TopLevelNode
							label="AI Agent"
							onSelect={() => {
								dispatch('close')
								dispatch('new', { kind: 'aiagent' })
							}}
						/>
					{/if}
				{/if}
			</div>
		{/if}

		<FlowInputsQuick
			{selectedKind}
			bind:loading
			filter={funcDesc}
			{disableAi}
			{funcDesc}
			{kind}
			bind:owners
			on:close={() => {
				dispatch('close')
			}}
			on:new
			on:pickScript
			on:pickFlow
			{preFilter}
			{displayPath}
			refreshCount={refreshCount.val}
		/>
	</div>
</div>
