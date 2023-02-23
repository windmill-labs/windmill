<script lang="ts">
	import { getContext } from 'svelte'
	import SubGridEditor from '../../editor/SubGridEditor.svelte'
	import type { AppInput } from '../../inputType'
	import type { AppEditorContext, GridItem } from '../../types'
	import InputValue from '../helpers/InputValue.svelte'

	export let id: string
	export let configuration: Record<string, AppInput>
	export let subGrids: GridItem[][] | undefined = undefined
	export let componentContainerHeight: number
	let noPadding: boolean | undefined = undefined

	export const staticOutputs: string[] = []
	const { focusedGrid, selectedComponent } = getContext<AppEditorContext>('AppEditorContext')

	let gridContent: string[] | undefined = undefined

	function onFocus() {
		$focusedGrid = {
			parentComponentId: id,
			subGridIndex: 0
		}
	}

	$: $selectedComponent === id && onFocus()
</script>

<InputValue {id} input={configuration.noPadding} bind:value={noPadding} />

<InputValue {id} input={configuration.gridContent} bind:value={gridContent} />
{#if subGrids && subGrids[0]}
	<SubGridEditor
		{noPadding}
		bind:subGrid={subGrids[0]}
		containerHeight={componentContainerHeight}
		on:focus={() => {
			$selectedComponent = id
		}}
	/>
{/if}
