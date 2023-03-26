<script lang="ts">
	import { getContext } from 'svelte'
	import { initOutput } from '../../editor/appUtils'
	import SubGridEditor from '../../editor/SubGridEditor.svelte'
	import type { AppViewerContext, ComponentCustomCSS } from '../../types'
	import { concatCustomCss } from '../../utils'

	export let id: string
	export let componentContainerHeight: number
	export let customCss: ComponentCustomCSS<'containercomponent'> | undefined = undefined
	export let render: boolean

	const { app, focusedGrid, selectedComponent, worldStore } =
		getContext<AppViewerContext>('AppViewerContext')

	//used so that we can count number of outputs setup for first refresh
	initOutput($worldStore, id, {})

	function onFocus() {
		$focusedGrid = {
			parentComponentId: id,
			subGridIndex: 0
		}
	}

	$: css = concatCustomCss($app.css?.containercomponent, customCss)
</script>

<div class="w-full h-full">
	{#if $app.subgrids?.[`${id}-0`]}
		<SubGridEditor
			visible={render}
			{id}
			class={css?.container?.class}
			style={css?.container?.style}
			subGridId={`${id}-0`}
			containerHeight={componentContainerHeight}
			on:focus={() => {
				$selectedComponent = [id]
				onFocus()
			}}
		/>
	{/if}
</div>
