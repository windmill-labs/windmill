<script lang="ts">
	import { getContext } from 'svelte'
	import SubGridEditor from '../../editor/SubGridEditor.svelte'
	import type { AppInput } from '../../inputType'
	import type { AppEditorContext, ComponentCustomCSS, GridItem } from '../../types'
	import { concatCustomCss } from '../../utils'
	import InputValue from '../helpers/InputValue.svelte'
	import RunnableWrapper from '../helpers/RunnableWrapper.svelte'

	export let id: string
	export let componentInput: AppInput | undefined
	export let initializing: boolean | undefined = undefined
	export let configuration: Record<string, AppInput>
	export let subGrids: GridItem[][] | undefined = undefined
	export let componentContainerHeight: number
	export let customCss: ComponentCustomCSS<'container'> | undefined = undefined

	export const staticOutputs: string[] = ['loading', 'result']
	const { app, focusedGrid, selectedComponent } = getContext<AppEditorContext>('AppEditorContext')

	let result: string[] | undefined = undefined
	let gridContent: string[] | undefined = undefined

	function onFocus() {
		$focusedGrid = {
			parentComponentId: id,
			subGridIndex: 0
		}
	}

	$: $selectedComponent === id && onFocus()
	
	$: css = concatCustomCss($app.css?.containercomponent, customCss)
</script>

<InputValue {id} input={configuration.gridContent} bind:value={gridContent} />
<RunnableWrapper flexWrap bind:componentInput {id} bind:initializing bind:result>
	{#if subGrids && subGrids[0]}
		<SubGridEditor
			bind:subGrid={subGrids[0]}
			class={css?.container.class ?? ''}
			style={css?.container.style ?? ''}
			containerHeight={componentContainerHeight}
			on:focus={() => {
				$selectedComponent = id
			}}
		/>
	{/if}
</RunnableWrapper>
