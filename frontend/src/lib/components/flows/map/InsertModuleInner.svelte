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
		scriptOnly?: boolean
	}

	let {
		stop = false,
		funcDesc = $bindable(''),
		disableAi = false,
		kind = 'script',
		allowTrigger = true,
		scriptOnly = false
	}: Props = $props()

	let customUi: undefined | FlowBuilderWhitelabelCustomUi = getContext('customUi')
	let selectedKind: 'script' | 'trigger' | 'preprocessor' | 'approval' | 'flow' | 'failure' =
		$state(kind)
	let preFilter: 'all' | 'workspace' | 'hub' = $state('all')
	let loading = $state(false)
	let small = $derived(kind === 'preprocessor' || kind === 'failure')

	let width = $state(0)
	let height = $state(0)

	let displayPath = $derived(width > 650 || height > 400)
</script>

<div
	id="flow-editor-insert-module"
	class="flex flex-col h-[400px] {small
		? 'w-[450px]'
		: 'w-[650px]'} pt-1 pr-1 pl-1 gap-1.5 resize overflow-auto {small
		? 'min-w-[450px]'
		: 'min-w-[650px]'} min-h-[400px]"
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
		<RefreshButton {loading} on:click={() => (refreshCount.val += 1)} />
	</div>

	<div class="flex flex-row grow min-h-0">
		{#if kind === 'script' && !scriptOnly}
			<div class="flex-none flex flex-col text-xs text-primary">
				<TopLevelNode
					label="Action"
					selected={selectedKind === 'script'}
					on:select={() => {
						selectedKind = 'script'
					}}
				/>
				{#if customUi?.triggers != false && allowTrigger}
					<TopLevelNode
						label="Trigger"
						selected={selectedKind === 'trigger'}
						on:select={() => {
							selectedKind = 'trigger'
						}}
					/>
				{/if}
				<TopLevelNode
					label="Approval/Prompt"
					selected={selectedKind === 'approval'}
					on:select={() => {
						selectedKind = 'approval'
					}}
				/>
				{#if customUi?.flowNode != false}
					<TopLevelNode
						label="Flow"
						selected={selectedKind === 'flow'}
						on:select={() => {
							selectedKind = 'flow'
						}}
					/>
				{/if}
				{#if stop}
					<TopLevelNode
						label="End flow"
						selected={selectedKind === 'script'}
						on:select={() => {
							selectedKind = 'script'
						}}
					/>
				{/if}

				<TopLevelNode
					label="For loop"
					on:select={() => {
						dispatch('close')
						dispatch('new', { kind: 'forloop' })
					}}
				/>
				<TopLevelNode
					label="While loop"
					on:select={() => {
						dispatch('close')
						dispatch('new', { kind: 'whileloop' })
					}}
				/>
				<TopLevelNode
					label="Branch to one"
					on:select={() => {
						dispatch('close')
						dispatch('new', { kind: 'branchone' })
					}}
				/>
				<TopLevelNode
					label="Branch to all"
					on:select={() => {
						dispatch('close')
						dispatch('new', { kind: 'branchall' })
					}}
				/>
				{#if customUi?.aiAgent != false}
					<TopLevelNode
						label="AI Agent"
						on:select={() => {
							dispatch('close')
							dispatch('new', { kind: 'aiagent' })
						}}
					/>
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
			on:close={() => {
				dispatch('close')
			}}
			on:new
			on:pickScript
			on:pickFlow
			{preFilter}
			{small}
			{displayPath}
			refreshCount={refreshCount.val}
		/>
	</div>
</div>
