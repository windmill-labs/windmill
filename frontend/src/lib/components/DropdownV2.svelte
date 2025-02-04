<script lang="ts">
	import { MoreVertical } from 'lucide-svelte'
	import { Menu, Menubar } from '$lib/components/meltComponents'
	import { melt } from '@melt-ui/svelte'
	import type { Placement } from '@floating-ui/core'

	import DropdownV2Inner from './DropdownV2Inner.svelte'

	type Item = {
		displayName: string
		action?: (e: CustomEvent<any>) => void
		icon?: any
		href?: string
		disabled?: boolean
		type?: 'action' | 'delete'
		hide?: boolean | undefined
	}

	export let items: Item[] | (() => Item[]) | (() => Promise<Item[]>) = []
	export let justifyEnd: boolean = true
	export let disabled = false
	export let placement: Placement = 'bottom-end'

	async function computeItems(): Promise<Item[]> {
		if (typeof items === 'function') {
			return ((await items()) ?? []).filter((item) => !item.hide)
		} else {
			return items.filter((item) => !item.hide)
		}
	}
</script>

<Menubar let:createMenu>
	<Menu {createMenu} {placement} {justifyEnd} on:close on:open {disabled} let:item>
		<svelte:fragment slot="trigger" let:trigger>
			<div use:melt={trigger}>
				{#if $$slots.buttonReplacement}
					<slot name="buttonReplacement" />
				{:else}
					<MoreVertical
						size={16}
						class="w-8  h-8 p-2 hover:bg-surface-hover cursor-pointer rounded-md"
					/>
				{/if}
			</div>
		</svelte:fragment>

		<DropdownV2Inner items={computeItems} meltItem={item} />
	</Menu>
</Menubar>
