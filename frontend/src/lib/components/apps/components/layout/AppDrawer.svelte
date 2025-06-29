<script lang="ts">
	import { getContext } from 'svelte'
	import SubGridEditor from '../../editor/SubGridEditor.svelte'
	import type { AppViewerContext, ComponentCustomCSS, RichConfigurations } from '../../types'
	import Portal from '$lib/components/Portal.svelte'

	import { initCss } from '../../utils'
	import { Button, Drawer, DrawerContent } from '$lib/components/common'
	import { twMerge } from 'tailwind-merge'
	import { initConfig, initOutput } from '../../editor/appUtils'
	import InitializeComponent from '../helpers/InitializeComponent.svelte'
	import { components } from '../../editor/component'
	import ResolveConfig from '../helpers/ResolveConfig.svelte'
	import ResolveStyle from '../helpers/ResolveStyle.svelte'
	import AlignWrapper from '../helpers/AlignWrapper.svelte'

	interface Props {
		customCss?: ComponentCustomCSS<'drawercomponent'> | undefined
		id: string
		configuration: RichConfigurations
		horizontalAlignment?: 'left' | 'center' | 'right' | undefined
		verticalAlignment?: 'top' | 'center' | 'bottom' | undefined
		noWFull?: boolean
		render: boolean
		onOpenRecomputeIds?: string[] | undefined
		onCloseRecomputeIds?: string[] | undefined
	}

	let {
		customCss = undefined,
		id,
		configuration,
		horizontalAlignment = undefined,
		verticalAlignment = undefined,
		noWFull = false,
		render,
		onOpenRecomputeIds = undefined,
		onCloseRecomputeIds = undefined
	}: Props = $props()

	let everRender = $state(render)
	$effect(() => {
		render && !everRender && (everRender = true)
	})

	const {
		app,
		focusedGrid,
		selectedComponent,
		worldStore,
		connectingInput,
		mode,
		componentControl,
		runnableComponents
	} = getContext<AppViewerContext>('AppViewerContext')

	const resolvedConfig = $state(
		initConfig(components['drawercomponent'].initialData.configuration, configuration)
	)
	const outputs = initOutput($worldStore, id, {
		open: false
	})

	let containerHeight: number = $state(0)

	let appDrawer: Drawer | undefined = $state()

	$componentControl[id] = {
		open: () => {
			appDrawer?.openDrawer()
		},
		close: () => {
			appDrawer?.closeDrawer()
		}
	}

	let css = $state(initCss($app.css?.drawercomponent, customCss))
</script>

{#each Object.keys(components['drawercomponent'].initialData.configuration) as key (key)}
	<ResolveConfig
		{id}
		{key}
		bind:resolvedConfig={resolvedConfig[key]}
		configuration={configuration[key]}
	/>
{/each}

{#each Object.keys(css ?? {}) as key (key)}
	<ResolveStyle
		{id}
		{customCss}
		{key}
		bind:css={css[key]}
		componentStyle={$app.css?.drawercomponent}
	/>
{/each}

<InitializeComponent {id} />
{#if render}
	<div class="h-full w-full">
		<AlignWrapper {noWFull} {horizontalAlignment} {verticalAlignment}>
			<Button
				btnClasses={twMerge(css?.button?.class, 'wm-drawer-button')}
				wrapperClasses={twMerge(
					css?.container?.class,
					'wm-drawer-button-container',
					resolvedConfig?.fillContainer ? 'w-full h-full' : '',
					resolvedConfig?.hideButtonOnView && $mode == 'preview'
						? 'invisible h-0 overflow-hidden'
						: ''
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
					appDrawer?.toggleDrawer()
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
{/if}

{#if everRender}
	<Portal target="#app-editor-top-level-drawer" name="app-drawer">
		<Drawer
			bind:this={appDrawer}
			size="800px"
			alwaysOpen
			positionClass={$mode == 'dnd' ? '!absolute' : '!fixed'}
			shouldUsePortal={false}
			on:open={() => {
				outputs?.open.set(true)
				onOpenRecomputeIds?.forEach((id) => $runnableComponents?.[id]?.cb?.map((cb) => cb?.()))
			}}
			on:close={() => {
				outputs?.open.set(false)
				onCloseRecomputeIds?.forEach((id) => $runnableComponents?.[id]?.cb?.map((cb) => cb?.()))
			}}
			disableChatOffset={true}
		>
			{#snippet children({ open })}
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
						bind:clientHeight={containerHeight}
						onpointerdown={(e) => {
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
								{containerHeight}
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
			{/snippet}
		</Drawer>
	</Portal>
{:else if $app.subgrids?.[`${id}-0`]}
	<SubGridEditor visible={false} {id} subGridId={`${id}-0`} />
{/if}
