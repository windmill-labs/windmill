<script lang="ts">
	import { createEventDispatcher, getContext } from 'svelte'
	import { Cross } from 'lucide-svelte'
	import StepGenQuick from '$lib/components/copilot/StepGenQuick.svelte'
	import FlowInputsQuick from '../content/FlowInputsQuick.svelte'
	import type { FlowModule } from '$lib/gen'
	import type { FlowBuilderWhitelabelCustomUi } from '$lib/components/custom_ui'
	import ToggleHubWorkspaceQuick from '$lib/components/ToggleHubWorkspaceQuick.svelte'
	import { twMerge } from 'tailwind-merge'
	import type { ComputeConfig } from 'svelte-floating-ui'
	import TopLevelNode from '../pickers/TopLevelNode.svelte'
	import PopupV2 from '$lib/components/common/popup/PopupV2.svelte'
	import { flip, offset } from 'svelte-floating-ui/dom'
	import { SchedulePollIcon } from '$lib/components/icons'

	// import type { Writable } from 'svelte/store'

	const dispatch = createEventDispatcher()
	export let stop = false
	export let index: number = 0
	export let funcDesc = ''
	export let modules: FlowModule[] = []
	export let disableAi = false
	export let kind: 'script' | 'trigger' | 'preprocessor' | 'failure' = 'script'
	export let allowTrigger = true
	export let iconSize = 12

	type Alignment = 'start' | 'end' | 'center'
	type Side = 'top' | 'bottom'
	type Placement = `${Side}-${Alignment}`

	export let placement: Placement = 'bottom-center'

	let floatingConfig: ComputeConfig = {
		strategy: 'fixed',
		// @ts-ignore
		placement,
		middleware: [offset(8), flip()],
		autoUpdate: true
	}
	$: !open && (funcDesc = '')
	let customUi: undefined | FlowBuilderWhitelabelCustomUi = getContext('customUi')
	let selectedKind: 'script' | 'trigger' | 'preprocessor' | 'approval' | 'flow' | 'failure' = kind
	let preFilter: 'all' | 'workspace' | 'hub' = 'all'
	let loading = false
	let small = false
	let open = false

	let width = 0
	let height = 0

	$: displayPath = width > 650 || height > 400

	$: small = kind === 'preprocessor' || kind === 'failure'
</script>

<!-- <Menu transitionDuration={0} pointerDown bind:show={open} noMinW {placement} let:close> -->

<!-- {floatingConfig}
floatingClasses="mt-2"
containerClasses="border rounded-lg shadow-lg  bg-surface"
noTransition
shouldUsePortal={true} -->

<PopupV2 {floatingConfig} bind:open let:close target="#flow-editor">
	<svelte:fragment let:pointerdown let:pointerup slot="button">
		<button
			title={`Add ${
				kind === 'failure'
					? ' failure module '
					: kind === 'preprocessor'
						? 'preprocessor step'
						: kind === 'trigger'
							? 'trigger'
							: 'step'
			}`}
			id={`flow-editor-add-step-${index}`}
			type="button"
			class={twMerge(
				'w-[17.5px] h-[17.5px] flex items-center justify-center !outline-[1px] outline dark:outline-gray-500 outline-gray-300 text-secondary bg-surface focus:outline-none hover:bg-surface-hover rounded',
				$$props.class
			)}
			on:pointerdown|preventDefault|stopPropagation={() => {
				dispatch('open')
				pointerdown()
			}}
			on:pointerup={pointerup}
		>
			{#if kind === 'trigger'}
				<SchedulePollIcon size={14} />
			{:else}
				<Cross size={iconSize} />
			{/if}
		</button>
	</svelte:fragment>
	<div
		id="flow-editor-insert-module"
		class="flex flex-col h-[400px] {small
			? 'w-[450px]'
			: 'w-[650px]'} pt-1 pr-1 pl-1 gap-1.5 resize overflow-auto {small
			? 'min-w-[450px]'
			: 'min-w-[650px]'} min-h-[400px]"
		on:wheel={(e) => {
			e.stopPropagation()
		}}
		role="none"
		bind:clientWidth={width}
		bind:clientHeight={height}
	>
		<div class="flex flex-row items-center gap-2">
			<StepGenQuick
				on:escape={() => close(null)}
				{disableAi}
				on:insert
				bind:funcDesc
				{preFilter}
				{loading}
			/>
			{#if selectedKind != 'preprocessor' && selectedKind != 'flow'}
				<ToggleHubWorkspaceQuick bind:selected={preFilter} />
			{/if}
		</div>

		<div class="flex flex-row grow min-h-0">
			{#if kind === 'script'}
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
							close(null)
							dispatch('new', { kind: 'forloop' })
						}}
					/>
					<TopLevelNode
						label="While loop"
						on:select={() => {
							close(null)
							dispatch('new', { kind: 'whileloop' })
						}}
					/>
					<TopLevelNode
						label="Branch to one"
						on:select={() => {
							close(null)
							dispatch('new', { kind: 'branchone' })
						}}
					/>
					<TopLevelNode
						label="Branch to all"
						on:select={() => {
							close(null)
							dispatch('new', { kind: 'branchall' })
						}}
					/>
				</div>
			{/if}

			<FlowInputsQuick
				{selectedKind}
				bind:loading
				filter={funcDesc}
				{modules}
				{index}
				{disableAi}
				{funcDesc}
				{kind}
				on:close={() => {
					close(null)
				}}
				on:new
				on:pickScript
				on:pickFlow
				{preFilter}
				{small}
				{displayPath}
			/>
		</div>
	</div>
</PopupV2>
