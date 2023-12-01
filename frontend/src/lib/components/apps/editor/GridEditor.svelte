<script lang="ts">
	import { getContext } from 'svelte'
	import type { AppEditorContext, AppViewerContext } from '../types'
	import { columnConfiguration, isFixed, toggleFixed } from '../gridUtils'
	import { twMerge } from 'tailwind-merge'

	import RecomputeAllComponents from './RecomputeAllComponents.svelte'
	import type { Policy } from '$lib/gen'
	import HiddenComponent from '../components/helpers/HiddenComponent.svelte'
	import Component from './component/Component.svelte'
	import { push } from '$lib/history'
	import { dfs, expandGriditem, findGridItem } from './appUtils'
	import Grid from '../svelte-grid/Grid.svelte'
	import { deepEqual } from 'fast-equals'
	import ComponentWrapper from './component/ComponentWrapper.svelte'
	import { classNames } from '$lib/utils'
	import Toggle from '$lib/components/Toggle.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import { BG_PREFIX } from '../utils'
	import { Loader2 } from 'lucide-svelte'
	import Popover from '$lib/components/Popover.svelte'

	export let policy: Policy

	const {
		selectedComponent,
		app,
		mode,
		connectingInput,
		summary,
		focusedGrid,
		parentWidth,
		breakpoint,
		allIdsInPath,
		bgRuns
	} = getContext<AppViewerContext>('AppViewerContext')

	const { history, scale, componentActive } = getContext<AppEditorContext>('AppEditorContext')

	let previousSelectedIds: string[] | undefined = undefined
	$: if (!deepEqual(previousSelectedIds, $selectedComponent)) {
		previousSelectedIds = $selectedComponent
		$allIdsInPath = ($selectedComponent ?? [])
			.flatMap((id) => dfs($app.grid, id, $app.subgrids ?? {}))
			.filter((x) => x != undefined) as string[]
	}
</script>

<div class="w-full z-[1000] overflow-visible h-full">
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
				<Tooltip
					>The scripts will be run on behalf of the author and a tight policy ensure security about
					the possible inputs of the runnables.</Tooltip
				>
			</div>
		</div>
	</div>

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
				rowHeight={36}
				cols={columnConfiguration}
				gap={[4, 2]}
			>
				<ComponentWrapper
					id={dataItem.id}
					type={dataItem.data.type}
					class={classNames(
						'h-full w-full center-center outline outline-surface-secondary',
						Boolean($selectedComponent?.includes(dataItem.id)) ? 'active-grid-item' : ''
					)}
				>
					<Component
						render={true}
						component={dataItem.data}
						selected={Boolean($selectedComponent?.includes(dataItem.id))}
						locked={isFixed(dataItem)}
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
					/>
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
