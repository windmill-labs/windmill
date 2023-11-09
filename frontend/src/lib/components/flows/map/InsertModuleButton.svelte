<script lang="ts">
	import { Menu } from '$lib/components/common'
	import { createEventDispatcher } from 'svelte'
	import Icon from 'svelte-awesome'
	import { CheckCircle2, Code, Cross, GitBranch, Repeat, Square, Zap } from 'lucide-svelte'
	import StepGen from '$lib/components/copilot/StepGen.svelte'
	import type { FlowModule } from '$lib/gen'
	import { faBarsStaggered } from '@fortawesome/free-solid-svg-icons'

	const dispatch = createEventDispatcher()
	export let trigger = false
	export let stop = false
	export let open: boolean | undefined = undefined
	export let index: number
	export let funcDesc = ''
	export let modules: FlowModule[]
	export let disableAi = false

	$: !open && (funcDesc = '')
</script>

<Menu
	transitionDuration={0}
	pointerDown
	bind:show={open}
	noMinW
	placement="bottom-center"
	let:close
>
	<button
		title="Add step"
		slot="trigger"
		id={`flow-editor-add-step-${index}`}
		type="button"
		class="text-primary bg-surface border-[1px] mx-[1px] border-gray-300 dark:border-gray-500 focus:outline-none hover:bg-surface-hover focus:ring-4 focus:ring-surface-selected font-medium rounded-full text-sm w-[25px] h-[25px] flex items-center justify-center"
	>
		<Cross class="mx-[5px]" size={15} />
	</button>
	<div id="flow-editor-insert-module ">
		{#if !disableAi}
			<StepGen {index} bind:funcDesc bind:open {close} {modules} />
		{/if}

		{#if funcDesc.length === 0}
			<div class="font-mono divide-y text-xs w-full text-secondary">
				<button
					class="w-full text-left py-2 px-3 hover:bg-surface-hover whitespace-nowrap"
					on:pointerdown={() => {
						close()
						dispatch('new', 'script')
					}}
					role="menuitem"
					tabindex="-1"
				>
					<Code size={14} class="mr-2" />
					Action
				</button>
				{#if trigger}
					<button
						class="w-full text-left py-2 px-3 hover:bg-surface-hover whitespace-nowrap"
						on:pointerdown={() => {
							close()
							dispatch('new', 'trigger')
						}}
						role="menuitem"
						tabindex="-1"
					>
						<Zap size={14} class="mr-2" />
						Trigger
					</button>
				{/if}
				<button
					class="w-full text-left gap-1 py-2 px-3 hover:bg-surface-hover whitespace-nowrap"
					on:pointerdown={() => {
						close()
						dispatch('new', 'approval')
					}}
					role="menuitem"
					tabindex="-1"
				>
					<CheckCircle2 size={14} class="mr-2" />
					Approval
				</button>
				<button
					class="w-full inline-flex text-left py-2 px-3 hover:bg-surface-hover whitespace-nowrap"
					on:pointerdown={() => {
						close()
						dispatch('new', 'forloop')
					}}
					role="menuitem"
				>
					<span class="mr-3">
						<Repeat size={14} />
					</span>

					For Loop
				</button>

				<button
					class="w-full text-left py-2 px-3 hover:bg-surface-hover whitespace-nowrap"
					on:pointerdown={() => {
						close()
						dispatch('new', 'branchone')
					}}
					role="menuitem"
				>
					<GitBranch size={14} class="mr-2" />
					Branch to one
				</button>

				<button
					class="w-full text-left py-2 px-3 hover:bg-surface-hover whitespace-nowrap"
					on:pointerdown={() => {
						close()
						dispatch('new', 'branchall')
					}}
					role="menuitem"
				>
					<GitBranch size={14} class="mr-2" />

					Branch to all
				</button>

				<button
					class="w-full text-left py-2 px-3 hover:bg-surface-hover rounded-none whitespace-nowrap"
					on:pointerdown={() => {
						close()
						dispatch('new', 'flow')
					}}
					role="menuitem"
				>
					<Icon data={faBarsStaggered} scale={0.8} class="mr-2" />
					Flow
				</button>
				{#if stop}
					<button
						class="w-full text-left py-2 px-3 hover:bg-surface-hover inline-flex gap-2.5"
						on:pointerdown={() => {
							close()
							dispatch('new', 'end')
						}}
						role="menuitem"
					>
						<Square size={14} />
						End Flow
					</button>
				{/if}
			</div>
		{/if}
	</div>
</Menu>
