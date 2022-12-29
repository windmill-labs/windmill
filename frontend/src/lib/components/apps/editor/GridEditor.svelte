<script lang="ts">
	import { getContext } from 'svelte'
	import type { AppEditorContext } from '../types'
	import Grid from 'svelte-grid'
	import ComponentEditor from './ComponentEditor.svelte'
	import { classNames } from '$lib/utils'
	import { columnConfiguration, disableDrag, enableDrag, isFixed, toggleFixed } from '../gridUtils'
	import { Alert } from '$lib/components/common'
	import { fly } from 'svelte/transition'

	import Button from '$lib/components/common/button/Button.svelte'
	import RecomputeAllComponents from './RecomputeAllComponents.svelte'

	const {
		selectedComponent,
		app,
		mode,
		connectingInput,
		staticOutputs,
		runnableComponents,
		lazyGrid
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
				if (JSON.stringify($app.grid) != JSON.stringify($lazyGrid)) {
					lazyGrid.set($app.grid)
				}
				intervalId = undefined
			}, 50)
		}
	}
</script>

<div class="bg-white h-full relative">
	<RecomputeAllComponents />
	<Grid
		bind:items={$app.grid}
		let:dataItem
		rowHeight={46}
		cols={columnConfiguration}
		fastStart={true}
		on:pointerup={({ detail }) => selectComponent(detail.id)}
		gap={[4, 2]}
	>
		{#each $lazyGrid as gridComponent (gridComponent.id)}
			{#if gridComponent.data.id === dataItem.data.id}
				<!-- svelte-ignore a11y-click-events-have-key-events -->
				<div
					class={classNames(
						'h-full w-full flex justify-center align-center',
						gridComponent.data.card ? 'border border-gray-100' : '',
						'z-50'
					)}
					on:click|preventDefault|capture|once
				>
					<ComponentEditor
						bind:component={gridComponent.data}
						selected={$selectedComponent === dataItem.data.id}
						locked={isFixed(gridComponent)}
						on:delete={() => removeGridElement(gridComponent.data)}
						on:lock={() => {
							gridComponent = toggleFixed(gridComponent)
						}}
					/>
				</div>
			{/if}
		{/each}
	</Grid>

	{#if $connectingInput.opened}
		<div
			class="fixed top-32  z-10 flex justify-center items-center"
			transition:fly={{ duration: 100, y: -100 }}
		>
			<Alert title="Connecting" type="info">
				<div class="flex gap-2 flex-col">
					Click on the output of the component you want to connect to on the left panel.
					<div>
						<Button
							color="blue"
							variant="border"
							size="xs"
							on:click={() => {
								$connectingInput.opened = false
								$connectingInput.input = undefined
							}}
						>
							Stop connecting
						</Button>
					</div>
				</div>
			</Alert>
		</div>
	{/if}
</div>

<style>
	:global(.svlt-grid-shadow) {
		/* Back shadow */
		background: rgb(147 197 253) !important;
	}
	:global(.svlt-grid-active) {
		opacity: 1 !important;
	}
</style>
