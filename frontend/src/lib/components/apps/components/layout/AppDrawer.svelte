<script lang="ts">
	import { getContext } from 'svelte'
	import SubGridEditor from '../../editor/SubGridEditor.svelte'
	import type { AppViewerContext, ComponentCustomCSS, RichConfigurations } from '../../types'
	import InputValue from '../helpers/InputValue.svelte'
	import Portal from 'svelte-portal'
	import { concatCustomCss } from '../../utils'
	import { Button, ButtonType, Drawer, DrawerContent } from '$lib/components/common'
	import { twMerge } from 'tailwind-merge'
	import { AlignWrapper } from '../helpers'
	import { initOutput } from '../../editor/appUtils'

	export let customCss: ComponentCustomCSS<'drawercomponent'> | undefined = undefined
	export let id: string
	export let configuration: RichConfigurations
	export let horizontalAlignment: 'left' | 'center' | 'right' | undefined = undefined
	export let verticalAlignment: 'top' | 'center' | 'bottom' | undefined = undefined
	export let noWFull = false
	export let render: boolean

	const { app, focusedGrid, selectedComponent, worldStore } =
		getContext<AppViewerContext>('AppViewerContext')

	//used so that we can count number of outputs setup for first refresh
	initOutput($worldStore, id, {})

	let gridContent: string[] | undefined = undefined
	let drawerTitle: string | undefined = undefined
	let appDrawer: Drawer

	let labelValue: string
	let color: ButtonType.Color
	let size: ButtonType.Size
	let disabled: boolean | undefined = undefined
	let fillContainer: boolean | undefined = undefined

	$: css = concatCustomCss($app.css?.containercomponent, customCss)
</script>

<div class="h-full w-full">
	<AlignWrapper {noWFull} {horizontalAlignment} {verticalAlignment}>
		<Button
			btnClasses={twMerge(
				$app.css?.['buttoncomponent']?.['button']?.class,
				fillContainer ? 'w-full h-full' : ''
			)}
			{disabled}
			on:pointerdown={(e) => {
				e?.stopPropagation()
			}}
			on:click={async (e) => {
				$focusedGrid = {
					parentComponentId: id,
					subGridIndex: 0
				}
				appDrawer.toggleDrawer()
			}}
			{size}
			{color}
		>
			<div>{labelValue}</div>
		</Button>
	</AlignWrapper>
</div>

<InputValue {id} input={configuration.gridContent} bind:value={gridContent} />
<InputValue {id} input={configuration.drawerTitle} bind:value={drawerTitle} />
<InputValue {id} input={configuration.label} bind:value={labelValue} />
<InputValue {id} input={configuration.color} bind:value={color} />
<InputValue {id} input={configuration.size} bind:value={size} />
<InputValue {id} input={configuration.fillContainer} bind:value={fillContainer} />

<Portal target="#app-editor-top-level-drawer">
	<Drawer let:open bind:this={appDrawer} size="800px" alwaysOpen positionClass="!absolute">
		<DrawerContent
			title={drawerTitle}
			on:close={() => {
				appDrawer?.toggleDrawer()
				$focusedGrid = undefined
			}}
		>
			{#if $app.subgrids?.[`${id}-0`]}
				<SubGridEditor
					visible={open && render}
					{id}
					class={css?.container?.class}
					style={css?.container?.style}
					subGridId={`${id}-0`}
					containerHeight={1200}
					on:focus={() => {
						$selectedComponent = [id]
					}}
				/>
			{/if}
		</DrawerContent>
	</Drawer>
</Portal>
