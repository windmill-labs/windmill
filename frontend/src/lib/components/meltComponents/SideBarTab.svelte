<script lang="ts">
	import { twMerge } from 'tailwind-merge'

	interface Props {
		dropdownItems?: Array<{
			label: string
			onClick: () => void
			icon?: any
			selected?: boolean
		}>
		fullMenu?: boolean
		noTrigger?: boolean
		close_button?: import('svelte').Snippet
	}

	let { dropdownItems = [], fullMenu = false, noTrigger = false, close_button }: Props = $props()

	let open = $state(false)
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

	let hasCloseButton = close_button
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
	class="flex flex-col relative !overflow-visible w-[33px]"
	onmouseenter={handleMouseEnter}
	onmouseleave={handleMouseLeave}
>
	{#if hasCloseButton}
		{@render close_button?.()}
	{:else}
		<div class="w-[31px]"></div>
	{/if}

	{#if fullMenu}
		<div
			class={twMerge(
				'absolute flex-col left-0 z-50 bg-surface border-l border-b overflow-hidden',
				hasCloseButton ? 'top-[30px]' : 'top-[1px]',
				open ? 'rounded-md rounded-tl-none shadow-md' : 'rounded-bl-md',
				hasCloseButton ? '' : 'rounded-tl-md border-t',
				noTrigger && !dropdownItems.some((item) => item.selected) ? 'rounded-md border' : ''
			)}
		>
			{#each dropdownItems as item}
				<button
					class="hover:bg-surface-hover p-2 transition-colors duration-0 w-full {item.selected
						? 'bg-surface-selected'
						: 'text-secondary'}"
					onclick={() => {
						item.onClick()
						open = false
					}}
				>
					<div class="flex flex-row items-center gap-2 min-h-[20px]">
						{#if item.icon}
							<item.icon size={14} />
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
