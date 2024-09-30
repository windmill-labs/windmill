<script lang="ts">
	import { Popup } from '$lib/components/common'
	import { createEventDispatcher, getContext } from 'svelte'
	import {
		CheckCircle2,
		Code,
		Cross,
		GitBranch,
		Repeat,
		Square,
		Zap,
		ChevronRight
	} from 'lucide-svelte'
	import StepGenQuick from '$lib/components/copilot/StepGenQuick.svelte'
	import FlowInputsQuick from '../content/FlowInputsQuick.svelte'
	import type { FlowModule } from '$lib/gen'
	import BarsStaggered from '$lib/components/icons/BarsStaggered.svelte'
	import type { FlowBuilderWhitelabelCustomUi } from '$lib/components/custom_ui'
	import ToggleHubWorkspaceQuick from '$lib/components/ToggleHubWorkspaceQuick.svelte'
	import { twMerge } from 'tailwind-merge'
	import type { FlowCopilotModule } from '../../copilot/flow'
	import type { ComputeConfig } from 'svelte-floating-ui'

	// import type { Writable } from 'svelte/store'

	const dispatch = createEventDispatcher()
	export let stop = false
	export let index: number = 0
	export let funcDesc = ''
	export let modules: FlowModule[] = []
	export let disableAi = false
	let hubCompletions: FlowCopilotModule['hubCompletions'] = []
	export let kind: 'script' | 'trigger' | 'preprocessor' | 'failure' = 'script'
	export let allowTrigger = true

	type Alignment = 'start' | 'end' | 'center'
	type Side = 'top' | 'bottom'
	type Placement = `${Side}-${Alignment}`

	export let placement: Placement = 'bottom-center'

	let floatingConfig: ComputeConfig = {
		strategy: 'absolute',
		// @ts-ignore
		placement
	}
	$: !open && (funcDesc = '')
	let customUi: undefined | FlowBuilderWhitelabelCustomUi = getContext('customUi')
	let selectedKind: 'script' | 'trigger' | 'preprocessor' | 'approval' | 'flow' | 'failure' = kind
	let preFilter: 'all' | 'workspace' | 'hub' = 'all'
	let loading = false
	let small = false

	$: small = kind === 'preprocessor' || kind === 'failure'
</script>

<!-- <Menu transitionDuration={0} pointerDown bind:show={open} noMinW {placement} let:close> -->

<Popup
	let:close
	{floatingConfig}
	floatingClasses="mt-2"
	containerClasses="border rounded-lg shadow-lg  bg-surface"
	noTransition
