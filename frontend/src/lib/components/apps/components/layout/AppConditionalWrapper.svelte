<script lang="ts">
	import { getContext } from 'svelte'
	import { initOutput } from '../../editor/appUtils'
	import SubGridEditor from '../../editor/SubGridEditor.svelte'
	import type { AppViewerContext, ComponentCustomCSS, RichConfiguration } from '../../types'
	import { initCss } from '../../utils'
	import InitializeComponent from '../helpers/InitializeComponent.svelte'
	import { InputValue } from '../helpers'
	import { twMerge } from 'tailwind-merge'
	import ResolveStyle from '../helpers/ResolveStyle.svelte'

	export let id: string
	export let componentContainerHeight: number
	export let customCss: ComponentCustomCSS<'conditionalwrapper'> | undefined = undefined
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

	let css = initCss($app.css?.conditionalwrapper, customCss)

	let resolvedConditions: boolean[] = []
	let selectedConditionIndex = 0

	function handleResolvedConditions() {
		const slicedArray = resolvedConditions.slice(0, conditions.length)
		const firstTrueIndex = slicedArray.findIndex((c) => c)

		outputs.conditions.set(slicedArray, true)

		setSelectedIndex(firstTrueIndex)
	}

	function setSelectedIndex(index: number) {
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
	<InputValue key="conditions" {id} input={condition} bind:value={resolvedConditions[index]} />
{/each}

{#each Object.keys(css ?? {}) as key (key)}
	<ResolveStyle
		{id}
		{customCss}
		{key}
		bind:css={css[key]}
		componentStyle={$app.css?.conditionalwrapper}
	/>
{/each}

<InitializeComponent {id} />

<div class="w-full h-full">
	{#if $app.subgrids}
		{#each resolvedConditions ?? [] as _res, i}
			<SubGridEditor
				visible={render && i == selectedConditionIndex}
				{id}
				class={twMerge(css?.container?.class, 'wm-conditional-tabs')}
				style={css?.container?.style}
				subGridId={`${id}-${i}`}
				containerHeight={componentContainerHeight}
				on:focus={() => {
					if (!$connectingInput.opened) {
						$selectedComponent = [id]
					}
					onFocus()
				}}
			/>
		{/each}
	{/if}
</div>
