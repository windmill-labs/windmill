<script lang="ts">
	import { getContext } from 'svelte'
	import SubGridEditor from '../../editor/SubGridEditor.svelte'
	import type { AppViewerContext, ComponentCustomCSS, RichConfigurations } from '../../types'
	import { initCss } from '../../utils'
	import InitializeComponent from '../helpers/InitializeComponent.svelte'
	import { InputValue } from '../helpers'
	import { twMerge } from 'tailwind-merge'
	import ResolveStyle from '../helpers/ResolveStyle.svelte'
	import { components, type DecisionTreeNode } from '../../editor/component'
	import Button from '$lib/components/common/button/Button.svelte'
	import { ArrowLeft, ArrowRight, RotateCcw } from 'lucide-svelte'
	import ResolveConfig from '../helpers/ResolveConfig.svelte'
	import { initConfig } from '../../editor/appUtils'

	export let id: string
	export let componentContainerHeight: number
	export let customCss: ComponentCustomCSS<'decisiontreecomponent'> | undefined = undefined
	export let render: boolean
	export let nodes: DecisionTreeNode[]
	export let configuration: RichConfigurations
	export let extraKey: string | undefined = undefined

	const { app, focusedGrid, selectedComponent, connectingInput } =
		getContext<AppViewerContext>('AppViewerContext')

	function onFocus() {
		$focusedGrid = {
			parentComponentId: id,
			subGridIndex: selectedConditionIndex
		}
	}

	let css = initCss($app.css?.conditionalwrapper, customCss)

	$: resolvedConditions = nodes.reduce((acc, node) => {
		acc[node.id] = acc[node.id] || []
		return acc
	}, resolvedConditions || {})

	$: resolvedNext = nodes.reduce((acc, node) => {
		acc[node.id] = acc[node.id] || false
		return acc
	}, resolvedNext || {})

	$: if (!nodes.map((n) => n.id).includes(currentNodeId)) {
		currentNodeId = nodes[0].id
	}

	let selectedConditionIndex = 0

	let currentNodeId = nodes[0].id

	function next() {
		if (currentNodeId === lastNodeId) {
			ended = true
			return
		}

		const resolvedNodeConditions = resolvedConditions[currentNodeId]

		let found: boolean = false

		resolvedNodeConditions.forEach((condition, index) => {
			if (found) return

			const node = nodes.find((node) => node.id == currentNodeId)

			if (condition && node && resolvedNext[node.id] !== false) {
				found = true
				currentNodeId = node.next[index].id

				$focusedGrid = {
					parentComponentId: id,
					subGridIndex: nodes.findIndex((node) => node.id == currentNodeId)
				}
			}
		})
	}

	function prev() {
		const previousNode = nodes.find((node) => {
			return node.next.find((next) => next.id == currentNodeId)
		})

		if (previousNode) {
			currentNodeId = previousNode.id
		}
	}

	$: lastNodeId = nodes?.find((node) => node.next.length === 0)?.id

	$: isNextDisabled =
		(resolvedConditions?.[currentNodeId].length > 1 &&
			resolvedConditions?.[currentNodeId]?.every((condition) => !condition)) ||
		resolvedNext?.[currentNodeId] === false

	let ended = false

	let resolvedConfig = initConfig(
		components['decisiontreecomponent'].initialData.configuration,
		configuration
	)
</script>

{#if Object.keys(resolvedConditions).length === nodes.length}
	{#each nodes ?? [] as node (node.id)}
		{#each node.next ?? [] as next, conditionIndex}
			{#if next.condition}
				<InputValue
					key="conditions"
					{id}
					input={next.condition}
					bind:value={resolvedConditions[node.id][conditionIndex]}
				/>
			{/if}
		{/each}
	{/each}
{/if}

{#each Object.keys(components['decisiontreecomponent'].initialData.configuration) as key (key)}
	<ResolveConfig
		{id}
		{extraKey}
		{key}
		bind:resolvedConfig={resolvedConfig[key]}
		configuration={configuration[key]}
	/>
{/each}

{#if Object.keys(resolvedConditions).length === nodes.length}
	{#each nodes ?? [] as node (node.id)}
		{#if node.allowed}
			<InputValue key="conditions" {id} input={node.allowed} bind:value={resolvedNext[node.id]} />
		{/if}
	{/each}
{/if}

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

{#if ended}
	<div class="flex flex-col gap-2 w-full items-center h-full justify-center">
		<div class="flex flex-col gap-2 items-center">
			<h1 class="text-2xl font-bold">{resolvedConfig.endtitle}</h1>
			<p class="text-sm text-center">
				{resolvedConfig.endDescription}
			</p>
		</div>
		<Button
			on:click={() => {
				currentNodeId = nodes[0].id
				ended = false
			}}
			size="sm"
			color="light"
			startIcon={{ icon: RotateCcw }}
		>
			Restart
		</Button>
	</div>
{:else}
	<div class="w-full overflow-auto">
		<div class="w-full">
			{#if $app.subgrids}
				{#each Object.values(nodes) ?? [] as node, i}
					<SubGridEditor
						visible={render && node.id === currentNodeId}
						{id}
						class={twMerge(css?.container?.class, 'wm-conditional-tabs')}
						style={css?.container?.style}
						subGridId={`${id}-${i}`}
						containerHeight={componentContainerHeight - 40}
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

		<div class="h-8 flex flex-row gap-2 justify-end items-center px-2">
			{#if nodes[0].id !== currentNodeId}
				<Button on:click={prev} size="xs2" color="light" startIcon={{ icon: ArrowLeft }}
					>Prev</Button
				>
			{/if}
			<span class="text-xs text-primary">Tab: {currentNodeId}</span>
			<Button
				on:click={next}
				size="xs2"
				color="dark"
				endIcon={{ icon: ArrowRight }}
				disabled={isNextDisabled}
			>
				{currentNodeId === lastNodeId ? 'Finish' : 'Next'}
			</Button>
		</div>
	</div>
{/if}
