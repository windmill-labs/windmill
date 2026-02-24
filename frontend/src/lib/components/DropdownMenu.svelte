<script lang="ts">
	type Item = {
		label: string
		icon?: any
		right?: string
		onClick?: () => void
		onHover?: (hover: boolean) => void
	}
	export type Props = {
		closeCallback?: () => void
		items: Item[]
	}
	let { items, closeCallback }: Props = $props()
</script>

<ul class="bg-surface-tertiary rounded-md border w-56 relative drop-shadow-base">
	{#each items as item}
		<li class="w-full">
			<button
				class="px-3 h-9 text-xs cursor-pointer hover:bg-surface-hover font-normal w-full text-left flex items-center gap-2.5"
				onclick={() => {
					item.onClick?.()
					item.onHover?.(false)
					closeCallback?.()
				}}
				onmouseenter={() => item.onHover?.(true)}
				onmouseleave={() => item.onHover?.(false)}
			>
				{#if item.icon}
					<item.icon size="16"></item.icon>
				{/if}
				<span class="flex-1">
					{item.label}
				</span>
				{#if item.right}
					<span class="text-xs text-hint">{item.right}</span>
				{/if}
			</button>
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
