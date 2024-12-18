<script lang="ts">
	import Popover from '$lib/components/meltComponents/Popover.svelte'

	export let dropdownItems: Array<{
		label: string
		onClick: () => void
		icon?: any
	}> = []
	export let closeOnClick: boolean = false
	export let placement: any = 'bottom'

	export let open: boolean = false
</script>

<Popover bind:open closeButton={false} {placement}>
	<svelte:fragment slot="trigger">
		<slot {open} />
	</svelte:fragment>
	<svelte:fragment slot="content">
		<div class="flex flex-col bg-surface">
			{#each dropdownItems as item}
				<button
					class="hover:bg-surface-hover p-2 transition-colors duration-150"
					on:click={() => {
						item.onClick()
						if (closeOnClick) {
							open = false
						}
					}}
				>
					<div class="flex flex-row items-center gap-2">
						<svelte:component this={item.icon} />
						<p class="text-xs text-secondary">{item.label}</p>
					</div>
				</button>
			{/each}
		</div>
	</svelte:fragment>
</Popover>
