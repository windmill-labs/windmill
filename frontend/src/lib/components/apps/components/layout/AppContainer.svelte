<script lang="ts">
	import { getContext } from 'svelte'
	import SubGridEditor from '../../editor/SubGridEditor.svelte'
	import type { AppInput } from '../../inputType'
	import type { AppEditorContext, GridItem } from '../../types'
	import InputValue from '../helpers/InputValue.svelte'
	import RunnableWrapper from '../helpers/RunnableWrapper.svelte'

	export let id: string
	export let componentInput: AppInput | undefined
	export let initializing: boolean | undefined = undefined
	export let configuration: Record<string, AppInput>
	export let subGrids: GridItem[][] | undefined = undefined
	export let componentContainerHeight: number

	export const staticOutputs: string[] = ['loading', 'result']
	const { focusedGrid, selectedComponent } = getContext<AppEditorContext>('AppEditorContext')

	let result: string[] | undefined = undefined
	let gridContent: string[] | undefined = undefined

	function onFocus() {
		$focusedGrid = {
			parentComponentId: id,
			subGridIndex: 0
		}
	}

	$: $selectedComponent === id && onFocus()
</script>

<InputValue {id} input={configuration.gridContent} bind:value={gridContent} />
<RunnableWrapper flexWrap bind:componentInput {id} bind:initializing bind:result>
	{#if subGrids && subGrids[0]}
		<SubGridEditor
			bind:subGrid={subGrids[0]}
			containerHeight={componentContainerHeight}
			on:focus={() => {
				$selectedComponent = id
			}}
		/>
	{/if}
</RunnableWrapper>
