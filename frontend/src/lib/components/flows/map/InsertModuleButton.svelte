<script lang="ts">
	import { Menu } from '$lib/components/common'
	import {
		faCode,
		faCodeBranch,
		faBarsStaggered,
		faBolt,
		faCheck
	} from '@fortawesome/free-solid-svg-icons'
	import { createEventDispatcher } from 'svelte'
	import Icon from 'svelte-awesome'
	import { Cross, Repeat, Square } from 'lucide-svelte'

	const dispatch = createEventDispatcher()
	export let trigger = false
	export let stop = false
	export let open: boolean | undefined = undefined
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
		type="button"
		class="text-primary bg-surface border mx-0.5 focus:outline-none hover:bg-surface-hover focus:ring-4 focus:ring-surface-selected font-medium rounded-full text-sm w-6 h-6 flex items-center justify-center"
	>
		<Cross size={12} />
	</button>
	<div class="font-mono divide-y text-xs w-40 text-secondary">
		<button
			class="w-full text-left p-2 hover:bg-surface-hover"
			on:pointerdown={() => {
				close()
				dispatch('new', 'script')
			}}
			role="menuitem"
			tabindex="-1"
		>
			<Icon data={faCode} scale={0.8} class="mr-2" />
			Action
		</button>
		{#if trigger}
			<button
				class="w-full text-left p-2 hover:bg-surface-hover"
				on:pointerdown={() => {
					close()
					dispatch('new', 'trigger')
				}}
				role="menuitem"
				tabindex="-1"
			>
				<Icon data={faBolt} scale={0.8} class="mr-2" />
				Trigger
			</button>
		{/if}
		<button
			class="w-full text-left gap-1 p-2 hover:bg-surface-hover"
			on:pointerdown={() => {
				close()
				dispatch('new', 'approval')
			}}
			role="menuitem"
			tabindex="-1"
		>
			<Icon data={faCheck} class="mr-1.5" scale={0.8} />
			Approval
		</button>
		<button
			class="w-full inline-flex text-left p-2 hover:bg-surface-hover"
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
			class="w-full text-left p-2 hover:bg-surface-hover"
			on:pointerdown={() => {
				close()
				dispatch('new', 'branchone')
			}}
			role="menuitem"
		>
			<Icon data={faCodeBranch} scale={0.8} class="mr-2" />
			Branch to one
		</button>

		<button
			class="w-full text-left p-2 hover:bg-surface-hover"
			on:pointerdown={() => {
				close()
				dispatch('new', 'branchall')
			}}
			role="menuitem"
		>
			<Icon data={faCodeBranch} scale={0.8} class="mr-2" />
			Branch to all
		</button>

		<button
			class="w-full text-left p-2 hover:bg-surface-hover"
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
				class="w-full text-left p-2 hover:bg-surface-hover inline-flex gap-2.5"
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
</Menu>
