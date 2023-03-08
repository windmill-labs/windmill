<script lang="ts">
	import { getContext } from 'svelte'
	import SubGridEditor from '../../editor/SubGridEditor.svelte'
	import type { AppInput } from '../../inputType'
	import type { AppEditorContext, ComponentCustomCSS, GridItem } from '../../types'
	import { concatCustomCss } from '../../utils'
	import InputValue from '../helpers/InputValue.svelte'

	export let id: string
	export let configuration: Record<string, AppInput>
	export let componentContainerHeight: number
	export let customCss: ComponentCustomCSS<'container'> | undefined = undefined
	export let render: boolean

	let noPadding: boolean | undefined = undefined

	export const staticOutputs: string[] = []
	const { app, focusedGrid, selectedComponent } = getContext<AppEditorContext>('AppEditorContext')

	function onFocus() {
		$focusedGrid = {
			parentComponentId: id,
			subGridIndex: 0
		}
	}

	$: $selectedComponent === id && onFocus()

	$: css = concatCustomCss($app.css?.containercomponent, customCss)
</script>

<InputValue {id} input={configuration.noPadding} bind:value={noPadding} />

<div class="border">
	{#if $app.subgrids?.[`${id}-0`]}
		<SubGridEditor
			visible={render}
			{noPadding}
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
