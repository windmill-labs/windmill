<script lang="ts">
	import { getContext } from 'svelte'
	import type { AppEditorContext } from '../types'
	import Grid from 'svelte-grid'
	import ComponentEditor from './ComponentEditor.svelte'
	import { classNames } from '$lib/utils'
	import { columnConfiguration, disableDrag, enableDrag, isFixed, toggleFixed } from '../gridUtils'

	import RecomputeAllComponents from './RecomputeAllComponents.svelte'
	import type { Policy } from '$lib/gen'
	import HiddenComponent from '../components/HiddenComponent.svelte'

	export let policy: Policy

	const {
		selectedComponent,
		app,
		mode,
		connectingInput,
		staticOutputs,
		runnableComponents,
		lazyGrid,
		summary
	} = getContext<AppEditorContext>('AppEditorContext')

	// The drag is disabled when the user is connecting an input
	$: setAllDrags($mode === 'preview' || $connectingInput.opened)

	function setAllDrags(enable: boolean) {
		if (enable) {
			$app.grid.map((gridItem) => disableDrag(gridItem))
		} else {
			$app.grid.map((gridItem) => enableDrag(gridItem))
		}
	}

	function removeGridElement(component) {
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

			// Delete static inputs
			delete $staticOutputs[component.id]
			$staticOutputs = $staticOutputs

			delete $runnableComponents[component.id]
			$runnableComponents = $runnableComponents

			$selectedComponent = undefined
		}
	}

	function selectComponent(id: string) {
		if (!$connectingInput.opened) {
			$selectedComponent = id
		}
	}

	$: $app.grid && recomputeLazyGrid()

	let intervalId: NodeJS.Timeout | undefined = undefined
	function recomputeLazyGrid() {
		{
			if (intervalId) {
				clearTimeout(intervalId)
			}
			intervalId = setTimeout(() => {
				lazyGrid.set($app.grid)
				intervalId = undefined
			}, 100)
		}
	}

	let pointerdown = false
	const onpointerdown = () => {
		pointerdown = true
	}
	const onpointerup = () => {
		pointerdown = false
	}
</script>

<div class="pb-2 relative z-20">
	<div
		class="w-full flex justify-between border-b {$connectingInput?.opened
			? ''
			: 'bg-gray-50 '}px-4 py-2 items-center gap-4"
	>
		<h2 class="truncate">{$summary}</h2>
		{#if !$connectingInput.opened}
			<RecomputeAllComponents />
		{/if}
		<div class="text-2xs text-gray-600"
			>{policy.on_behalf_of ? `on behalf of ${policy.on_behalf_of_email}` : ''}</div
		>
	</div>
	<div
		class="px-4 pt-4 {$connectingInput?.opened ? '' : ''}"
		on:pointerdown={onpointerdown}
		on:pointerleave={onpointerup}
		on:pointerup={onpointerup}
	>
		<Grid
			bind:items={$app.grid}
			let:dataItem
			rowHeight={36}
			cols={columnConfiguration}
			fastStart={true}
			gap={[4, 2]}
		>
			{#each $lazyGrid as gridComponent (gridComponent.id)}
				{#if gridComponent.data.id === dataItem.data.id}
					<!-- svelte-ignore a11y-click-events-have-key-events -->
					{#if $connectingInput.opened}
						{#if $connectingInput.opened}
							<div
								on:pointerenter={() => ($connectingInput.hoveredComponent = gridComponent.data.id)}
								on:pointerleave={() => ($connectingInput.hoveredComponent = undefined)}
								class="absolute  w-full h-full bg-black border-2 bg-opacity-25 z-20 flex justify-center items-center"
							/>
						{/if}
						<div
							style="transform: translate(-50%, -50%);"
							class="absolute w-fit justify-center bg-indigo-500/90 left-[50%] top-[50%] z-50 px-6 rounded border text-white py-2 text-5xl center-center"
							>{dataItem.data.id}</div
						>
					{/if}
					<!-- svelte-ignore a11y-click-events-have-key-events -->
					<div
						on:pointerdown={() => {
							selectComponent(dataItem.data.id)
						}}
						class={classNames(
							'h-full w-full flex justify-center align-center items-center',
							gridComponent.data.card ? 'border border-gray-100' : ''
						)}
					>
						<ComponentEditor
							{pointerdown}
							bind:component={gridComponent.data}
							selected={$selectedComponent === dataItem.data.id}
							locked={isFixed(gridComponent)}
							on:delete={() => removeGridElement(gridComponent.data)}
							on:lock={() => {
								let fComponent = $app.grid.find((c) => c.id === gridComponent.id)
								if (fComponent) {
									fComponent = toggleFixed(fComponent)
								}
							}}
						/>
					</div>
				{/if}
			{/each}
		</Grid>
	</div>
</div>

{#if $app.hiddenInlineScripts}
	{#each $app.hiddenInlineScripts as script, index}
		{#if script}
			<HiddenComponent
				id={`bg_${index}`}
				inlineScript={script.inlineScript}
				name={script.name}
				bind:fields={script.fields}
				bind:staticOutputs={$staticOutputs[`bg_${index}`]}
			/>
		{/if}
	{/each}
{/if}

<style>
	:global(.svlt-grid-shadow) {
		/* Back shadow */
		background: rgb(147 197 253) !important;
	}
	:global(.svlt-grid-active) {
		opacity: 1 !important;
	}
</style>
