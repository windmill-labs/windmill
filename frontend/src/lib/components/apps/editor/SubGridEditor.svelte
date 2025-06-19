<script lang="ts">
	import { stopPropagation } from 'svelte/legacy'

	import { push } from '$lib/history'
	import { classNames } from '$lib/utils'
	import { createEventDispatcher, getContext, onDestroy } from 'svelte'
	import { twMerge } from 'tailwind-merge'
	import { gridColumns, isFixed, toggleFixed } from '../gridUtils'
	import Grid from '../svelte-grid/Grid.svelte'
	import type { AppEditorContext, AppViewerContext, GridItem } from '../types'
	import {
		expandGriditem,
		findGridItem,
		findGridItemParentGrid,
		insertNewGridItem,
		isContainer,
		maxHeight,
		selectId,
		subGridIndexKey
	} from './appUtils'
	import Component from './component/Component.svelte'
	import ComponentWrapper from './component/ComponentWrapper.svelte'
	import GridViewer from './GridViewer.svelte'
	import GridEditorMenu from './GridEditorMenu.svelte'

	interface Props {
		containerHeight?: number | undefined
		containerWidth?: number | undefined
		class?: string
		style?: string
		noPadding?: boolean
		noYPadding?: boolean
		subGridId: string
		visible?: boolean
		id: string
		shouldHighlight?: boolean
	}

	let {
		containerHeight = undefined,
		containerWidth = undefined,
		class: classes = '',
		style = '',
		noPadding = false,
		noYPadding = false,
		subGridId,
		visible = true,
		id,
		shouldHighlight = true
	}: Props = $props()

	const dispatch = createEventDispatcher()

	const {
		app,
		connectingInput,
		selectedComponent,
		focusedGrid,
		mode,
		parentWidth,
		breakpoint,
		allIdsInPath,
		worldStore
	} = getContext<AppViewerContext>('AppViewerContext')

	const editorContext = getContext<AppEditorContext>('AppEditorContext')

	let isActive = $state(false)
	let sber = editorContext?.componentActive?.subscribe((x) => (isActive = x))

	let everVisible = $state(visible)

	$effect.pre(() => {
		visible && !everVisible && (everVisible = true)
	})

	onDestroy(() => {
		sber?.()
	})
	let highlight = $derived(id === $focusedGrid?.parentComponentId && shouldHighlight)

	const onpointerdown = (e) => {
		dispatch('focus')
	}

	function selectComponent(e: PointerEvent, id: string) {
		if (!$connectingInput.opened) {
			selectId(e, id, selectedComponent, app.val)
		}
	}

	function lock(dataItem: GridItem) {
		let fComponent = findGridItem(app.val, dataItem.id)
		if (fComponent) {
			fComponent = toggleFixed(fComponent)
		}
		app.val = app.val
	}

	let container: HTMLElement | undefined = $state(undefined)

	let maxRow = $derived(
		maxHeight(app.val.subgrids?.[subGridId] ?? [], containerHeight ?? 0, $breakpoint)
	)

	export function moveComponentBetweenSubgrids(
		componentId: string,
		parentComponentId: string,
		subGridIndex: number,
		position?: { x: number; y: number }
	) {
		// Find the component in the source subgrid
		const component = findGridItem(app.val, componentId)

		if (!component) {
			return
		}

		let parentGrid = findGridItemParentGrid(app.val, component.id)

		if (parentGrid) {
			app.val.subgrids &&
				(app.val.subgrids[parentGrid] = app.val.subgrids[parentGrid].filter(
					(item) => item.id !== component?.id
				))
		} else {
			app.val.grid = app.val.grid.filter((item) => item.id !== component?.id)
		}

		const gridItem = component

		insertNewGridItem(
			app.val,
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
		app.val = app.val

		if (parentGrid) {
			$focusedGrid = {
				parentComponentId,
				subGridIndex
			}

			$selectedComponent = [parentComponentId]
		} else {
			$focusedGrid = undefined
		}
	}

	export function moveToRoot(componentId: string, position?: { x: number; y: number }) {
		// Find the component in the source subgrid
		const component = findGridItem(app.val, componentId)

		if (!component) {
			return
		}

		let parentGrid = findGridItemParentGrid(app.val, component.id)

		if (parentGrid) {
			app.val.subgrids &&
				(app.val.subgrids[parentGrid] = app.val.subgrids[parentGrid].filter(
					(item) => item.id !== component?.id
				))
		} else {
			app.val.grid = app.val.grid.filter((item) => item.id !== component?.id)
		}

		const gridItem = component

		insertNewGridItem(
			app.val,
			(id) => ({ ...gridItem.data, id }),
			undefined,
			Object.fromEntries(gridColumns.map((column) => [column, gridItem[column]])),
			component.id,
			position,
			undefined,
			undefined,
			undefined,
			true
		)

		// Update the app state
		app.val = app.val
	}
</script>

<!-- {visible}
{everVisible} -->
{#if everVisible || app.val.eagerRendering}
	<div
		class="translate-x-0 translate-y-0 w-full subgrid {visible
			? 'visible'
			: 'invisible h-0 overflow-hidden'}"
		bind:this={container}
		{onpointerdown}
	>
		<div
			class={twMerge(
				$allIdsInPath.includes(id) && $mode == 'dnd' ? 'overflow-visible' : 'overflow-auto',
				noYPadding ? '' : 'py-2',
				classes ?? '',
				noPadding ? 'px-0' : 'px-2'
			)}
			style="{containerHeight ? `height: ${containerHeight - 2}px;` : ''} {style ?? ''}"
		>
			{#if $mode !== 'preview'}
				<div
					class={highlight
						? `outline !outline-dashed outline-2 min-h-full ${
								isActive && !$selectedComponent?.includes(id)
									? 'outline-orange-600'
									: 'outline-gray-400 dark:outline-gray-600'
							}`
						: ''}
				>
					<!-- A{JSON.stringify(app.val.subgrids?.[subGridId]) ?? []}B -->
					<Grid
						allIdsInPath={$allIdsInPath}
						items={app.val.subgrids?.[subGridId] ?? []}
						on:redraw={(e) => {
							push(editorContext?.history, app.val)
							if (app.val.subgrids) {
								app.val.subgrids[subGridId] = e.detail
							}
						}}
						selectedIds={$selectedComponent}
						scroller={container}
						parentWidth={$parentWidth - 17}
						{containerWidth}
						on:dropped={(e) => {
							const { id, overlapped, x, y } = e.detail

							if (!overlapped) {
								moveToRoot(id, { x, y })
							} else {
								const overlappedComponent = findGridItem(app.val, overlapped)

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
							}
						}}
						disableMove={!!$connectingInput.opened}
					>
						{#snippet children({ dataItem, overlapped, moveMode, componentDraggedId })}
							<ComponentWrapper
								id={dataItem.id}
								type={dataItem.data.type}
								class={classNames(
									'h-full w-full center-center',
									$selectedComponent?.includes(dataItem.id) ? 'active-grid-item' : '',
									'top-0'
								)}
							>
								<GridEditorMenu id={dataItem.id}>
									<Component
										{overlapped}
										fullHeight={dataItem?.[$breakpoint === 'sm' ? 3 : 12]?.fullHeight}
										render={visible}
										component={dataItem.data}
										selected={Boolean($selectedComponent?.includes(dataItem.id))}
										locked={isFixed(dataItem)}
										on:lock={() => lock(dataItem)}
										on:expand={() => {
											const parentGridItem = findGridItem(app.val, id)

											if (!parentGridItem) {
												return
											}

											$selectedComponent = [dataItem.id]
											push(editorContext?.history, app.val)

											expandGriditem(
												app.val.subgrids?.[subGridId] ?? [],
												dataItem.id,
												$breakpoint,
												parentGridItem
											)
											app.val = app.val
										}}
										on:fillHeight={() => {
											const gridItem = findGridItem(app.val, dataItem.id)
											const b = $breakpoint === 'sm' ? 3 : 12

											if (gridItem?.[b]) {
												gridItem[b].fullHeight = !gridItem[b].fullHeight
											}
											app.val = app.val
										}}
										{moveMode}
										{componentDraggedId}
									/>
								</GridEditorMenu>
							</ComponentWrapper>
						{/snippet}
					</Grid>
				</div>
			{:else}
				<GridViewer
					allIdsInPath={$allIdsInPath}
					items={app.val.subgrids?.[subGridId] ?? []}
					breakpoint={$breakpoint}
					parentWidth={$parentWidth - 17}
					{containerWidth}
					{maxRow}
				>
					{#snippet children({ dataItem })}
						<!-- svelte-ignore a11y_click_events_have_key_events -->
						<div
							onpointerdown={stopPropagation((e) =>
								selectComponent(e as PointerEvent, dataItem.id)
							)}
							class={classNames('h-full w-full center-center', 'top-0')}
						>
							<Component
								fullHeight={dataItem?.[$breakpoint === 'sm' ? 3 : 12]?.fullHeight}
								render={visible}
								component={dataItem.data}
								selected={Boolean($selectedComponent?.includes(dataItem.id))}
								locked={isFixed(dataItem)}
							/>
						</div>
					{/snippet}
				</GridViewer>
			{/if}
		</div>
	</div>
{:else if app.val.lazyInitRequire == undefined}
	{#each app.val?.subgrids?.[subGridId] ?? [] as item}
		<Component selected={false} fullHeight={false} render={false} component={item.data} />
	{/each}
{/if}
