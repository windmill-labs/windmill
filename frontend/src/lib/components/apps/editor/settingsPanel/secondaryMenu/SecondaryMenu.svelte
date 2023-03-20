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

<div
	bind:clientWidth={width}
	class="absolute z-50 inset-0 overflow-hidden"
	class:pointer-events-none={!$secondaryMenu.isOpen}
>
	{#if $secondaryMenu.isOpen && $secondaryMenu.component}
		<div
			transition:fly|local={{ duration: 300, x: width, y: 0, opacity: 1 }}
			id={SECONDARY_MENU_ID}
			class="w-full h-full bg-white p-3"
		>
			<div class="flex justify-between items-center gap-1 mb-3">
				<Button
					color="light"
					size="xs"
					variant="border"
					startIcon={{ icon: faChevronLeft }}
					on:click={secondaryMenu.close}
				>
					Back
				</Button>
				<Badge color="blue">{$selectedComponent}</Badge>
			</div>
			{#if typeof $secondaryMenu.component === 'string'}
				{@html $secondaryMenu.component}
			{:else}
				<svelte:component this={$secondaryMenu.component} {...$secondaryMenu.props} />
			{/if}
		</div>
	{/if}
</div>
