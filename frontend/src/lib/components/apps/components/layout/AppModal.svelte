<script lang="ts">
	import { stopPropagation } from 'svelte/legacy'

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

	interface Props {
		customCss?: ComponentCustomCSS<'modalcomponent'> | undefined
		id: string
		configuration: RichConfigurations
		horizontalAlignment?: 'left' | 'center' | 'right' | undefined
		verticalAlignment?: 'top' | 'center' | 'bottom' | undefined
		noWFull?: boolean
		render: boolean
		onOpenRecomputeIds?: string[] | undefined
		onCloseRecomputeIds?: string[] | undefined
		preclickAction?: (() => Promise<void>) | undefined
		onClose?: () => void
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
		onCloseRecomputeIds = undefined,
		preclickAction,
		onClose = undefined
	}: Props = $props()

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

	let everRender = $state(render)
	$effect.pre(() => {
		render && !everRender && (everRender = true)
	})

	//used so that we can count number of outputs setup for first refresh
	const outputs = initOutput($worldStore, id, {
		open: false
	})

	let css = $state(initCss($app.css?.modalcomponent, customCss))
	let disposable: Disposable | undefined = $state(undefined)

	let resolvedConfig = $state(
		initConfig(components['modalcomponent'].initialData.configuration, configuration)
	)

	let unclickableOutside = $state(false)
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
	let wrapperHeight: number = $state(0)
	let headerHeight: number = $state(0)

	let containerHeight = $derived(
		Math.min(
			// 8px * 2 of padding
			maxHeight($app.subgrids?.[`${id}-0`] ?? [], 0, $breakpoint) * (ROW_HEIGHT + ROW_GAP_Y) + 16,
			// 32px (2rem) of top and bottom margin
			wrapperHeight - headerHeight - 64
		)
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
					await preclickAction?.()
					$focusedGrid = {
						parentComponentId: id,
						subGridIndex: 0
					}
					disposable?.openDrawer()
				}}
				extendedSize={resolvedConfig.buttonSize}
				color={resolvedConfig.buttonColor}
				variant="contained"
			>
				<div>{resolvedConfig.buttonLabel}</div>
			</Button>
		</AlignWrapper>
	</div>
	<Portal target="#app-editor-top-level-drawer" name="app-modal">
		<Disposable
			{id}
			bind:this={disposable}
			onOpen={() => {
				outputs?.open.set(true)
				onOpenRecomputeIds?.forEach((id) => $runnableComponents?.[id]?.cb?.map((cb) => cb?.()))
			}}
			onClose={() => {
				outputs?.open.set(false)
				onCloseRecomputeIds?.forEach((id) => $runnableComponents?.[id]?.cb?.map((cb) => cb?.()))
				onClose?.()
			}}
		>
			{#snippet children({ handleClickAway, zIndex, open })}
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
						class={twMerge('mx-24 mt-8 bg-surface wm-modal rounded-lg relative', css?.popup?.class)}
						use:clickOutside={{
							capture: false,
							stopPropagation: false,
							exclude: getMenuElements,
							onClickOutside: (e: MouseEvent) => {
								if ($mode !== 'dnd' && !unclickableOutside) {
									handleClickAway(e)
								}
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
									onclick={stopPropagation(() => {
										disposable?.closeDrawer()
									})}
									class="hover:bg-surface-hover bg-surface-secondary rounded-full w-8 h-8 flex items-center justify-center transition-all"
								>
									<X class="text-primary" />
								</button>
							</div>
						</div>

						<div
							class={twMerge('wm-modal-container h-full', 'overflow-y-auto', css?.container?.class)}
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
									onFocus={() => {
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
			{/snippet}
		</Disposable>
	</Portal>
{:else if $app.subgrids?.[`${id}-0`]}
	<SubGridEditor visible={false} {id} subGridId={`${id}-0`} />
{/if}
