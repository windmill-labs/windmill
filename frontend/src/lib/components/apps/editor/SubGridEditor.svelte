<script lang="ts">
	import { classNames } from '$lib/utils'
	import { createEventDispatcher, getContext } from 'svelte'
	import Grid from '@windmill-labs/svelte-grid'
	import { twMerge } from 'tailwind-merge'
	import { columnConfiguration, isFixed, toggleFixed } from '../gridUtils'
	import type { AppEditorContext, GridItem } from '../types'
	import Component from './component/Component.svelte'
	import { findGridItem } from './appUtils'

	export let containerHeight: number
	let classes = ''
	export { classes as class }
	export let style = ''
	export let noPadding = false
	export let subGrid: GridItem[] = []
	export let visible: boolean = true
	export let id: string
	export let shouldHighlight: boolean = true

	const dispatch = createEventDispatcher()

	const { app, connectingInput, selectedComponent, focusedGrid, mode } =
		getContext<AppEditorContext>('AppEditorContext')

	$: highlight = id === $focusedGrid?.parentComponentId && shouldHighlight

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
		let fComponent = findGridItem($app, gridComponent.data.id)
		if (fComponent) {
			fComponent = toggleFixed(fComponent)
		}
	}

	// @ts-ignore
	let container
</script>

<div
	class="relative w-full subgrid {visible ? 'visible' : 'invisible h-0 overflow-hidden'} 	"
	bind:this={container}
>
	<div
		class={twMerge('py-2 overflow-auto', classes ?? '', noPadding ? 'px-0' : 'px-2')}
		on:pointerdown|stopPropagation={onpointerdown}
		on:pointerleave={onpointerup}
		on:pointerup={onpointerup}
		style="height: {containerHeight}px; {style ?? ''}"
	>
		<div
			class={highlight && $mode !== 'preview' ? 'border-gray-400 border border-dashed h-full' : ''}
		>
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
					{#if gridComponent?.data?.id && gridComponent?.data?.id === dataItem?.data?.id}
						<!-- svelte-ignore a11y-click-events-have-key-events -->
						{#if $connectingInput.opened}
							<div
								on:pointerenter={() => ($connectingInput.hoveredComponent = gridComponent.data.id)}
								on:pointerleave={() => ($connectingInput.hoveredComponent = undefined)}
								class="absolute w-full h-full bg-black border-2 bg-opacity-25 z-20 flex justify-center items-center"
							/>
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
