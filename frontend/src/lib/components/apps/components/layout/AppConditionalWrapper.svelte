<script lang="ts">
	import { getContext } from 'svelte'
	import { initOutput } from '../../editor/appUtils'
	import SubGridEditor from '../../editor/SubGridEditor.svelte'
	import type { AppViewerContext, ComponentCustomCSS, RichConfiguration } from '../../types'
	import { concatCustomCss } from '../../utils'
	import InitializeComponent from '../helpers/InitializeComponent.svelte'
	import { InputValue } from '../helpers'

	export let id: string
	export let componentContainerHeight: number
	export let customCss: ComponentCustomCSS<'containercomponent'> | undefined = undefined
	export let render: boolean
	export let conditions: RichConfiguration[]

	const { app, focusedGrid, selectedComponent, worldStore, connectingInput, componentControl } =
		getContext<AppViewerContext>('AppViewerContext')

	const outputs = initOutput($worldStore, id, {
		conditions: [] as boolean[],
		selectedConditionIndex: 0
	})

	function onFocus() {
		$focusedGrid = {
			parentComponentId: id,
			subGridIndex: selectedConditionIndex
		}
	}

	$: css = concatCustomCss($app.css?.containercomponent, customCss)

	let resolvedConditions: boolean[] = []
	let selectedConditionIndex = 0

	function handleResolvedConditions() {
		const slicedArray = resolvedConditions.slice(0, conditions.length)
		const firstTrueIndex = slicedArray.findIndex((c) => c)

		outputs.conditions.set(slicedArray, true)

		setSelectedIndex(firstTrueIndex)
	}

	function setSelectedIndex(index: number) {
		console.trace('setSelectedIndex', index)
		$focusedGrid = {
			parentComponentId: id,
			subGridIndex: index
		}

		selectedConditionIndex = index
		outputs.selectedConditionIndex.set(index)
	}

	$: resolvedConditions && handleResolvedConditions()

	$componentControl[id] = {
		setTab: (conditionIndex: number) => {
			if (conditionIndex === -1) {
				handleResolvedConditions()
			} else {
				setSelectedIndex(conditionIndex)
			}
		}
	}
</script>

{#each conditions ?? [] as condition, index}
	<InputValue {id} input={condition} bind:value={resolvedConditions[index]} />
{/each}

<InitializeComponent {id} />

<div class="w-full h-full">
	{#if $app.subgrids?.[`${id}-${selectedConditionIndex}`]}
		<SubGridEditor
			visible={render}
			{id}
			class={css?.container?.class}
			style={css?.container?.style}
			subGridId={`${id}-${selectedConditionIndex}`}
			containerHeight={componentContainerHeight}
			on:focus={() => {
				if (!$connectingInput.opened) {
					$selectedComponent = [id]
				}
				onFocus()
			}}
		/>
	{/if}
</div>
