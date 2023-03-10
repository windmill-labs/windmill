<script lang="ts">
	import { getContext } from 'svelte'
	import { findGridItem } from '../../editor/appUtils'
	import type { AppViewerContext } from '../../types'

	export let id: string
	export let shouldWrap: boolean = false
	const { app, breakpoint } = getContext<AppViewerContext>('AppViewerContext')

	$: gridItem = findGridItem($app, id)

	let wrapper: HTMLElement

	$: {
		if (wrapper && gridItem && shouldWrap) {
			const wrapperHeight = wrapper.getBoundingClientRect().height
			const width = $breakpoint === 'sm' ? 3 : 12
			gridItem[width].h = Math.ceil(wrapperHeight / 36)
			gridItem = gridItem
		}
	}
</script>

{#if shouldWrap}
	<div bind:this={wrapper}>
		<slot />
	</div>
{:else}
	<slot />
{/if}
