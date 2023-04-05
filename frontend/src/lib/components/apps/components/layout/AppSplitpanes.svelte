<script lang="ts">
	import { getContext } from 'svelte'
	import SubGridEditor from '../../editor/SubGridEditor.svelte'
	import type { AppViewerContext, ComponentCustomCSS } from '../../types'
	import { concatCustomCss } from '../../utils'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import { deepEqual } from 'fast-equals'
	import { initOutput } from '../../editor/appUtils'

	export let id: string
	export let componentContainerHeight: number
	export let customCss:
		| ComponentCustomCSS<'horizontalsplitpanescomponent' | 'verticalsplitpanescomponent'>
		| undefined = undefined
	export let horizontal: boolean = false
	export let panes: number[]
	export let render: boolean

	const { app, focusedGrid, selectedComponent, componentControl, worldStore, connectingInput } =
		getContext<AppViewerContext>('AppViewerContext')

	//used so that we can count number of outputs setup for first refresh
	initOutput($worldStore, id, {})

	function onFocus() {
		$focusedGrid = {
			parentComponentId: id,
			subGridIndex: 0
		}
	}

	$: css = concatCustomCss($app.css?.containercomponent, customCss)

	$componentControl[id] = {
		left: () => {
			if ($focusedGrid?.subGridIndex) {
				const index = $focusedGrid?.subGridIndex ?? 0
				if (index > 0) {
					$focusedGrid.subGridIndex = index - 1
					return true
				}
			}
			return false
		},
		right: () => {
			// subGridIndex can be 0
			if ($focusedGrid?.subGridIndex !== undefined) {
				const index = $focusedGrid?.subGridIndex ?? 0

				if (index < panes.length - 1) {
					$focusedGrid.subGridIndex = index + 1
					return true
				}
			}
			return false
		}
	}

	let sumedup = [50, 50]
	$: {
		let ns = panes.map((x) => (x / panes.reduce((a, b) => a + b, 0)) * 100)
		if (!deepEqual(ns, sumedup)) {
			sumedup = ns
		}
	}
</script>

<div class="h-full w-full border" on:pointerdown={onFocus}>
	<Splitpanes {horizontal}>
		{#each sumedup as paneSize, index (index)}
			<Pane size={paneSize} minSize={20}>
				<div
					class="w-full h-full"
					on:pointerdown|stopPropagation={() => {
						$selectedComponent = [id]
						$focusedGrid = {
							parentComponentId: id,
							subGridIndex: index
						}
					}}
				>
					{#if $app.subgrids?.[`${id}-${index}`]}
						<SubGridEditor
							visible={render}
							{id}
							shouldHighlight={$focusedGrid?.subGridIndex === index}
							class={css?.container?.class}
							style={css?.container?.style}
							subGridId={`${id}-${index}`}
							containerHeight={horizontal ? undefined : componentContainerHeight - 8}
							on:focus={() => {
								if (!$connectingInput.opened) {
									$selectedComponent = [id]
									$focusedGrid = {
										parentComponentId: id,
										subGridIndex: index
									}
								}
							}}
						/>
					{/if}
				</div>
			</Pane>
		{/each}
	</Splitpanes>
</div>
