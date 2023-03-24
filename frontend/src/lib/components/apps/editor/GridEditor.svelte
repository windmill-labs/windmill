<script lang="ts">
	import { getContext } from 'svelte'
	import type { AppEditorContext, AppViewerContext } from '../types'
	import { classNames } from '$lib/utils'
	import { columnConfiguration, isFixed, toggleFixed } from '../gridUtils'
	import { twMerge } from 'tailwind-merge'

	import RecomputeAllComponents from './RecomputeAllComponents.svelte'
	import type { Policy } from '$lib/gen'
	import HiddenComponent from '../components/helpers/HiddenComponent.svelte'
	import Component from './component/Component.svelte'
	import { push } from '$lib/history'
	import { expandGriditem, findGridItem } from './appUtils'
	import Grid from '../svelte-grid/Grid.svelte'
	import type { AppComponent } from './component'

	export let policy: Policy

	const {
		selectedComponent,
		app,
		mode,
		connectingInput,
		runnableComponents,
		summary,
		focusedGrid,
		parentWidth,
		breakpoint
	} = getContext<AppViewerContext>('AppViewerContext')

	const { history } = getContext<AppEditorContext>('AppEditorContext')

	function removeGridElement(component: AppComponent) {
		if (component) {
			$app.grid = $app.grid.filter((gridComponent) => {
				if (gridComponent.data.id === component.id) {
					if (
						gridComponent.data.componentInput?.type === 'runnable' &&
						gridComponent.data.componentInput?.runnable?.type === 'runnableByName' &&
						gridComponent.data.componentInput?.runnable.inlineScript
					) {
						const { name, inlineScript } = gridComponent.data.componentInput?.runnable

						if (!$app.unusedInlineScripts) {
							$app.unusedInlineScripts = []
						}

						$app.unusedInlineScripts.push({
							name,
							inlineScript
						})

						$app = $app
					}
				}

				return gridComponent.data.id !== component?.id
			})

			delete $runnableComponents[component.id]
			$runnableComponents = $runnableComponents

			$selectedComponent = undefined
		}
	}

	function selectComponent(id: string) {
		if (!$connectingInput.opened) {
			$selectedComponent = id
			if ($focusedGrid?.parentComponentId != id) {
				$focusedGrid = undefined
			}
		}
	}
</script>

<div class="relative w-full z-20 overflow-visible">
	<div
		class="w-full sticky top-0 flex justify-between border-b {$connectingInput?.opened
			? ''
			: 'bg-gray-50 '} px-4 py-1 items-center gap-4"
		style="z-index: 1000;"
	>
		<h2 class="truncate">{$summary}</h2>
		{#if !$connectingInput.opened}
			<RecomputeAllComponents />
		{/if}
		<div class="flex text-2xs text-gray-600 gap-1 items-center">
			<div class="py-2 pr-2  text-gray-600 flex gap-2 items-center">
				Hide bar on view
				<input class="windmillapp" type="checkbox" bind:checked={$app.norefreshbar} />
			</div>
			<div>
				{policy.on_behalf_of ? `on behalf of ${policy.on_behalf_of_email}` : ''}
			</div>
		</div>
	</div>

	<!-- svelte-ignore a11y-click-events-have-key-events -->
	<div
		style={$app.css?.['app']?.['grid']?.style}
		class={twMerge(
			'px-4 pt-4 pb-2 overflow-visible h-full',
			$app.css?.['app']?.['grid']?.class ?? ''
		)}
		on:pointerdown={() => {
			$selectedComponent = undefined
			$focusedGrid = undefined
		}}
		bind:clientWidth={$parentWidth}
	>
		<div class={!$focusedGrid && $mode !== 'preview' ? 'border-gray-400 border border-dashed' : ''}>
			<Grid
				onTopId={$selectedComponent}
				items={$app.grid}
				on:redraw={(e) => {
					push(history, $app)
					$app.grid = e.detail
				}}
				let:dataItem
				rowHeight={36}
				cols={columnConfiguration}
				fastStart={true}
				gap={[4, 2]}
			>
				<!-- svelte-ignore a11y-click-events-have-key-events -->
				{#if $connectingInput.opened}
					<div
						on:pointerenter={() => ($connectingInput.hoveredComponent = dataItem.id)}
						on:pointerleave={() => ($connectingInput.hoveredComponent = undefined)}
						class="absolute w-full h-full bg-black border-2 bg-opacity-25 z-20 flex justify-center items-center"
					/>
					<div
						style="transform: translate(-50%, -50%);"
						class="absolute w-fit justify-center bg-indigo-500/90 left-[50%] top-[50%] z-50 px-6 rounded border text-white py-2 text-5xl center-center"
					>
						{dataItem.id}
					</div>
				{/if}
				<!-- svelte-ignore a11y-click-events-have-key-events -->
				<div
					on:pointerdown={() => selectComponent(dataItem.id)}
					class={classNames(
						'h-full w-full center-center',
						$selectedComponent === dataItem.id ? 'active-grid-item' : ''
					)}
				>
					<Component
						render={true}
						component={dataItem.data}
						selected={$selectedComponent === dataItem.id}
						locked={isFixed(dataItem)}
						on:delete={() => removeGridElement(dataItem.data)}
						on:lock={() => {
							const gridItem = findGridItem($app, dataItem.id)
							if (gridItem) {
								toggleFixed(gridItem)
							}
							$app = $app
						}}
						on:expand={() => {
							push(history, $app)
							$selectedComponent = dataItem.id
							expandGriditem($app.grid, dataItem.id, $breakpoint)
							$app = $app
						}}
					/>
				</div>
			</Grid>
		</div>
	</div>
</div>

{#if $app.hiddenInlineScripts}
	{#each $app.hiddenInlineScripts as script, index}
		{#if script}
			<HiddenComponent
				id={`bg_${index}`}
				inlineScript={script.inlineScript}
				name={script.name}
				fields={script.fields}
				doNotRecomputeOnInputChanged={script.doNotRecomputeOnInputChanged}
			/>
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
