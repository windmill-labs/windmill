<script lang="ts">
	import { classNames } from '$lib/utils'
	import { createEventDispatcher, getContext } from 'svelte'
	import Grid from '@windmill-labs/svelte-grid'
	import { columnConfiguration, isFixed, toggleFixed } from '../gridUtils'
	import type { AppEditorContext, GridItem } from '../types'
	import Component from './component/Component.svelte'

	export let containerHeight: number
	export let noPadding = false
	//export let id: string
	export let subGrid: GridItem[] = []

	const dispatch = createEventDispatcher()

	const { app, connectingInput, selectedComponent, focusedGrid } =
		getContext<AppEditorContext>('AppEditorContext')

	let pointerdown = false

	let onComponent: string | undefined = undefined

	const onpointerdown = (e) => {
		if (onComponent === undefined) {
			dispatch('focus')
		} else {
			onComponent = undefined
		}
		pointerdown = true
	}

	const onpointerup = () => {
		pointerdown = false
	}

	function selectComponent(id: string) {
		onComponent = id
		if (!$connectingInput.opened) {
			$selectedComponent = id
			/*
			$focusedGrid = {
				parentComponentId: parentId,
				subGridIndex: index
			}
			*/
		}
	}

	function lock(gridComponent: GridItem) {
		let fComponent = $app.grid.find((c) => c.id === gridComponent.id)
		if (fComponent) {
			fComponent = toggleFixed(fComponent)
		}
	}

	// @ts-ignore
	let container
</script>

<div class="relative w-full subgrid " bind:this={container}>
	<div
		class:px-2={!noPadding}
		class="py-2 overflow-auto  {$connectingInput?.opened ? '' : ''}"
		on:pointerdown|stopPropagation={onpointerdown}
		on:pointerleave={onpointerup}
		on:pointerup={onpointerup}
		style="height: {containerHeight}px;"
	>
		<div>
			<Grid
				bind:items={subGrid}
				let:dataItem
				rowHeight={36}
				cols={columnConfiguration}
				fastStart={true}
				gap={[4, 2]}
				scroller={container}
			>
				{#each subGrid as gridComponent (gridComponent.id)}
					{#if gridComponent.data.id === dataItem.data.id}
						<!-- svelte-ignore a11y-click-events-have-key-events -->
						{#if $connectingInput.opened}
							{#if $connectingInput.opened}
								<div
									on:pointerenter={() =>
										($connectingInput.hoveredComponent = gridComponent.data.id)}
									on:pointerleave={() => ($connectingInput.hoveredComponent = undefined)}
									class="absolute w-full h-full bg-black border-2 bg-opacity-25 z-20 flex justify-center items-center"
								/>
							{/if}
							<div
								style="transform: translate(-50%, -50%);"
								class="absolute w-fit justify-center bg-indigo-500/90 left-[50%] top-[50%] z-50 px-6 rounded border text-white py-2 text-5xl center-center"
							>
								{dataItem.data.id}
							</div>
						{/if}

						<!-- svelte-ignore a11y-click-events-have-key-events -->
						<div
							on:pointerdown={() => selectComponent(dataItem.data.id)}
							class={classNames(
								'h-full w-full center-center',
								$selectedComponent === dataItem.data.id ? 'active-grid-item' : '',
								gridComponent.data.card ? 'border border-gray-100' : '',
								'top-0'
							)}
						>
							<Component
								{pointerdown}
								bind:component={gridComponent.data}
								selected={$selectedComponent === dataItem.data.id}
								locked={isFixed(gridComponent)}
								on:lock={() => lock(gridComponent)}
							/>
						</div>
					{/if}
				{/each}
			</Grid>
		</div>
	</div>
</div>
