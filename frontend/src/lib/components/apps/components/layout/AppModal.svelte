<script lang="ts">
	import { getContext } from 'svelte'
	import SubGridEditor from '../../editor/SubGridEditor.svelte'
	import type { AppViewerContext, ComponentCustomCSS, RichConfigurations } from '../../types'
	import { initCss } from '../../utils'
	import { Button } from '$lib/components/common'
	import { twMerge } from 'tailwind-merge'
	import { initConfig, initOutput, maxHeight, ROW_GAP_Y, ROW_HEIGHT } from '../../editor/appUtils'
	import InitializeComponent from '../helpers/InitializeComponent.svelte'
	import Portal from '$lib/components/Portal.svelte'

	import { clickOutside } from '$lib/utils'
	import { X } from 'lucide-svelte'
	import { components } from '../../editor/component'
	import ResolveConfig from '../helpers/ResolveConfig.svelte'
	import ResolveStyle from '../helpers/ResolveStyle.svelte'
	import Disposable from '$lib/components/common/drawer/Disposable.svelte'
	import AlignWrapper from '../helpers/AlignWrapper.svelte'

	export let customCss: ComponentCustomCSS<'modalcomponent'> | undefined = undefined
	export let id: string
	export let configuration: RichConfigurations
	export let horizontalAlignment: 'left' | 'center' | 'right' | undefined = undefined
	export let verticalAlignment: 'top' | 'center' | 'bottom' | undefined = undefined
	export let noWFull = false
	export let render: boolean

	export let onOpenRecomputeIds: string[] | undefined = undefined
	export let onCloseRecomputeIds: string[] | undefined = undefined

	const {
		app,
		focusedGrid,
		selectedComponent,
		worldStore,
		connectingInput,
		mode,
		componentControl,
		runnableComponents,
		breakpoint
	} = getContext<AppViewerContext>('AppViewerContext')

	let everRender = render
	$: render && !everRender && (everRender = true)

	//used so that we can count number of outputs setup for first refresh
	const outputs = initOutput($worldStore, id, {
		open: false
	})

	let css = initCss($app.css?.modalcomponent, customCss)
	let disposable: Disposable | undefined = undefined

	let resolvedConfig = initConfig(
		components['modalcomponent'].initialData.configuration,
		configuration
	)

	let unclickableOutside = false
	function unclosableModal() {
		unclickableOutside = true
		setTimeout(() => {
			unclickableOutside = false
		}, 1000)
	}

	$componentControl[id] = {
		openModal: () => {
			unclosableModal()

			disposable?.openDrawer()
		},
		closeModal: () => {
			disposable?.closeDrawer()
		},
		open: () => {
			unclosableModal()
			disposable?.openDrawer()
		},
		close: () => {
			disposable?.closeDrawer()
		}
	}
	let wrapperHeight: number = 0
	let headerHeight: number = 0

	$: containerHeight = Math.min(
		// 8px * 2 of padding
		maxHeight($app.subgrids?.[`${id}-0`] ?? [], 0, $breakpoint) * (ROW_HEIGHT + ROW_GAP_Y) + 16,
		// 32px (2rem) of top and bottom margin
		wrapperHeight - headerHeight - 64
	)

	async function getMenuElements(): Promise<HTMLElement[]> {
		return Array.from(document.querySelectorAll('[data-menu]')) as HTMLElement[]
	}
</script>

<InitializeComponent {id} />

{#each Object.keys(components['modalcomponent'].initialData.configuration) as key (key)}
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
		componentStyle={$app.css?.modalcomponent}
	/>
{/each}

{#if everRender}
	<div class="h-full w-full">
		<AlignWrapper {noWFull} {horizontalAlignment} {verticalAlignment}>
			<Button
				btnClasses={twMerge(css?.button?.class, 'wm-button', 'wm-modal-button')}
				wrapperClasses={twMerge(
					resolvedConfig?.buttonFillContainer ? 'w-full h-full' : '',
					css?.buttonContainer?.class,
					'wm-button-container',
					'wm-modal-button-container',
					resolvedConfig?.hideButtonOnView && $mode == 'preview'
						? 'invisible h-0 overflow-hidden'
						: ''
				)}
				style={css?.button?.style}
				wrapperStyle={css?.buttonContainer?.style}
				disabled={resolvedConfig.buttonDisabled}
				on:pointerdown={(e) => {
					e?.stopPropagation()
				}}
				on:click={async (e) => {
					$focusedGrid = {
						parentComponentId: id,
						subGridIndex: 0
					}
					disposable?.openDrawer()
				}}
				size={resolvedConfig.buttonSize}
				color={resolvedConfig.buttonColor}
			>
				<div>{resolvedConfig.buttonLabel}</div>
			</Button>
		</AlignWrapper>
	</div>
	<Portal target="#app-editor-top-level-drawer" name="app-modal">
		<Disposable
			{id}
			let:handleClickAway
			let:zIndex
			let:open
			bind:this={disposable}
			on:open={() => {
				outputs?.open.set(true)
				onOpenRecomputeIds?.forEach((id) => $runnableComponents?.[id]?.cb?.map((cb) => cb?.()))
			}}
			on:close={() => {
				outputs?.open.set(false)
				onCloseRecomputeIds?.forEach((id) => $runnableComponents?.[id]?.cb?.map((cb) => cb?.()))
			}}
		>
			<div
				class={twMerge(
					`${
						$mode == 'dnd' ? 'absolute' : 'fixed'
					} top-0 bottom-0 left-0 right-0 transition-all duration-50`,
					open ? ' bg-black bg-opacity-60' : 'h-0 overflow-hidden invisible'
				)}
				style="z-index: {zIndex}"
				bind:clientHeight={wrapperHeight}
			>
				<div
					style={css?.popup?.style}
					class={twMerge('mx-24 mt-8 bg-surface rounded-lg relative', css?.popup?.class)}
					use:clickOutside={{
						capture: false,
						stopPropagation: false,
						exclude: getMenuElements
					}}
					on:click_outside={(e) => {
						if ($mode !== 'dnd' && !unclickableOutside) {
							handleClickAway(e)
						}
					}}
				>
					<div
						class="px-4 py-2 border-b flex justify-between items-center"
						bind:clientHeight={headerHeight}
					>
						<div>{resolvedConfig.modalTitle}</div>
						<div class="w-8">
							<button
								on:click|stopPropagation={() => {
									disposable?.closeDrawer()
								}}
								class="hover:bg-surface-hover bg-surface-secondary rounded-full w-8 h-8 flex items-center justify-center transition-all"
							>
								<X class="text-tertiary" />
							</button>
						</div>
					</div>

					<div
						class={twMerge('wm-modal h-full', 'overflow-y-auto')}
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
				</div>
			</div>
		</Disposable>
	</Portal>
{:else if $app.subgrids?.[`${id}-0`]}
	<SubGridEditor visible={false} {id} subGridId={`${id}-0`} />
{/if}
