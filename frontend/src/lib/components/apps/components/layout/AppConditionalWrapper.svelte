<script lang="ts">
	import { getContext } from 'svelte'
	import { initOutput } from '../../editor/appUtils'
	import SubGridEditor from '../../editor/SubGridEditor.svelte'
	import type { AppViewerContext, ComponentCustomCSS } from '../../types'
	import { concatCustomCss } from '../../utils'
	import InitializeComponent from '../helpers/InitializeComponent.svelte'
	import type { AppInputSpec } from '../../inputType'
	import { InputValue } from '../helpers'

	export let id: string
	export let componentContainerHeight: number
	export let customCss: ComponentCustomCSS<'containercomponent'> | undefined = undefined
	export let render: boolean
	export let conditions: AppInputSpec<'boolean', boolean>[]

	const { app, focusedGrid, selectedComponent, worldStore, connectingInput, componentControl } =
		getContext<AppViewerContext>('AppViewerContext')

	const outputs = initOutput($worldStore, id, {
		conditions: [] as boolean[],
		selectedConditionIndex: 0
	})

	function onFocus() {
		$focusedGrid = {
			parentComponentId: id,
			subGridIndex: 0
		}
	}

	$: css = concatCustomCss($app.css?.containercomponent, customCss)

	let resolvedConditions: boolean[] = []
	let selectedConditionIndex = 0
	let forcedIndex: number = -1

	function handleResolvedConditions() {
		const slicedArray = resolvedConditions.slice(0, conditions.length)
		outputs.conditions.set(slicedArray, true)

		const firstTrueIndex = forcedIndex !== -1 ? forcedIndex : slicedArray.findIndex((c) => c)

		if ($focusedGrid?.parentComponentId === id) {
			$focusedGrid = {
				parentComponentId: id,
				subGridIndex: firstTrueIndex
			}
		}

		selectedConditionIndex = firstTrueIndex
		outputs.selectedConditionIndex.set(firstTrueIndex)
	}

	$: (resolvedConditions || conditions) && handleResolvedConditions()

	$componentControl[id] = {
		setTab: (conditionIndex: number) => {
			forcedIndex = conditionIndex
			handleResolvedConditions()
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