>
	<svelte:fragment slot="button">
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
				'w-5 h-5 flex items-center justify-center',
				'outline-[1px] outline dark:outline-gray-500 outline-gray-300',
				'text-secondary',
				'bg-surface focus:outline-none hover:bg-surface-hover rounded '
			)}
		>
			{#if kind === 'trigger'}
				<Zap size={12} />
			{:else}
				<Cross size={12} />
			{/if}
		</button>
	</svelte:fragment>
	<div
		id="flow-editor-insert-module"
		class="flex flex-col h-[400px] {small ? 'w-[450px]' : 'w-[650px]'}  pt-1 pr-1 pl-1 gap-1.5"
		on:wheel={(e) => {
			e.stopPropagation()
		}}
		role="none"
	>
		<div class="flex flex-row items-center gap-2">
			<StepGenQuick on:insert bind:funcDesc bind:hubCompletions {loading} />
			{#if selectedKind != 'preprocessor' && selectedKind != 'flow'}
				<ToggleHubWorkspaceQuick bind:selected={preFilter} />
			{/if}
		</div>

		<div class="flex flex-row grow min-h-0">
			{#if kind === 'script'}
				<div class="flex-none flex flex-col text-xs text-primary">
					<button
						class={twMerge(
							'w-full text-left py-2 px-1.5 hover:bg-surface-hover font-medium transition-all whitespace-nowrap flex flex-row gap-2 items-center rounded-md',
							selectedKind === 'script' ? 'bg-surface-hover' : ''
						)}
						on:click={() => {
							selectedKind = 'script'
						}}
						role="menuitem"
						tabindex="-1"
					>
						<Code size={14} />
						Action
						<ChevronRight size={12} class="ml-auto" color="#4c566a" />
					</button>
					{#if customUi?.triggers != false && allowTrigger}
						<button
							class={twMerge(
								'w-full text-left py-2 px-1.5 hover:bg-surface-hover font-medium transition-all whitespace-nowrap flex flex-row gap-2 items-center rounded-md',
								selectedKind === 'trigger' ? 'bg-surface-hover' : ''
							)}
							on:click={() => {
								selectedKind = 'trigger'
							}}
							role="menuitem"
							tabindex="-1"
						>
							<Zap size={14} />
							Trigger
							<ChevronRight size={12} class="ml-auto" color="#4c566a" />
						</button>
					{/if}
					<button
						class={twMerge(
							'w-full text-left py-2 px-1.5 hover:bg-surface-hover font-medium transition-all whitespace-nowrap flex flex-row gap-2 items-center rounded-md',
							selectedKind === 'approval' ? 'bg-surface-hover' : ''
						)}
						on:click={() => {
							selectedKind = 'approval'
						}}
						role="menuitem"
						tabindex="-1"
					>
						<CheckCircle2 size={14} />
						Approval/Prompt
						<ChevronRight size={12} class="ml-auto" color="#4c566a" />
					</button>

					{#if customUi?.flowNode != false}
						<button
							class={twMerge(
								'w-full text-left py-2 px-1.5 hover:bg-surface-hover font-medium transition-all whitespace-nowrap flex flex-row gap-2 items-center rounded-md',
								selectedKind === 'flow' ? 'bg-surface-hover' : ''
							)}
							on:click={() => {
								selectedKind = 'flow'
							}}
							role="menuitem"
						>
							<BarsStaggered size={14} />
							Flow
							<ChevronRight size={12} class="ml-auto" color="#4c566a" />
						</button>
					{/if}
					{#if stop}
						<button
							class="w-full text-left py-2 px-1.5 hover:bg-surface-hover font-medium transition-all whitespace-nowrap flex flex-row gap-2 items-center rounded-md"
							on:pointerdown={() => {
								close(null)
								dispatch('new', { kind: 'end' })
							}}
							role="menuitem"
						>
							<Square size={14} />
							End Flow
						</button>
					{/if}
					<button
						class="w-full text-left py-2 px-1.5 hover:bg-surface-hover font-medium transition-all whitespace-nowrap flex flex-row gap-2 items-center rounded-md"
						on:pointerdown={() => {
							close(null)
							dispatch('new', { kind: 'forloop' })
						}}
						role="menuitem"
					>
						<Repeat size={14} />

						For Loop
					</button>
					<button
						class="w-full text-left py-2 px-1.5 hover:bg-surface-hover font-medium transition-all whitespace-nowrap flex flex-row gap-2 items-center rounded-md"
						on:pointerdown={() => {
							close(null)
							dispatch('new', { kind: 'whileloop' })
						}}
						role="menuitem"
					>
						<Repeat size={14} />

						While Loop
					</button>

					<button
						class="w-full text-left py-2 px-1.5 hover:bg-surface-hover font-medium transition-all whitespace-nowrap flex flex-row gap-2 items-center rounded-md"
						on:pointerdown={() => {
							close(null)
							dispatch('new', { kind: 'branchone' })
						}}
						role="menuitem"
					>
						<GitBranch size={14} />
						Branch to one
					</button>

					<button
						class="w-full text-left py-2 px-1.5 hover:bg-surface-hover font-medium transition-all whitespace-nowrap flex flex-row gap-2 items-center rounded-md"
						on:pointerdown={() => {
							close(null)
							dispatch('new', { kind: 'branchall' })
						}}
						role="menuitem"
					>
						<GitBranch size={14} />

						Branch to all
					</button>
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
			/>
		</div>
	</div>
</Popup>
