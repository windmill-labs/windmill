<script lang="ts">
	import { getContext, untrack } from 'svelte'
	import type { AppEditorContext, AppViewerContext } from '../types'
	import { gridColumns, isFixed, toggleFixed } from '../gridUtils'
	import { twMerge } from 'tailwind-merge'

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

	interface Props {
		policy: Policy
	}

	let { policy }: Props = $props()

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

	const { history, componentActive } = getContext<AppEditorContext>('AppEditorContext')

	let previousSelectedIds: string[] | undefined = $state(undefined)
	$effect(() => {
		if (!deepEqual(previousSelectedIds, $selectedComponent)) {
			untrack(() => {
				previousSelectedIds = $selectedComponent
				$allIdsInPath = ($selectedComponent ?? [])
					.flatMap((id) => dfs($app.grid, id, $app.subgrids ?? {}))
					.filter((x) => x != undefined) as string[]
			})
		}
	})

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
		$app = $app

		$selectedComponent = [parentComponentId]
		$focusedGrid = {
			parentComponentId,
			subGridIndex
		}
	}
</script>

<input />
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
						{#snippet text()}
							<span
								><div class="flex flex-col">
									{#each $bgRuns as bgRun}
										<div class="flex gap-2 items-center">
											<div class="text-2xs text-tertiary">{bgRun}</div>
										</div>
									{/each}
								</div></span
							>
						{/snippet}
					</Popover>
				{:else}
					<span class="w-9"></span>
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
	<!-- svelte-ignore a11y_click_events_have_key_events -->
	<div
		style={$app.css?.['app']?.['grid']?.style}
		class={twMerge(
			'p-2 overflow-visible z-50',
			$app.css?.['app']?.['grid']?.class ?? '',
			'wm-app-grid !static h-full w-full'
		)}
		onpointerdown={() => {
			$selectedComponent = undefined
			$focusedGrid = undefined
		}}
		bind:clientWidth={$parentWidth}
	>
		<div
			class="subgrid overflow-visible z-100 outline-dashed outline-2 outline-offset-4 outline-gray-300 dark:outline-gray-500"
			style={`transform: scale(1);`}
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
				{#snippet children({ dataItem, overlapped, componentDraggedId })}
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
								on:expand={() => {
									push(history, $app)
									$selectedComponent = [dataItem.id]
									expandGriditem($app.grid, dataItem.id, $breakpoint)
									$app = $app
								}}
								{overlapped}
								{componentDraggedId}
							/>
						</GridEditorMenu>
					</ComponentWrapper>
				{/snippet}
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
