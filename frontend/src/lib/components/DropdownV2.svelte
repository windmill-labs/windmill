<script lang="ts">
	import { MoreVertical } from 'lucide-svelte'
	import Menu from './common/menu/MenuV2.svelte'
	import { MenuItem } from '@rgossiaux/svelte-headlessui'
	import { twMerge } from 'tailwind-merge'

	type Item = {
		displayName: string
		action?: (e: CustomEvent<any>) => void
		icon?: any
		href?: string
		disabled?: boolean
		type?: 'action' | 'delete'
	}

	export let items: Item[] | (() => Item[]) = []

	function computeItems(): Item[] {
		if (typeof items === 'function') {
			return items()
		} else {
			return items
		}
	}
</script>

<Menu placement="bottom-end" justifyEnd on:close on:open>
	<div slot="trigger">
		<MoreVertical size={16} class="w-8  h-8 p-2 hover:bg-surface-hover cursor-pointer rounded-md" />
	</div>

	<div class="flex flex-col">
		{#each computeItems() ?? [] as item}
			<MenuItem
				on:click={(e) => item?.action?.(e)}
				href={item?.href}
				disabled={item?.disabled}
				class={twMerge(
					'px-4 py-2 text-primary hover:bg-surface-hover hover:text-primary cursor-pointer text-xs transition-all',
					'flex flex-row gap-2 items-center',
					item?.type === 'delete' && 'text-red-500 hover:bg-red-100 hover:text-red-500'
				)}
			>
				{#if item.icon}
					<svelte:component this={item.icon} size={14} />
				{/if}
				{item.displayName}
			</MenuItem>
		{/each}
	</div>
</Menu>
