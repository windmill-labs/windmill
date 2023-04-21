<script lang="ts">
	import { fly } from 'svelte/transition'
	import { faChevronLeft } from '@fortawesome/free-solid-svg-icons'
	import { Badge, Button } from '../../../../common'
	import { secondaryMenu, SECONDARY_MENU_ID } from './'
	import { getContext } from 'svelte'
	import type { AppViewerContext } from '../../../types'

	const { selectedComponent } = getContext<AppViewerContext>('AppViewerContext')
	let width: number
	let lastSelected = $selectedComponent

	$: if (lastSelected !== $selectedComponent) {
		secondaryMenu.close()
		lastSelected = $selectedComponent
	}
</script>

<!-- z-index must be above the split pane handles' z-index (which is 1001 atm.) -->
<div
	bind:clientWidth={width}
	class="absolute z-[1002] inset-0 overflow-hidden"
	class:pointer-events-none={!$secondaryMenu.isOpen}
>
	{#if $secondaryMenu.isOpen && $secondaryMenu.component}
		<div
			transition:fly|local={{ duration: 300, x: width, y: 0, opacity: 1 }}
			id={SECONDARY_MENU_ID}
			class="flex flex-col w-full h-full bg-white"
		>
			<div class="flex justify-between items-center bg-white gap-1 px-3 py-2">
				<Button
					color="light"
					size="xs2"
					spacingSize="xs"
					variant="border"
					startIcon={{ icon: faChevronLeft }}
					on:click={secondaryMenu.close}
				>
					Back
				</Button>
				<Badge color="blue">{$selectedComponent}</Badge>
			</div>
			<div class="relative h-full overflow-y-auto">
				{#if typeof $secondaryMenu.component === 'string'}
					{@html $secondaryMenu.component}
				{:else}
					<svelte:component this={$secondaryMenu.component} {...$secondaryMenu.props} />
				{/if}
			</div>
		</div>
	{/if}
</div>
