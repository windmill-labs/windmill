<script lang="ts">
	import { MoreVertical } from 'lucide-svelte'
	import { Menu, Menubar, MeltButton } from '$lib/components/meltComponents'
	import type { Placement } from '@floating-ui/core'
	import type { Item } from '$lib/utils'
	import DropdownV2Inner from './DropdownV2Inner.svelte'

	export let items: Item[] | (() => Item[]) | (() => Promise<Item[]>) = []
	export let justifyEnd: boolean = true
	export let disabled = false
	export let placement: Placement = 'bottom-end'
	export let usePointerDownOutside = false

	async function computeItems(): Promise<Item[]> {
		if (typeof items === 'function') {
			const result = await items()
			return Array.isArray(result) ? result.filter((item) => !item.hide) : []
		} else {
			return items.filter((item) => !item.hide)
		}
	}
</script>

<Menubar let:createMenu>
	<Menu
		{createMenu}
		{placement}
		{justifyEnd}
		on:close
		on:open
		{disabled}
		let:item
		class={$$props.class}
		{usePointerDownOutside}
	>
		<svelte:fragment slot="trigger" let:trigger>
			<MeltButton meltElement={trigger} on:click={(e) => e.stopPropagation()}>
				{#if $$slots.buttonReplacement}
					<slot name="buttonReplacement" />
				{:else}
					<MoreVertical
						size={16}
						class="w-8 h-8 p-2 hover:bg-surface-hover cursor-pointer rounded-md"
					/>
				{/if}
			</MeltButton>
		</svelte:fragment>

		<DropdownV2Inner items={computeItems} meltItem={item} />
	</Menu>
</Menubar>
