<script lang="ts">
	import { fly } from 'svelte/transition'
	import { Badge } from '../../../../common'
	import { secondaryMenuLeft, secondaryMenuRight } from './'
	import { getContext } from 'svelte'
	import type { AppViewerContext } from '../../../types'
	import CloseButton from '$lib/components/common/CloseButton.svelte'

	const { selectedComponent } = getContext<AppViewerContext>('AppViewerContext')
	export let right: boolean

	let secondaryMenu = right ? secondaryMenuRight : secondaryMenuLeft
	let width: number
	let lastSelected = $selectedComponent

	$: if (right && lastSelected !== $selectedComponent) {
		secondaryMenu.close()
		lastSelected = $selectedComponent
	}
</script>

<!-- z-index must be above the split pane handles' z-index (which is 1001 atm.) -->
<div
	bind:clientWidth={width}
	class="absolute z-[1002] inset-0 overflow-hidden w-full"
	class:pointer-events-none={!$secondaryMenu.isOpen}
>
	{#if $secondaryMenu.isOpen && $secondaryMenu.component}
		<div
			transition:fly|local={{ duration: 300, x: right ? width : -width, y: 0, opacity: 1 }}
			class="flex flex-col w-full h-full bg-surface"
		>
			<div
				class="flex justify-between {right ? '' : 'flex-row-reverse'} items-center gap-1 px-3 py-2"
			>
				<CloseButton on:close={() => secondaryMenu?.close()} />
				{#if $selectedComponent}
					<Badge color="blue">{$selectedComponent}</Badge>
				{:else}
					<div />
				{/if}
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
