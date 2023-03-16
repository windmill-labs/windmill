<script lang="ts">
	import { getContext } from 'svelte'
	import { findGridItem } from '../../editor/appUtils'
	import type { AppViewerContext } from '../../types'

	export let id: string
	export let shouldWrap: boolean = false
	const { app, breakpoint, mode } = getContext<AppViewerContext>('AppViewerContext')

	$: gridItem = findGridItem($app, id)

	let wrapper: HTMLElement

	$: {
		if (wrapper && gridItem && shouldWrap) {
			const wrapperHeight = wrapper.getBoundingClientRect().height
			const width = $breakpoint === 'sm' ? 3 : 12
			gridItem[width].h = Math.ceil(wrapperHeight / 36)
		}
	}
</script>

{#if shouldWrap && $mode !== 'preview'}
	<div class="h-full w-full" bind:this={wrapper}>
		<slot />
	</div>
{:else}
	<slot />
{/if}
