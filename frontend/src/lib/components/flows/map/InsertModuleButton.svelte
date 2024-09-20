<script lang="ts">
	import { Menu } from '$lib/components/common'
	import { createEventDispatcher, getContext } from 'svelte'
	import { CheckCircle2, Code, Cross, GitBranch, Repeat, Square, Zap } from 'lucide-svelte'
	import StepGenQuick from '$lib/components/copilot/StepGenQuick.svelte'
	import FlowInputsQuick from '../content/FlowInputsQuick.svelte'
	import type { FlowModule, Script } from '$lib/gen'
	import BarsStaggered from '$lib/components/icons/BarsStaggered.svelte'
	import type { FlowBuilderWhitelabelCustomUi } from '$lib/components/custom_ui'
	import ToggleHubWorkspaceQuick from '$lib/components/ToggleHubWorkspaceQuick.svelte'
	import { twMerge } from 'tailwind-merge'
	import type { FlowCopilotModule } from '../../copilot/flow'
	import type { FlowEditorContext } from '../types'

	const dispatch = createEventDispatcher()
	export let trigger = false
	export let stop = false
	export let open: boolean | undefined = undefined
	export let index: number
	export let funcDesc = ''
	let filteredItems: (Script & { marked?: string })[] = []
	export let modules: FlowModule[]
	export let disableAi = false
	let hubCompletions: FlowCopilotModule['hubCompletions'] = []

	// export let failureModule: boolean

	const { selectedId } = getContext<FlowEditorContext>('FlowEditorContext')

	$: !open && (funcDesc = '')
	let customUi: undefined | FlowBuilderWhitelabelCustomUi = getContext('customUi')
	let selectedKind: 'action' | 'trigger' | 'approval' | 'flow' = 'action'
	let preFilter: 'all' | 'workspace' | 'hub' = 'all'
</script>

<Menu
	transitionDuration={0}
	pointerDown
	bind:show={open}
	noMinW
	placement="bottom-center"
	let:close
>
	<svelte:fragment slot="trigger">
		<button
			title="Add step"
			id={`flow-editor-add-step-${index}`}
			type="button"
			class={twMerge(
				'w-5 h-5 flex items-center justify-center',
				'outline-[1px] outline dark:outline-gray-500 outline-gray-300',
				'text-secondary',
				'bg-surface focus:outline-none hover:bg-surface-hover   rounded '
			)}
		>
			<Cross size={12} />
		</button>
	</svelte:fragment>
	<div id="flow-editor-insert-module" class="flex flex-col h-[400px] w-[650px]">
		<div class="flex flex-row items-center gap-2 px-2"
			><StepGenQuick on:insert bind:funcDesc bind:hubCompletions />
			<ToggleHubWorkspaceQuick bind:selected={preFilter} /></div
		>

		<div class="flex flex-row flex-grow min-h-0 w-full divide-x">
			<div class="flex-none font-mono text-xs w-40 flex flex-col text-secondary">
				<button
					class={twMerge(
						'w-full text-left py-2 px-3 hover:bg-surface-hover transition-all whitespace-nowrap flex flex-row gap-2 items-center',
						selectedKind === 'action' ? 'bg-surface-hover' : ''
					)}
					on:click={() => {
						selectedKind = 'action'
					}}
					role="menuitem"
					tabindex="-1"
				>
					<Code size={14} />
					Action
				</button>
				{#if customUi?.triggers != false && trigger}
					<button
						class={twMerge(
							'w-full text-left py-2 px-3 hover:bg-surface-hover transition-all whitespace-nowrap flex flex-row gap-2 items-center',
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
					</button>
				{/if}
				<button
					class={twMerge(
						'w-full text-left py-2 px-3 hover:bg-surface-hover transition-all whitespace-nowrap flex flex-row gap-2 items-center',
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
				</button>

				{#if customUi?.flowNode != false}
					<button
						class={twMerge(
							'w-full text-left py-2 px-3 hover:bg-surface-hover transition-all whitespace-nowrap flex flex-row gap-2 items-center',
							selectedKind === 'flow' ? 'bg-surface-hover' : ''
						)}
						on:click={() => {
							selectedKind = 'flow'
						}}
						role="menuitem"
					>
						<BarsStaggered size={14} />
						Flow
					</button>
				{/if}
				{#if stop}
					<button
						class="w-full text-left py-2 px-3 hover:bg-surface-hover transition-all whitespace-nowrap flex flex-row gap-2 items-center"
						on:pointerdown={() => {
							close()
							dispatch('new', { kind: 'end' })
						}}
						role="menuitem"
					>
						<Square size={14} />
						End Flow
					</button>
				{/if}
				<button
					class="w-full text-left py-2 px-3 hover:bg-surface-hover transition-all whitespace-nowrap flex flex-row gap-2 items-center"
					on:pointerdown={() => {
						close()
						dispatch('new', { kind: 'forloop' })
					}}
					role="menuitem"
				>
					<Repeat size={14} />

					For Loop
				</button>
				<button
					class="w-full text-left py-2 px-3 hover:bg-surface-hover transition-all whitespace-nowrap flex flex-row gap-2 items-center"
					on:pointerdown={() => {
						close()
						dispatch('new', { kind: 'whileloop' })
					}}
					role="menuitem"
				>
					<Repeat size={14} />

					While Loop
				</button>

				<button
					class="w-full text-left py-2 px-3 hover:bg-surface-hover transition-all whitespace-nowrap flex flex-row gap-2 items-center"
					on:pointerdown={() => {
						close()
						dispatch('new', { kind: 'branchone' })
					}}
					role="menuitem"
				>
					<GitBranch size={14} />
					Branch to one
				</button>

				<button
					class="w-full text-left py-2 px-3 hover:bg-surface-hover transition-all whitespace-nowrap flex flex-row gap-2 items-center"
					on:pointerdown={() => {
						close()
						dispatch('new', { kind: 'branchall' })
					}}
					role="menuitem"
				>
					<GitBranch size={14} />

					Branch to all
				</button>
			</div>

			<FlowInputsQuick
				filter={funcDesc}
				{modules}
				{index}
				{hubCompletions}
				{disableAi}
				{filteredItems}
				{funcDesc}
				{selectedKind}
				failureModule={$selectedId === 'failure'}
				on:new
				on:pickScript
				on:pickFlow
				{preFilter}
			/>
		</div>
	</div>
</Menu>
