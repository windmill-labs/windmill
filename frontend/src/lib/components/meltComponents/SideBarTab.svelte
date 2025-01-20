<script lang="ts">
	export let dropdownItems: Array<{
		label: string
		onClick: () => void
		icon?: any
		selected?: boolean
	}> = []

	export let fullMenu: boolean = false

	let open = false
	let timeout: NodeJS.Timeout | null = null

	function handleMouseEnter() {
		if (!fullMenu) return
		timeout = setTimeout(() => {
			open = true
		}, 500)
	}

	function handleMouseLeave() {
		if (timeout) {
			clearTimeout(timeout)
			timeout = null
		}
		open = false
	}
</script>

<!-- svelte-ignore a11y-no-static-element-interactions -->
<div
	class="flex flex-col relative !overflow-visible"
	on:mouseenter={handleMouseEnter}
	on:mouseleave={handleMouseLeave}
>
	<slot name="close button" />

	{#if fullMenu}
		<div
			class="absolute flex-col top-[30px] left-0 z-50 bg-surface border-l border-b {open
				? 'rounded-md rounded-tl-none overflow-hidden'
				: 'rounded-bl-md'}"
		>
			{#each dropdownItems as item}
				<button
					class="hover:bg-surface-hover p-2 transition-colors duration-150 w-full {item.selected
						? 'bg-surface-selected'
						: 'text-secondary'}"
					on:click={() => {
						item.onClick()
						open = false
					}}
				>
					<div class="flex flex-row items-center gap-2 min-h-[20px]">
						{#if item.icon}
							<svelte:component this={item.icon} size={14} />
						{/if}

						{#if open}
							<p class="text-xs text-secondary whitespace-nowrap text-left">
								{item.label}
							</p>
						{/if}
					</div>
				</button>
			{/each}
		</div>
	{/if}
</div>
