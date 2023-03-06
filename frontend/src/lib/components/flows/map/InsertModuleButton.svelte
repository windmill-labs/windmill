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

<Menu bind:show={open} noMinW placement="bottom-center" let:close>
	<button
		slot="trigger"
		type="button"
		class=" text-gray-900 bg-white border mx-0.5 border-gray-300 focus:outline-none hover:bg-gray-100 focus:ring-4 focus:ring-gray-200 font-medium rounded-full text-sm w-6 h-6 flex items-center justify-center"
	>
		<Cross size={12} />
	</button>
	<div class="font-mono divide-y divide-gray-100 text-xs w-40">
		<button
			class="w-full text-left p-2 hover:bg-gray-100"
			on:click={() => {
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
				class="w-full text-left p-2 hover:bg-gray-100"
				on:click={() => {
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
			class="w-full text-left gap-1 p-2 hover:bg-gray-100"
			on:click={() => {
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
			class="w-full inline-flex text-left p-2 hover:bg-gray-100"
			on:click={() => {
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
			class="w-full text-left p-2 hover:bg-gray-100"
			on:click={() => {
				close()
				dispatch('new', 'branchone')
			}}
			role="menuitem"
		>
			<Icon data={faCodeBranch} scale={0.8} class="mr-2" />
			Branch to one
		</button>

		<button
			class="w-full text-left p-2 hover:bg-gray-100"
			on:click={() => {
				close()
				dispatch('new', 'branchall')
			}}
			role="menuitem"
		>
			<Icon data={faCodeBranch} scale={0.8} class="mr-2" />
			Branch to all
		</button>

		<button
			class="w-full text-left p-2 hover:bg-gray-100"
			on:click={() => {
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
				class="w-full text-left p-2 hover:bg-gray-100 inline-flex gap-2.5"
				on:click={() => {
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
