<script lang="ts">
	import { getContext } from 'svelte'
	import type { AppEditorContext, AppViewerContext } from '../types'
	import { columnConfiguration, gridColumns, isFixed, toggleFixed } from '../gridUtils'
	import { twMerge } from 'tailwind-merge'
	import panzoom from 'panzoom'

	import HiddenComponent from '../components/helpers/HiddenComponent.svelte'
	import Component from './component/Component.svelte'
	import { push } from '$lib/history'
	import {
		dfs,
		expandGriditem,
		findGridItem,
		findGridItemParentGrid,
		insertNewGridItem,
		isContainer,
		subGridIndexKey
	} from './appUtils'
	import Grid from '../svelte-grid/Grid.svelte'
	import { deepEqual } from 'fast-equals'
	import ComponentWrapper from './component/ComponentWrapper.svelte'
	import { classNames } from '$lib/utils'
	import { BG_PREFIX } from '../utils'
	import GridEditorMenu from './GridEditorMenu.svelte'
	import RecomputeAllComponents from './RecomputeAllComponents.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import { Loader2 } from 'lucide-svelte'
	import Popover from '$lib/components/Popover.svelte'
	import type { Policy } from '$lib/gen'

	export let policy: Policy

	const {
		selectedComponent,
		app,
		connectingInput,
		summary,
		focusedGrid,
		parentWidth,
		breakpoint,
		allIdsInPath,
		bgRuns,
		worldStore
	} = getContext<AppViewerContext>('AppViewerContext')

	const { history, scale, componentActive } = getContext<AppEditorContext>('AppEditorContext')

	let previousSelectedIds: string[] | undefined = undefined
	$: if (!deepEqual(previousSelectedIds, $selectedComponent)) {
		previousSelectedIds = $selectedComponent
		$allIdsInPath = ($selectedComponent ?? [])
			.flatMap((id) => dfs($app.grid, id, $app.subgrids ?? {}))
			.filter((x) => x != undefined) as string[]
	}

	function handleLock(id: string) {
		const gridItem = findGridItem($app, id)
		if (gridItem) {
			toggleFixed(gridItem)
		}
		$app = $app
	}

	function handleFillHeight(id: string) {
		const gridItem = findGridItem($app, id)
		const b = $breakpoint === 'sm' ? 3 : 12
		if (gridItem?.[b]) {
			gridItem[b].fullHeight = !gridItem[b].fullHeight
		}
		$app = $app
	}

	export function moveComponentBetweenSubgrids(
		componentId: string,
		parentComponentId: string,
		subGridIndex: number,
		position?: { x: number; y: number }
	) {
		// Find the component in the source subgrid
		const component = findGridItem($app, componentId)

		if (!component) {
			return
		}

		let parentGrid = findGridItemParentGrid($app, component.id)
		if (parentGrid) {
			$app.subgrids &&
				($app.subgrids[parentGrid] = $app.subgrids[parentGrid].filter(
					(item) => item.id !== component?.id
				))
		} else {
			$app.grid = $app.grid.filter((item) => item.id !== component?.id)
		}

		const gridItem = component
		insertNewGridItem(
			$app,
			(id) => ({ ...gridItem.data, id }),
			{ parentComponentId: parentComponentId, subGridIndex: subGridIndex },
			Object.fromEntries(gridColumns.map((column) => [column, gridItem[column]])),
			component.id,
			position,
			undefined,
			undefined,
			undefined,
			true
		)

		// Update the app state
		$app = { ...$app }

		$selectedComponent = [parentComponentId]
		$focusedGrid = {
			parentComponentId,
			subGridIndex
		}
	}

	let instance: any
	let parentHeight: number = 0

	function initPanzoom(node: HTMLElement) {
		instance = panzoom(node, {
			bounds: true,
			boundsPadding: 0.1,
			maxZoom: 1.5,
			minZoom: 0.3,
			zoomDoubleClickSpeed: 1,
			smoothScroll: false,

			initialZoom: $scale / 100,
			beforeMouseDown: (e) => {
				if (e.ctrlKey || e.metaKey) {
					// Prevent event propagation to children when panning
					e.stopPropagation()
					return false
				}
				return true
			},
			beforeWheel: (e) => {
				if (e.ctrlKey || e.metaKey) {
					// Prevent event propagation to children when zooming
					e.stopPropagation()
					return false
				}
				return true
			}
		})

		// Update scale store when zoom changes
		instance.on('zoom', (e) => {
			const currentScale = e.getTransform().scale * 100
			if (currentScale !== $scale) {
				$scale = currentScale
			}
		})

		return {
			destroy() {
				instance.dispose()
			}
		}
	}

	$: if (instance && $scale) {
		const currentScale = instance.getTransform().scale * 100
		if (currentScale !== $scale) {
			instance.zoomAbs($parentWidth / 2, parentHeight / 2, $scale / 100)
		}
	}

	let isModifierKeyPressed = false

	function handleKeyDown(e: KeyboardEvent) {
		isModifierKeyPressed = e.ctrlKey || e.metaKey
	}

	function handleKeyUp(e: KeyboardEvent) {
		isModifierKeyPressed = false
	}

	export function resetView() {
		if (instance) {
			instance.reset()
		}
	}
</script>

<svelte:window on:keydown={handleKeyDown} on:keyup={handleKeyUp} />

