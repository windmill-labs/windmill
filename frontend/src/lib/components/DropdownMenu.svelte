<script lang="ts">
	type Item = {
		label: string
		icon?: any
		onClick: () => void
	}
	type Props = {
		closeCallback?: () => void
		items: Item[]
	}
	let { items, closeCallback }: Props = $props()
</script>

<ul class="bg-surface-tertiary rounded-md border w-48 relative drop-shadow-base">
	{#each items as item}
		<li class="w-full">
			<button
				class="px-3 h-9 text-xs cursor-pointer hover:bg-surface-hover font-normal w-full text-left flex items-center gap-2.5"
				onclick={() => {
					item.onClick()
					closeCallback?.()
				}}
			>
				{#if item.icon}
					<item.icon size="16"></item.icon>
				{/if}
				{item.label}
			</button>
		</li>
	{/each}
</ul>
