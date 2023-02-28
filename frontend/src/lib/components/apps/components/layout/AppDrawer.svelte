<script lang="ts">
	import { getContext } from 'svelte'
	import SubGridEditor from '../../editor/SubGridEditor.svelte'
	import type { AppInput } from '../../inputType'
	import type { AppEditorContext, ComponentCustomCSS } from '../../types'
	import InputValue from '../helpers/InputValue.svelte'
	import Portal from 'svelte-portal'
	import { concatCustomCss } from '../../utils'
	import { Drawer, DrawerContent } from '$lib/components/common'

	export let customCss: ComponentCustomCSS<'container'> | undefined = undefined
	export let id: string
	export let configuration: Record<string, AppInput>

	const { app, focusedGrid, selectedComponent } = getContext<AppEditorContext>('AppEditorContext')

	let gridContent: string[] | undefined = undefined
	let noPadding: boolean | undefined = undefined
	let drawerTitle: string | undefined = undefined
	let appDrawer: Drawer

	$: css = concatCustomCss($app.css?.containercomponent, customCss)
</script>

<button
	on:click={() => {
		$focusedGrid = {
			parentComponentId: id,
			subGridIndex: 0
		}
		appDrawer.toggleDrawer()
	}}
>
	Open drawer
</button>

<InputValue {id} input={configuration.gridContent} bind:value={gridContent} />
<InputValue {id} input={configuration.noPadding} bind:value={noPadding} />
<InputValue {id} input={configuration.drawerTitle} bind:value={drawerTitle} />

<Portal target="#app-editor-top-level-drawer">
	<Drawer bind:this={appDrawer} size="800px" alwaysOpen>
		<DrawerContent title={drawerTitle} on:close={appDrawer.toggleDrawer}>
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
		</DrawerContent>
	</Drawer>
</Portal>
