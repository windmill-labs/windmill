<script lang="ts">
	import { MoreVertical } from 'lucide-svelte'
	import Menu from './common/menu/MenuV2.svelte'

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

	async function computeItems(): Promise<Item[]> {
		if (typeof items === 'function') {
			return ((await items()) ?? []).filter((item) => !item.hide)
		} else {
			return items.filter((item) => !item.hide)
		}
	}
</script>

<Menu placement="bottom-end" {justifyEnd} on:close on:open>
	<div slot="trigger">
		{#if $$slots.buttonReplacement}
			<slot name="buttonReplacement" />
		{:else}
			<MoreVertical
				size={16}
				class="w-8  h-8 p-2 hover:bg-surface-hover cursor-pointer rounded-md"
			/>
		{/if}
	</div>

	<DropdownV2Inner items={computeItems} />
</Menu>