<div class="w-full z-[1000] overflow-visible h-full">
	<div class={$app.hideLegacyTopBar ? 'hidden' : ''}>
		<div
			class="w-full sticky top-0 flex justify-between border-b {$componentActive
				? 'invisible'
				: 'z-50'} {$connectingInput?.opened ? '' : 'bg-surface'} px-4 py-1 items-center gap-4"
		>
			<h3 class="truncate">{$summary}</h3>
			<div class="flex gap-2 items-center">
				<div>
					{#if !$connectingInput.opened}
						<RecomputeAllComponents />
					{/if}
				</div>
				{#if $bgRuns.length > 0}
					<Popover notClickable>
						<span class="!text-2xs text-tertiary inline-flex gap-1 items-center"
							><Loader2 size={10} class="animate-spin" /> {$bgRuns.length}
						</span>
						<span slot="text"
							><div class="flex flex-col">
								{#each $bgRuns as bgRun}
									<div class="flex gap-2 items-center">
										<div class="text-2xs text-tertiary">{bgRun}</div>
									</div>
								{/each}
							</div></span
						>
					</Popover>
				{:else}
					<span class="w-9" />
				{/if}
			</div>
			<div class="flex text-2xs gap-8 items-center">
				<div class="py-2 pr-2 text-secondary flex gap-1 items-center">
					Hide bar on view
					<Toggle size="xs" bind:checked={$app.norefreshbar} />
				</div>
				<div>
					{policy.on_behalf_of ? `Author ${policy.on_behalf_of_email}` : ''}
					<Tooltip>
						The scripts will be run on behalf of the author and a tight policy ensure security about
						the possible inputs of the runnables.
					</Tooltip>
				</div>
			</div>
		</div>
	</div>
	<!-- svelte-ignore a11y-click-events-have-key-events -->
	<div
		style={$app.css?.['app']?.['grid']?.style}
		class={twMerge(
			'p-2 overflow-visible z-50',
			$app.css?.['app']?.['grid']?.class ?? '',
			'wm-app-grid !static h-full w-full',
			isModifierKeyPressed && 'cursor-grab'
		)}
		on:pointerdown={() => {
			$selectedComponent = undefined
			$focusedGrid = undefined
		}}
		bind:clientWidth={$parentWidth}
		bind:clientHeight={parentHeight}
	>
		<div
			class="subgrid overflow-visible z-100 border border-dashed border-tertiary {isModifierKeyPressed
				? 'pointer-events-none'
				: ''}"
			use:initPanzoom
		>
			<Grid
				allIdsInPath={$allIdsInPath}
				selectedIds={$selectedComponent}
				items={$app.grid}
				on:redraw={(e) => {
					push(history, $app)
					$app.grid = e.detail
				}}
				root
				let:dataItem
				let:hidden
				let:overlapped
				let:moveMode
				let:componentDraggedId
				cols={columnConfiguration}
				on:dropped={(e) => {
					const { id, overlapped, x, y } = e.detail

					const overlappedComponent = findGridItem($app, overlapped)

					if (overlappedComponent && !isContainer(overlappedComponent.data.type)) {
						return
					}

					if (!overlapped) {
						return
					}

					if (id === overlapped) {
						return
					}

					moveComponentBetweenSubgrids(
						id,
						overlapped,
						subGridIndexKey(overlappedComponent?.data?.type, overlapped, $worldStore),
						{ x, y }
					)
				}}
				disableMove={!!$connectingInput.opened}
			>
				<ComponentWrapper
					id={dataItem.id}
					type={dataItem.data.type}
					class={classNames(
						'h-full w-full center-center outline outline-surface-secondary',
						Boolean($selectedComponent?.includes(dataItem.id)) ? 'active-grid-item' : ''
					)}
				>
					<GridEditorMenu
						id={dataItem.id}
						on:expand={() => {
							push(history, $app)
							$selectedComponent = [dataItem.id]
							expandGriditem($app.grid, dataItem.id, $breakpoint)
							$app = $app
						}}
						on:lock={() => {
							handleLock(dataItem.id)
						}}
						on:fillHeight={() => {
							handleFillHeight(dataItem.id)
						}}
						locked={isFixed(dataItem)}
						fullHeight={dataItem?.[$breakpoint === 'sm' ? 3 : 12]?.fullHeight}
					>
						<Component
							{hidden}
							render={true}
							component={dataItem.data}
							selected={Boolean($selectedComponent?.includes(dataItem.id))}
							locked={isFixed(dataItem)}
							fullHeight={dataItem?.[$breakpoint === 'sm' ? 3 : 12]?.fullHeight}
							on:lock={() => {
								handleLock(dataItem.id)
							}}
							on:fillHeight={() => {
								handleFillHeight(dataItem.id)
							}}
							{overlapped}
							{moveMode}
							{componentDraggedId}
						/>
					</GridEditorMenu>
				</ComponentWrapper>
			</Grid>
		</div>
	</div>
</div>

{#if $app.hiddenInlineScripts}
	{#each $app.hiddenInlineScripts as runnable, index}
		{#if runnable}
			<HiddenComponent id={BG_PREFIX + index} {runnable} />
		{/if}
	{/each}
{/if}

<style global>
	.svlt-grid-active {
		opacity: 1 !important;
	}
</style>
