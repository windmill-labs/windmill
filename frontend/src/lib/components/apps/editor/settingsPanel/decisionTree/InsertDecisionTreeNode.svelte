<script lang="ts">
	import { Menu } from '$lib/components/common'
	import { createEventDispatcher } from 'svelte'
	import { ArrowDown, Cross, Split } from 'lucide-svelte'

	export let open: boolean | undefined = undefined

	const dispatch = createEventDispatcher()
</script>

<Menu
	transitionDuration={0}
	pointerDown
	noMinW
	placement="bottom-center"
	let:close
	bind:show={open}
>
	<button
		title="Add step"
		slot="trigger"
		type="button"
		class="text-primary bg-surface border-[1px] mx-[1px] border-gray-300 dark:border-gray-500 focus:outline-none hover:bg-surface-hover focus:ring-4 focus:ring-surface-selected font-medium rounded-full text-sm w-[25px] h-[25px] flex items-center justify-center"
	>
		<Cross class="mx-[5px]" size={15} />
	</button>
	<div id="flow-editor-insert-module">
		<button
			class="w-full text-left py-2 px-3 hover:bg-surface-hover whitespace-nowrap flex flex-row gap-2 items-center"
			on:pointerdown={() => {
				//close()

				debugger
				dispatch('node')
			}}
			role="menuitem"
			tabindex="-1"
		>
			<ArrowDown size={14} />
			Node
		</button>
		<button
			class="w-full text-left py-2 px-3 hover:bg-surface-hover whitespace-nowrap flex flex-row gap-2 items-center"
			on:pointerdown={() => {
				close()
				dispatch('branch')
			}}
			role="menuitem"
			tabindex="-1"
		>
			<Split size={14} class="rotate-180" />
			Branche
		</button>
	</div>
</Menu>
