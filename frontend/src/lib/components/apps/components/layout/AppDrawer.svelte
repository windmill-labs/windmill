<script lang="ts">
	import { getContext } from 'svelte'
	import SubGridEditor from '../../editor/SubGridEditor.svelte'
	import type { AppInput } from '../../inputType'
	import type { AppEditorContext, ComponentCustomCSS } from '../../types'
	import InputValue from '../helpers/InputValue.svelte'
	import Portal from 'svelte-portal'
	import { concatCustomCss } from '../../utils'

	export let customCss: ComponentCustomCSS<'container'> | undefined = undefined
	export let id: string
	export let configuration: Record<string, AppInput>

	const { app, focusedGrid, selectedComponent, toggleTopLevelDrawer } =
		getContext<AppEditorContext>('AppEditorContext')

	let gridContent: string[] | undefined = undefined
	let noPadding: boolean | undefined = undefined

	$: css = concatCustomCss($app.css?.containercomponent, customCss)
	$: toggled = false
</script>

<button
	on:click={() => {
		toggled = true
		$focusedGrid = {
			parentComponentId: id,
			subGridIndex: 0
		}
		toggleTopLevelDrawer()
	}}
>
	Open drawer
</button>

<InputValue {id} input={configuration.gridContent} bind:value={gridContent} />
<InputValue {id} input={configuration.noPadding} bind:value={noPadding} />

{#if toggled}
	<Portal target="#app-editor-top-level-drawer">
		{#if $app.subgrids?.[`${id}-0`]}
			<SubGridEditor
				{noPadding}
				class={css?.container.class}
				style={css?.container.style}
				bind:subGrid={$app.subgrids[`${id}-0`]}
				containerHeight={1200}
				on:focus={() => {
					$selectedComponent = id
				}}
			/>
		{/if}
	</Portal>
{/if}
