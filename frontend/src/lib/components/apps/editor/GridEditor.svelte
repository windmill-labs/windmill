<script lang="ts">
	import { getContext } from 'svelte'
	import type { AppEditorContext, InlineScript } from '../types'
	import Grid from 'svelte-grid'
	import ComponentEditor from './ComponentEditor.svelte'
	import { classNames } from '$lib/utils'
	import {
		columnConfiguration,
		disableDrag,
		enableDrag,
		gridColumns,
		isFixed,
		toggleFixed
	} from '../gridUtils'
	import { Alert } from '$lib/components/common'
	import { fly } from 'svelte/transition'
	import gridHelp from 'svelte-grid/build/helper/index.mjs'

	import Button from '$lib/components/common/button/Button.svelte'
	import RecomputeAllComponents from './RecomputeAllComponents.svelte'

	const { selectedComponent, app, mode, connectingInput, staticOutputs, runnableComponents } =
		getContext<AppEditorContext>('AppEditorContext')

	// The drag is disabled when the user is connecting an input
	$: if ($mode === 'preview' || $connectingInput.opened) {
		$app.grid.map((gridItem) => disableDrag(gridItem))
	} else {
		$app.grid.map((gridItem) => enableDrag(gridItem))
	}

	function deleteComponent(component) {
		if (component) {
			$app.grid = $app.grid.filter((gridComponent) => {
				if (gridComponent.data.id === component.id) {
					if (
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

			gridColumns.forEach((colIndex) => {
				$app.grid = gridHelp.adjust($app.grid, colIndex)
			})

			// Delete static inputs
			delete $staticOutputs[component.id]
			$staticOutputs = $staticOutputs

			delete $runnableComponents[component.id]
			$runnableComponents = $runnableComponents

			$selectedComponent = undefined
		}
	}
</script>

<div class="bg-white h-full relative">
	<RecomputeAllComponents />
	<Grid
		bind:items={$app.grid}
		let:dataItem
		rowHeight={64}
		cols={columnConfiguration}
		fastStart={true}
		throttleUpdate={50}
		on:pointerup={({ detail }) => {
			if (!$connectingInput.opened) {
				$selectedComponent = detail.id
			}
		}}
	>
		{#each $app.grid as gridComponent (gridComponent.id)}
			{#if gridComponent.data.id === dataItem.data.id}
				<div
					class={classNames(
						'h-full w-full flex justify-center align-center',
						gridComponent.data.card ? 'border border-gray-100' : ''
					)}
				>
					<ComponentEditor
						bind:component={gridComponent.data}
						selected={$selectedComponent === dataItem.data.id}
						on:delete={() => deleteComponent(gridComponent.data)}
						on:lock={() => {
							gridComponent = toggleFixed(gridComponent)
						}}
						locked={isFixed(gridComponent)}
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
</style>
