<script lang="ts">
	import { isNewTabClick } from '$lib/navigation'

	type Item = {
		label: string
		icon?: any
		right?: string
		onClick?: () => void
		/** Renders the row as a link, so it opens in a new tab on middle/ctrl-click. */
		href?: string
		onHover?: (hover: boolean) => void
	}
	export type Props = {
		closeCallback?: () => void
		items: Item[]
	}
	let { items, closeCallback }: Props = $props()

	const rowClass =
		'px-3 h-9 text-xs cursor-pointer hover:bg-surface-hover font-normal w-full text-left flex items-center gap-2.5'

	function onRowClick(item: Item, e: MouseEvent) {
		if (!(item.href && isNewTabClick(e))) item.onClick?.()
		item.onHover?.(false)
		closeCallback?.()
	}
</script>

{#snippet rowContent(item: Item)}
	{#if item.icon}
		<item.icon size="16"></item.icon>
	{/if}
	<span class="flex-1">
		{item.label}
	</span>
	{#if item.right}
		<span class="text-xs text-hint">{item.right}</span>
	{/if}
{/snippet}

<ul class="bg-surface-tertiary rounded-md border w-56 relative drop-shadow-base">
	{#each items as item}
		<li class="w-full">
			{#if item.href}
				<a
					href={item.href}
					class="{rowClass} text-[color:inherit]"
					onclick={(e) => onRowClick(item, e)}
					onauxclick={(e) => {
						if (e.button === 1) onRowClick(item, e)
					}}
					onmouseenter={() => item.onHover?.(true)}
					onmouseleave={() => item.onHover?.(false)}
				>
					{@render rowContent(item)}
				</a>
			{:else}
				<button
					class={rowClass}
					onclick={(e) => onRowClick(item, e)}
					onmouseenter={() => item.onHover?.(true)}
					onmouseleave={() => item.onHover?.(false)}
				>
					{@render rowContent(item)}
				</button>
			{/if}
		</li>
	{/each}
	{#if items.length === 0}
		<li class="w-full">
			<div
				class="px-3 h-9 text-xs font-normal w-full text-left flex items-center gap-2.5 text-hint"
			>
				No actions available
			</div>
		</li>
	{/if}
</ul>
