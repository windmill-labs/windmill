<script lang="ts">
	import { getContext } from 'svelte'
	import SubGridEditor from '../../editor/SubGridEditor.svelte'
	import type { AppInput } from '../../inputType'
	import type { AppViewerContext, ComponentCustomCSS, GridItem } from '../../types'
	import { concatCustomCss } from '../../utils'

	export let id: string
	export let configuration: Record<string, AppInput>
	export let componentContainerHeight: number
	export let customCss: ComponentCustomCSS<'container'> | undefined = undefined
	export let render: boolean

	export const staticOutputs: string[] = []
	const { app, focusedGrid, selectedComponent } = getContext<AppViewerContext>('AppViewerContext')

	function onFocus() {
		$focusedGrid = {
			parentComponentId: id,
			subGridIndex: 0
		}
	}

	$: $selectedComponent === id && onFocus()

	$: css = concatCustomCss($app.css?.containercomponent, customCss)
</script>

<div class="w-full h-full">
	{#if $app.subgrids?.[`${id}-0`]}
		<SubGridEditor
			visible={render}
			{id}
			class={css?.container.class}
			style={css?.container.style}
			bind:subGrid={$app.subgrids[`${id}-0`]}
			containerHeight={componentContainerHeight}
			on:focus={() => {
				$selectedComponent = id
			}}
		/>
	{/if}
</div>
