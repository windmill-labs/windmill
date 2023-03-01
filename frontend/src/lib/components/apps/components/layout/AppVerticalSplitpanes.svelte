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
	export let numberOfSubgrids: number | undefined = undefined
	export const staticOutputs: string[] = []

	const { app, focusedGrid, selectedComponent } = getContext<AppEditorContext>('AppEditorContext')

	let noPadding: boolean | undefined = undefined
	let numberOfPanes: number = Number(numberOfSubgrids) ?? 0

	function onFocus() {
		$focusedGrid = {
			parentComponentId: id,
			subGridIndex: 0
		}
	}

	$: paneSizes = []

	$: if (numberOfPanes !== paneSizes.length) {
		for (let i = numberOfPanes; i < paneSizes.length; i++) {
			if ($app.subgrids && $app.subgrids?.[`${id}-${i}`]) {
				delete $app.subgrids[`${id}-${i}`]
			}
		}

		numberOfSubgrids = numberOfPanes
		paneSizes = Array.from({ length: numberOfPanes }, (_, i) => paneSizes?.[i - 1] ?? 20)

		for (let i = 0; i < numberOfPanes; i++) {
			if ($app.subgrids && !$app.subgrids?.[`${id}-${i}`]) {
				$app.subgrids[`${id}-${i}`] = []
			}
		}
	}

	$: $selectedComponent === id && onFocus()
	$: css = concatCustomCss($app.css?.containercomponent, customCss)
</script>

<InputValue {id} input={configuration.noPadding} bind:value={noPadding} />
<InputValue {id} input={configuration.numberOfPanes} bind:value={numberOfPanes} />

<div on:pointerdown|stopPropagation>
	<Splitpanes horizontal={false}>
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
						containerHeight={componentContainerHeight}
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
