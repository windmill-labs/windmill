<script lang="ts">
	import { getContext } from 'svelte'
	import SubGridEditor from '../../editor/SubGridEditor.svelte'
	import type { AppInput } from '../../inputType'
	import type { AppEditorContext, ComponentCustomCSS } from '../../types'
	import { concatCustomCss } from '../../utils'
	import InputValue from '../helpers/InputValue.svelte'
	import { Pane, Splitpanes } from 'svelte-splitpanes'

	export let id: string
	export let configuration: Record<string, AppInput>
	export let componentContainerHeight: number
	export let customCss: ComponentCustomCSS<'container'> | undefined = undefined
	export const staticOutputs: string[] = []

	const { app, focusedGrid, selectedComponent } = getContext<AppEditorContext>('AppEditorContext')

	let noPadding: boolean | undefined = undefined
	let orientation: 'horizontal' | 'vertical' = 'horizontal'
	let paneSelected = 0

	function onFocus() {
		$focusedGrid = {
			parentComponentId: id,
			subGridIndex: paneSelected
		}
	}

	$: $selectedComponent === id && onFocus()

	$: css = concatCustomCss($app.css?.containercomponent, customCss)

	let firstSize = 50
	let secondSize = 50
	$: firstContainerHeight =
		orientation === 'horizontal'
			? (componentContainerHeight * firstSize) / 100
			: componentContainerHeight

	$: secondContainerHeight =
		orientation === 'horizontal'
			? (componentContainerHeight * secondSize) / 100
			: componentContainerHeight
</script>

<InputValue {id} input={configuration.noPadding} bind:value={noPadding} />
<InputValue {id} input={configuration.orientation} bind:value={orientation} />

<div on:pointerdown|stopPropagation>
	<Splitpanes horizontal={orientation === 'horizontal'}>
		<Pane bind:size={firstSize} minSize={20}>
			{#if $app.subgrids?.[`${id}-0`]}
				<SubGridEditor
					{noPadding}
					{id}
					class={css?.container.class}
					style={css?.container.style}
					bind:subGrid={$app.subgrids[`${id}-0`]}
					containerHeight={firstContainerHeight}
					on:focus={() => {
						$selectedComponent = id
						$focusedGrid = {
							parentComponentId: id,
							subGridIndex: 0
						}
					}}
				/>
			{/if}
		</Pane>

		<Pane bind:size={secondSize} minSize={20}>
			{#if $app.subgrids?.[`${id}-1`]}
				<SubGridEditor
					{noPadding}
					{id}
					class={css?.container.class}
					style={css?.container.style}
					bind:subGrid={$app.subgrids[`${id}-1`]}
					containerHeight={secondContainerHeight}
					on:focus={() => {
						$selectedComponent = id
						$focusedGrid = {
							parentComponentId: id,
							subGridIndex: 1
						}
					}}
				/>
			{/if}
		</Pane>
	</Splitpanes>
</div>
