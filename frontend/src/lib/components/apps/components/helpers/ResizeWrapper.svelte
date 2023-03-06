<script lang="ts">
	import { getContext } from 'svelte'
	import { findGridItem } from '../../editor/appUtils'
	import type { AppEditorContext } from '../../types'
	import gridHelp from '@windmill-labs/svelte-grid/src/utils/helper'

	export let id: string

	const { app } = getContext<AppEditorContext>('AppEditorContext')

	$: gridItem = findGridItem($app, id)

	let wrapper: HTMLElement

	$: {
		if (wrapper && gridItem) {
			const wrapperHeight = wrapper.getBoundingClientRect().height
			const parentHeight = wrapper.parentElement?.getBoundingClientRect().height

			console.log(wrapperHeight, parentHeight)

			if (parentHeight && wrapperHeight > parentHeight) {
				gridItem[12].h = Math.ceil(wrapperHeight / 36)

				gridItem = gridItem
			}

			if (parentHeight && wrapperHeight < parentHeight) {
				gridItem[12].h = Math.floor(parentHeight / 36)

				gridItem = gridItem
			}
		}
	}
</script>

<div bind:this={wrapper}>
	<slot />
</div>
