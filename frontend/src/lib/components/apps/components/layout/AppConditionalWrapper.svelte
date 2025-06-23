<script lang="ts">
	import { getContext, untrack } from 'svelte'
	import { initOutput } from '../../editor/appUtils'
	import SubGridEditor from '../../editor/SubGridEditor.svelte'
	import type { AppViewerContext, ComponentCustomCSS, RichConfiguration } from '../../types'
	import { initCss } from '../../utils'
	import InitializeComponent from '../helpers/InitializeComponent.svelte'
	import { twMerge } from 'tailwind-merge'
	import ResolveStyle from '../helpers/ResolveStyle.svelte'
	import InputValue from '../helpers/InputValue.svelte'

	interface Props {
		id: string
		componentContainerHeight: number
		customCss?: ComponentCustomCSS<'conditionalwrapper'> | undefined
		render: boolean
		conditions: RichConfiguration[]
		onTabChange?: string[] | undefined
	}

	let {
		id,
		componentContainerHeight,
		customCss = undefined,
		render,
		conditions,
		onTabChange = undefined
	}: Props = $props()

	const {
		app,
		focusedGrid,
		selectedComponent,
		worldStore,
		connectingInput,
		componentControl,
		runnableComponents
	} = getContext<AppViewerContext>('AppViewerContext')

	const outputs = initOutput($worldStore, id, {
		conditions: [] as boolean[],
		selectedTabIndex: 0
	})

	let everRender = $state(render)
	$effect.pre(() => {
		render && !everRender && (everRender = true)
	})

	function onFocus() {
		$focusedGrid = {
			parentComponentId: id,
			subGridIndex: selectedConditionIndex
		}
	}

	let css = $state(initCss($app.css?.conditionalwrapper, customCss))

	let resolvedConditions: boolean[] = $state(conditions.map((_x) => false))
	let selectedConditionIndex = $state(0)

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
		outputs.selectedTabIndex.set(index)
		onTabChange?.forEach((id) => $runnableComponents?.[id]?.cb?.forEach((cb) => cb?.()))
	}

	$effect.pre(() => {
		resolvedConditions && resolvedConditions.forEach((c) => c)
		resolvedConditions && untrack(() => handleResolvedConditions())
	})

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
	<InputValue
		key="condition{index + 1}"
		{id}
		input={condition}
		bind:value={resolvedConditions[index]}
	/>
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

{#if everRender}
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
					on:focus={(e) => {
						if (!$connectingInput.opened) {
							$selectedComponent = [id]
						}
						onFocus()
					}}
				/>
			{/each}
		{/if}
	</div>
{:else if $app.subgrids}
	{#each resolvedConditions ?? [] as _res, i}
		<SubGridEditor visible={false} {id} subGridId={`${id}-${i}`} />
	{/each}
{/if}
