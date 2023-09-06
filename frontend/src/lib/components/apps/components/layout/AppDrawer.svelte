<script lang="ts">
	import { getContext } from 'svelte'
	import SubGridEditor from '../../editor/SubGridEditor.svelte'
	import type { AppViewerContext, ComponentCustomCSS, RichConfigurations } from '../../types'
	import Portal from 'svelte-portal'
	import { initCss } from '../../utils'
	import { Button, Drawer, DrawerContent } from '$lib/components/common'
	import { twMerge } from 'tailwind-merge'
	import { AlignWrapper } from '../helpers'
	import { initConfig, initOutput } from '../../editor/appUtils'
	import InitializeComponent from '../helpers/InitializeComponent.svelte'
	import { components } from '../../editor/component'
	import ResolveConfig from '../helpers/ResolveConfig.svelte'

	export let customCss: ComponentCustomCSS<'drawercomponent'> | undefined = undefined
	export let id: string
	export let configuration: RichConfigurations
	export let horizontalAlignment: 'left' | 'center' | 'right' | undefined = undefined
	export let verticalAlignment: 'top' | 'center' | 'bottom' | undefined = undefined
	export let noWFull = false
	export let render: boolean

	const { app, focusedGrid, selectedComponent, worldStore, connectingInput, mode } =
		getContext<AppViewerContext>('AppViewerContext')

	const resolvedConfig = initConfig(
		components['drawercomponent'].initialData.configuration,
		configuration
	)
	initOutput($worldStore, id, {})

	let appDrawer: Drawer

	let css = initCss($app.css?.drawercomponent, customCss)
</script>

{#each Object.keys(components['drawercomponent'].initialData.configuration) as key (key)}
	<ResolveConfig
		{id}
		{key}
		bind:resolvedConfig={resolvedConfig[key]}
		configuration={configuration[key]}
	/>
{/each}

<InitializeComponent {id} />

<div class="h-full w-full">
	<AlignWrapper {noWFull} {horizontalAlignment} {verticalAlignment}>
		<Button
			btnClasses={twMerge(css?.button?.class, 'wm-drawer-button')}
			wrapperClasses={twMerge(
				css?.container?.class,
				'wm-drawer-button-container',
				resolvedConfig?.fillContainer ? 'w-full h-full' : ''
			)}
			wrapperStyle={css?.container?.style}
			disabled={resolvedConfig?.disabled}
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
			size={resolvedConfig.size}
			color={resolvedConfig.color}
			style={css?.button?.style}
		>
			{#if resolvedConfig.label && resolvedConfig.label?.length > 0}
				<div>{resolvedConfig.label}</div>
			{/if}
		</Button>
	</AlignWrapper>
</div>

<Portal target="#app-editor-top-level-drawer">
	<Drawer
		let:open
		bind:this={appDrawer}
		size="800px"
		alwaysOpen
		positionClass={$mode == 'dnd' ? '!absolute' : '!fixed'}
	>
		<DrawerContent
			title={resolvedConfig.drawerTitle}
			on:close={() => {
				appDrawer?.toggleDrawer()
				$focusedGrid = undefined
			}}
			fullScreen={$mode !== 'dnd'}
		>
			<div
				class={twMerge('h-full', css?.drawer?.class, 'wm-drawer')}
				style={css?.drawer?.style}
				on:pointerdown={(e) => {
					e?.stopPropagation()
					if (!$connectingInput.opened) {
						$selectedComponent = [id]
						$focusedGrid = {
							parentComponentId: id,
							subGridIndex: 0
						}
					}
				}}
			>
				{#if $app.subgrids?.[`${id}-0`]}
					<SubGridEditor
						visible={open && render}
						{id}
						subGridId={`${id}-0`}
						on:focus={() => {
							if (!$connectingInput.opened) {
								$selectedComponent = [id]
								$focusedGrid = {
									parentComponentId: id,
									subGridIndex: 0
								}
							}
						}}
					/>
				{/if}
			</div>
		</DrawerContent>
	</Drawer>
</Portal>
