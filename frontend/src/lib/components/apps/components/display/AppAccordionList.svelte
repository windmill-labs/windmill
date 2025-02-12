<script lang="ts">
	import { getContext } from 'svelte'
	import { initOutput } from '../../editor/appUtils'
	import SubGridEditor from '../../editor/SubGridEditor.svelte'
	import type { AppViewerContext, ComponentCustomCSS } from '../../types'
	import { initCss } from '../../utils'
	import InitializeComponent from '../helpers/InitializeComponent.svelte'
	import type { AppInput } from '../../inputType'
	import RunnableWrapper from '../helpers/RunnableWrapper.svelte'
	import ListWrapper from '../layout/ListWrapper.svelte'
	import { twMerge } from 'tailwind-merge'
	import ResolveStyle from '../helpers/ResolveStyle.svelte'

	export let id: string
	export let componentInput: AppInput | undefined
	export let customCss: ComponentCustomCSS<'accordionlistcomponent'> | undefined = undefined
	export let render: boolean
	export let initializing: boolean | undefined
	export let componentContainerHeight: number

	type AccordionListValue = { header: string; [key: string]: any }

	type InternalAccordionListInput = AppInput & {
		value: AccordionListValue[]
	}

	$: accordionInput = componentInput as InternalAccordionListInput

	const { app, focusedGrid, selectedComponent, worldStore, connectingInput } =
		getContext<AppViewerContext>('AppViewerContext')

	let everRender = render

	$: render && !everRender && (everRender = true)

	let activeIndex: number = 0

	const outputs = initOutput($worldStore, id, {
		result: undefined,
		activeIndex: 0,
		loading: false,
		inputs: {}
	})

	function onFocus() {
		$focusedGrid = {
			parentComponentId: id,
			subGridIndex: 0
		}
	}

	let css = initCss($app.css?.accordionlistcomponent, customCss)
	let result: any[] | undefined = undefined

	let inputs = {}

	$: $selectedComponent?.includes(id) &&
		$focusedGrid === undefined &&
		($focusedGrid = {
			parentComponentId: id,
			subGridIndex: 0
		})

	function toggleAccordion(index: number) {
		activeIndex = activeIndex === index ? -1 : index
		outputs.activeIndex.set(activeIndex)
	}
</script>

{#each Object.keys(css ?? {}) as key (key)}
	<ResolveStyle
		{id}
		{customCss}
		{key}
		bind:css={css[key]}
		componentStyle={$app.css?.accordionlistcomponent}
	/>
{/each}

<InitializeComponent {id} />

<RunnableWrapper
	render={true}
	{outputs}
	autoRefresh
	{componentInput}
	{id}
	bind:initializing
	bind:result
>
	{#if everRender}
		<div class="w-full flex flex-col overflow-auto max-h-full">
			{#if $app.subgrids?.[`${id}-0`]}
				{#if Array.isArray(result) && result.length > 0}
					{#each result ?? [] as value, index}
						<div class="border-b">
							<button
								on:pointerdown|stopPropagation
								on:click={() => toggleAccordion(index)}
								class={twMerge(
									'w-full text-left bg-surface !truncate text-sm hover:text-primary px-1 py-2',
									'wm-tabs-alltabs',
									activeIndex === index
										? twMerge('bg-surface text-primary ', 'wm-tabs-selectedTab')
										: 'text-secondary'
								)}
							>
								<span class="mr-2 w-8 font-mono">{activeIndex === index ? '-' : '+'}</span>
								{result[index]?.header || `Header ${index}`}
							</button>
							<div class="overflow-auto w-full">
								<ListWrapper
									onSet={(id, value) => {
										if (!inputs[id]) {
											inputs[id] = { [index]: value }
										} else {
											inputs[id] = { ...inputs[id], [index]: value }
										}
										outputs?.inputs.set(inputs, true)
									}}
									onRemove={(id) => {
										if (inputs?.[id] == undefined) {
											return
										}
										if (index == 0) {
											delete inputs[id]
											inputs = { ...inputs }
										} else {
											delete inputs[id][index]
											inputs[id] = { ...inputs[id] }
										}
										outputs?.inputs.set(inputs, true)
									}}
									{value}
									{index}
								>
									<SubGridEditor
										{id}
										visible={render && index === activeIndex}
										class={twMerge(css?.container?.class, 'wm-accordion p-2')}
										style={css?.container?.style}
										subGridId={`${id}-0`}
										containerHeight={componentContainerHeight -
											(30 * accordionInput?.value.length + 40)}
										on:focus={() => {
											if (!$connectingInput.opened) {
												$selectedComponent = [id]
											}
											onFocus()
										}}
									/>
								</ListWrapper>
							</div>
						</div>
					{/each}
				{:else}
					<ListWrapper disabled value={undefined} index={0}>
						<SubGridEditor visible={false} {id} subGridId={`${id}-0`} />
					</ListWrapper>
					{#if !Array.isArray(result)}
						<div class="text-center text-tertiary">Input data is not an array</div>
					{/if}
				{/if}
			{/if}
		</div>
	{:else if $app.subgrids}
		<ListWrapper disabled value={undefined} index={0}>
			<SubGridEditor visible={false} {id} subGridId={`${id}-0`} />
		</ListWrapper>
	{/if}
</RunnableWrapper>
