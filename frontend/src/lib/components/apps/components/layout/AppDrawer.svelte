<script lang="ts">
	import { getContext } from 'svelte'
	import SubGridEditor from '../../editor/SubGridEditor.svelte'
	import type { AppInput } from '../../inputType'
	import type { AppEditorContext, ComponentCustomCSS } from '../../types'
	import InputValue from '../helpers/InputValue.svelte'
	import Portal from 'svelte-portal'
	import { concatCustomCss } from '../../utils'
	import { Button, ButtonType, Drawer, DrawerContent } from '$lib/components/common'
	import { twMerge } from 'tailwind-merge'
	import { AlignWrapper } from '../helpers'

	export let customCss: ComponentCustomCSS<'container'> | undefined = undefined
	export let id: string
	export let configuration: Record<string, AppInput>
	export let horizontalAlignment: 'left' | 'center' | 'right' | undefined = undefined
	export let verticalAlignment: 'top' | 'center' | 'bottom' | undefined = undefined
	export let noWFull = false

	const { app, focusedGrid, selectedComponent } = getContext<AppEditorContext>('AppEditorContext')

	let gridContent: string[] | undefined = undefined
	let noPadding: boolean | undefined = undefined
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
				window.dispatchEvent(new Event('pointerup'))
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
<InputValue {id} input={configuration.noPadding} bind:value={noPadding} />
<InputValue {id} input={configuration.drawerTitle} bind:value={drawerTitle} />
<InputValue {id} input={configuration.label} bind:value={labelValue} />
<InputValue {id} input={configuration.color} bind:value={color} />
<InputValue {id} input={configuration.size} bind:value={size} />
<InputValue {id} input={configuration.fillContainer} bind:value={fillContainer} />

<Portal target="#app-editor-top-level-drawer">
	<Drawer bind:this={appDrawer} size="800px" alwaysOpen positionClass="!absolute">
		<DrawerContent title={drawerTitle} on:close={appDrawer.toggleDrawer}>
			{#if $app.subgrids?.[`${id}-0`]}
				<SubGridEditor
					{noPadding}
					{id}
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
