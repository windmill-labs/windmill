<script lang="ts">
	import { twMerge } from 'tailwind-merge'

	export let dropdownItems: Array<{
		label: string
		onClick: () => void
		icon?: any
		selected?: boolean
	}> = []

	export let fullMenu: boolean = false
	export let noTrigger: boolean = false
	export const expandRight: boolean = false

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

	let hasCloseButton = $$slots['close button']
</script>

<!-- svelte-ignore a11y-no-static-element-interactions -->
<div
	class="flex flex-col relative !overflow-visible w-[33px]"
	on:mouseenter={handleMouseEnter}
	on:mouseleave={handleMouseLeave}
>
	{#if hasCloseButton}
		<slot name="close button" />
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
