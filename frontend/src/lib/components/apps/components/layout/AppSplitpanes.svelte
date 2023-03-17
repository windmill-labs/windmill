<script lang="ts">
	import { getContext } from 'svelte'
	import SubGridEditor from '../../editor/SubGridEditor.svelte'
	import type { AppInput } from '../../inputType'
	import type {
		AppEditorContext,
		AppViewerContext,
		ComponentCustomCSS,
		RichConfigurations
	} from '../../types'
	import { concatCustomCss } from '../../utils'
	import InputValue from '../helpers/InputValue.svelte'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import { deepEqual } from 'fast-equals'

	export let id: string
	export let componentContainerHeight: number
	export let customCss: ComponentCustomCSS<'container'> | undefined = undefined
	export let horizontal: boolean = false
	export let panes: number[]
	export let render: boolean

	const { app, focusedGrid, selectedComponent, componentControl } =
		getContext<AppViewerContext>('AppViewerContext')

	function onFocus() {
		$focusedGrid = {
			parentComponentId: id,
			subGridIndex: 0
		}
	}

	$: $selectedComponent === id && onFocus()
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

<div class="h-full w-full border" on:pointerdown|stopPropagation>
	<Splitpanes {horizontal}>
		{#each sumedup as paneSize, index (index)}
			<Pane size={paneSize} minSize={20}>
				{#if $app.subgrids?.[`${id}-${index}`]}
					<SubGridEditor
						visible={render}
						{id}
						shouldHighlight={$focusedGrid?.subGridIndex === index}
						class={css?.container.class}
						style={css?.container.style}
						bind:subGrid={$app.subgrids[`${id}-${index}`]}
						containerHeight={horizontal ? undefined : componentContainerHeight - 8}
						on:focus={() => {
							$selectedComponent = id
							$focusedGrid = {
								parentComponentId: id,
								subGridIndex: index
							}
						}}
					/>
				{/if}
			</Pane>
		{/each}
	</Splitpanes>
</div>
