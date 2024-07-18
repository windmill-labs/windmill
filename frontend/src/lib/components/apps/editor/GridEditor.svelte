<script lang="ts">
	import { getContext } from 'svelte'
	import type { AppEditorContext, AppViewerContext } from '../types'
	import { columnConfiguration, isFixed, toggleFixed } from '../gridUtils'
	import { twMerge } from 'tailwind-merge'

	import HiddenComponent from '../components/helpers/HiddenComponent.svelte'
	import Component from './component/Component.svelte'
	import { push } from '$lib/history'
	import { dfs, expandGriditem, findGridItem } from './appUtils'
	import Grid from '../svelte-grid/Grid.svelte'
	import { deepEqual } from 'fast-equals'
	import ComponentWrapper from './component/ComponentWrapper.svelte'
	import { classNames } from '$lib/utils'
	import { BG_PREFIX } from '../utils'
	import GridEditorMenu from './GridEditorMenu.svelte'

	const { selectedComponent, app, mode, focusedGrid, parentWidth, breakpoint, allIdsInPath } =
		getContext<AppViewerContext>('AppViewerContext')

	const { history, scale } = getContext<AppEditorContext>('AppEditorContext')

	let previousSelectedIds: string[] | undefined = undefined
	$: if (!deepEqual(previousSelectedIds, $selectedComponent)) {
		previousSelectedIds = $selectedComponent
		$allIdsInPath = ($selectedComponent ?? [])
			.flatMap((id) => dfs($app.grid, id, $app.subgrids ?? {}))
			.filter((x) => x != undefined) as string[]
	}
</script>

<div class="w-full z-[1000] overflow-visible h-full">
	<!-- svelte-ignore a11y-click-events-have-key-events -->
	<div
		style={$app.css?.['app']?.['grid']?.style}
		class={twMerge(
			'p-2 overflow-visible z-50',
			$app.css?.['app']?.['grid']?.class ?? '',
			'wm-app-grid !static h-full w-full'
		)}
		on:pointerdown={() => {
			$selectedComponent = undefined
			$focusedGrid = undefined
		}}
		bind:clientWidth={$parentWidth}
	>
		<div
			class={twMerge(
				!$focusedGrid && $mode !== 'preview' ? 'outline-dashed' : '',
				'subgrid  overflow-visible  z-100',
				'outline-[#999999] dark:outline-[#aaaaaa] outline-dotted outline-offset-2 outline-1 '
			)}
			style={`transform: scale(${$scale / 100});`}
		>
			<Grid
				allIdsInPath={$allIdsInPath}
				selectedIds={$selectedComponent}
				items={$app.grid}
				on:redraw={(e) => {
					push(history, $app)
					$app.grid = e.detail
				}}
				let:dataItem
				let:hidden
				cols={columnConfiguration}
			>
				<ComponentWrapper
					id={dataItem.id}
					type={dataItem.data.type}
					class={classNames(
						'h-full w-full center-center outline outline-surface-secondary',
						Boolean($selectedComponent?.includes(dataItem.id)) ? 'active-grid-item' : ''
					)}
				>
					<GridEditorMenu id={dataItem.id}>
						<Component
							{hidden}
							render={true}
							component={dataItem.data}
							selected={Boolean($selectedComponent?.includes(dataItem.id))}
							locked={isFixed(dataItem)}
							fullHeight={dataItem?.[$breakpoint === 'sm' ? 3 : 12]?.fullHeight}
							on:lock={() => {
								const gridItem = findGridItem($app, dataItem.id)
								if (gridItem) {
									toggleFixed(gridItem)
								}
								$app = $app
							}}
							on:expand={() => {
								push(history, $app)
								$selectedComponent = [dataItem.id]
								expandGriditem($app.grid, dataItem.id, $breakpoint)
								$app = $app
							}}
							on:fillHeight={() => {
								const gridItem = findGridItem($app, dataItem.id)
								const b = $breakpoint === 'sm' ? 3 : 12
								if (gridItem?.[b]) {
									gridItem[b].fullHeight = !gridItem[b].fullHeight
								}
								$app = $app
							}}
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
	.svlt-grid-shadow {
		/* Back shadow */
		background: #93c4fdd0 !important;
	}
	.svlt-grid-active {
		opacity: 1 !important;
	}
</style>
