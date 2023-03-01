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
	export let horizontal: boolean = false
	export let panes: number[]

	export const staticOutputs: string[] = []

	const { app, focusedGrid, selectedComponent } = getContext<AppEditorContext>('AppEditorContext')

	let noPadding: boolean | undefined = undefined

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

<div class="h-full w-full" on:pointerdown|stopPropagation>
	<Splitpanes {horizontal}>
		{#each paneSizes as paneSize, index}
			<Pane bind:size={paneSize} minSize={20}>
				{#if $app.subgrids?.[`${id}-${index}`]}
					<SubGridEditor
						{noPadding}
						{id}
						shouldHighlight={$focusedGrid?.subGridIndex === index}
						class={css?.container.class}
						style={css?.container.style}
						bind:subGrid={$app.subgrids[`${id}-${index}`]}
						containerHeight={horizontal
							? (componentContainerHeight * paneSize) / 100
							: componentContainerHeight}
						on:focus={() => {
							$selectedComponent = id
							$focusedGrid = {
								parentComponentId: id,
								subGridIndex: index
							}
						}}
					/>
				{/if}
			</Pane>
		{/each}
	</Splitpanes>
</div>
