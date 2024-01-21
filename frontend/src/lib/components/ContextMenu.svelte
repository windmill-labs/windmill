<script lang="ts">
	import Portal from 'svelte-portal'
	import { twMerge } from 'tailwind-merge'

	let contextMenuVisible = false
	let menuX = 0
	let menuY = 0

	type ContextMenu = {
		menuItems: {
			label: string
			onClick: () => void
			color?: string
			icon: any
			shortcut?: string
		}[]
	}

	function handleRightClick(event: MouseEvent) {
		event.preventDefault()
		contextMenuVisible = true
		menuX = event.clientX
		menuY = event.clientY
	}

	function closeContextMenu() {
		contextMenuVisible = false
	}

	export let contextMenu: ContextMenu = { menuItems: [] }
</script>

<!-- svelte-ignore a11y-click-events-have-key-events -->
<!-- svelte-ignore a11y-no-static-element-interactions -->
<div on:click={closeContextMenu}>
	<!-- svelte-ignore a11y-no-static-element-interactions -->
	<div on:contextmenu={handleRightClick}>
		<slot />
	</div>

	{#if contextMenuVisible}
		<Portal>
			<div style="position: fixed; top: {menuY}px; left: {menuX}px; z-index:6000;">
				<div class="rounded-md bg-surface border shadow-md divide-y w-64">
					<div class="p-1">
						<!-- svelte-ignore a11y-click-events-have-key-events -->
						{#each contextMenu.menuItems as item}
							<!-- svelte-ignore a11y-no-static-element-interactions -->
							<!-- svelte-ignore a11y-click-events-have-key-events -->
							<div
								class={twMerge(
									'flex items-center p-2 hover:bg-surface-hover cursor-pointer transition-all round',
									item.color === 'red' && 'text-red-500',
									item.color === 'green' && 'text-green-500',
									item.color === 'blue' && 'text-blue-500'
								)}
								on:click={item.onClick}
							>
								<!-- svelte-ignore missing-declaration -->
								<svelte:component this={item.icon} class="w-4 h-4" />

								<span class="ml-2 text-xs">{item.label}</span>
								{#if item.shortcut}
									<span class="ml-auto text-xs text-gray-400">
										{item.shortcut}
									</span>
								{/if}
							</div>
						{/each}
					</div>
				</div>
			</div>
		</Portal>
	{/if}
</div>

<style>
</style>
