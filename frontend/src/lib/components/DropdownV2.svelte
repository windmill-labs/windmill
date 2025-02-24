<script lang="ts">
	import { MoreVertical } from 'lucide-svelte'
	import type { Placement } from '@floating-ui/core'
	import type { Item } from '$lib/utils'
	import DropdownV2Inner from './DropdownV2Inner.svelte'
	import { pointerDownOutside } from '$lib/utils'
	import { zIndexes } from '$lib/zIndexes'
	import { createDropdownMenu, melt, createSync } from '@melt-ui/svelte'
	import ResolveOpen from '$lib/components/common/menu/ResolveOpen.svelte'

	export let items: Item[] | (() => Item[]) | (() => Promise<Item[]>) = []
	export let disabled = false
	export let placement: Placement = 'bottom-end'
	export let usePointerDownOutside = false

	const {
		elements: { menu, item, trigger },
		states
	} = createDropdownMenu({
		positioning: {
			placement
		},
		loop: true
	})

	const zIndex = zIndexes.contextMenu

	let open = false
	const sync = createSync(states)
	$: sync.open(open, (v) => (open = Boolean(v)))

	export function close() {
		open = false
	}

	async function computeItems(): Promise<Item[]> {
		if (typeof items === 'function') {
			const result = await items()
			return Array.isArray(result) ? result.filter((item) => !item.hide) : []
		} else {
			return items.filter((item) => !item.hide)
		}
	}
	async function getMenuElements(): Promise<HTMLElement[]> {
		return Array.from(document.querySelectorAll('[data-menu]')) as HTMLElement[]
	}
</script>

<ResolveOpen {open} on:open on:close />

<button
	use:melt={$trigger}
	class={$$props.class}
	{disabled}
	on:click={(e) => e.stopPropagation()}
	use:pointerDownOutside={{
		capture: true,
		stopPropagation: false,
		exclude: getMenuElements,
		customEventName: 'pointerdown_menu'
	}}
	on:pointerdown_outside={() => {
		if (usePointerDownOutside) {
			close()
		}
	}}
	data-menu
>
	{#if $$slots.buttonReplacement}
		<slot name="buttonReplacement" />
	{:else}
		<MoreVertical size={16} class="w-8 h-8 p-2 hover:bg-surface-hover cursor-pointer rounded-md" />
	{/if}
</button>

<div use:melt={$menu} data-menu class={`z-[${zIndex}]`}>
	<div
		class="bg-surface border w-56 origin-top-right rounded-md shadow-md focus:outline-none overflow-y-auto py-1 max-h-[50vh]"
	>
		<DropdownV2Inner items={computeItems} meltItem={item} />
	</div>
</div>
