<script lang="ts">
	import { getContext, untrack } from 'svelte'
	import SubGridEditor from '../../editor/SubGridEditor.svelte'
	import type { AppViewerContext, ComponentCustomCSS } from '../../types'
	import { initCss } from '../../utils'
	import InitializeComponent from '../helpers/InitializeComponent.svelte'
	import { twMerge } from 'tailwind-merge'
	import ResolveStyle from '../helpers/ResolveStyle.svelte'
	import type { DecisionTreeNode } from '../../editor/component'
	import Button from '$lib/components/common/button/Button.svelte'
	import { ArrowLeft, ArrowRight } from 'lucide-svelte'
	import { initOutput } from '../../editor/appUtils'
	import Badge from '$lib/components/common/badge/Badge.svelte'
	import { getFirstNode, isDebugging } from '../../editor/settingsPanel/decisionTree/utils'
	import InputValue from '../helpers/InputValue.svelte'

	interface Props {
		id: string
		componentContainerHeight: number
		customCss?: ComponentCustomCSS<'decisiontreecomponent'> | undefined
		render: boolean
		nodes: DecisionTreeNode[]
	}

	let { id, componentContainerHeight, customCss = undefined, render, nodes }: Props = $props()

	let resolvedConditions = $state(
		nodes.reduce((acc, node) => {
			acc[node.id] = acc[node.id] || []
			return acc
		}, {})
	)

	let resolvedNext = $state(
		nodes.reduce((acc, node) => {
			acc[node.id] = acc[node.id] || false
			return acc
		}, {})
	)

	let everRender = $state(render)
	$effect.pre(() => {
		render && !everRender && (everRender = true)
	})

	const {
		app,
		focusedGrid,
		selectedComponent,
		connectingInput,
		componentControl,
		worldStore,
		debuggingComponents
	} = getContext<AppViewerContext>('AppViewerContext')

	let css = $state(initCss($app.css?.conditionalwrapper, customCss))
	let selectedConditionIndex = 0
	let currentNodeId = $state(getFirstNode(nodes)?.id ?? '')

	let outputs = initOutput($worldStore, id, {
		currentNodeId,
		currentNodeIndex: selectedConditionIndex
	})

	$effect.pre(() => {
		if (!nodes.map((n) => n.id).includes(currentNodeId)) {
			const firstNode = untrack(() => getFirstNode(nodes)?.id)

			if (firstNode) {
				currentNodeId = firstNode
			}
		}
	})

	let lastNodeId = $derived(nodes?.find((node) => node.next.length === 0)?.id)
	let isNextDisabled = $derived(resolvedNext?.[currentNodeId] === false)

	const history: string[] = []

	function updateCurrentNode(node, index) {
		currentNodeId = node.next[index].id
		history.push(node.id)
		selectedConditionIndex = index + 1
		$focusedGrid = {
			parentComponentId: id,
			subGridIndex: nodes.findIndex((node) => node.id == currentNodeId)
		}
	}

	function next() {
		const resolvedNodeConditions = resolvedConditions[currentNodeId]
		const node = nodes.find((node) => node.id == currentNodeId)

		if (!node) {
			return
		}

		for (let index = 0; index < resolvedNodeConditions.length; index++) {
			const condition = resolvedNodeConditions[index]
			if (condition && resolvedNext[node.id] !== false) {
				updateCurrentNode(node, index)
				return
			}
		}
	}
	function updateFocusedGrid(nodeId) {
		currentNodeId = nodeId
		selectedConditionIndex = nodes.findIndex((node) => node.id == currentNodeId)
		$focusedGrid = {
			parentComponentId: id,
			subGridIndex: selectedConditionIndex
		}
	}

	function prev() {
		const previousNodeId = history.pop()

		if (previousNodeId) {
			updateFocusedGrid(previousNodeId)
		} else {
			// if no history, go to the first node
			const node = getFirstNode(nodes)
			if (node) {
				updateFocusedGrid(node.id)
			}
		}
	}

	function onFocus(newIndex: number) {
		selectedConditionIndex = newIndex

		const nodeId = nodes[newIndex]?.id

		if (nodeId) {
			currentNodeId = nodeId
			$focusedGrid = {
				parentComponentId: id,
				subGridIndex: selectedConditionIndex
			}
		}
	}

	$componentControl[id] = {
		setTab: (conditionIndex: number) => {
			if (conditionIndex !== -1) {
				onFocus(conditionIndex)
			}
		}
	}

	$effect.pre(() => {
		if (currentNodeId) {
			outputs.currentNodeId.set(currentNodeId)
			outputs.currentNodeIndex.set(nodes.findIndex((next) => next.id == currentNodeId))
		}
	})

	$effect.pre(() => {
		if ($selectedComponent?.[0] === id && !$focusedGrid) {
			$focusedGrid = {
				parentComponentId: id,
				subGridIndex: nodes.findIndex((node) => node.id === currentNodeId)
			}
		}
	})
</script>

<!-- {JSON.stringify(resolvedConditions)}
{JSON.stringify(resolvedNext)} -->
{#if Object.keys(resolvedConditions).length === nodes.length}
	{#each nodes ?? [] as node (node.id)}
		{#each node.next ?? [] as next, conditionIndex}
			{#if next.condition}
				<InputValue
					key={`condition-${node.id}-${conditionIndex}`}
					{id}
					input={next.condition}
					bind:value={resolvedConditions[node.id][conditionIndex]}
					field={`condition-${node.id}-${conditionIndex}`}
				/>
			{/if}
		{/each}
	{/each}
{/if}

{#if Object.keys(resolvedConditions).length === nodes.length}
	{#each nodes ?? [] as node (node.id)}
		{#if node.allowed}
			<InputValue key="allowed" {id} input={node.allowed} bind:value={resolvedNext[node.id]} />
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

{#if everRender}
	<div class="w-full overflow-auto">
		<div class="w-full">
			{#if $app.subgrids}
				{#each Object.values(nodes) ?? [] as node, i}
					<SubGridEditor
						visible={render && node.id === currentNodeId}
						{id}
						class={twMerge(css?.container?.class, 'wm-decision-tree')}
						style={css?.container?.style}
						subGridId={`${id}-${i}`}
						containerHeight={componentContainerHeight - 40}
						on:focus={() => {
							if (!$connectingInput.opened) {
								$selectedComponent = [id]
							}
							onFocus(i)
						}}
					/>
				{/each}
			{/if}
		</div>
	</div>

	{#if render}
		<div class="h-8 flex flex-row gap-2 justify-end items-center px-2 bg-surface-primary z-50">
			{#if isDebugging($debuggingComponents, id)}
				<Badge color="red" size="xs2">
					{`Debugging. Actions are disabled.`}
				</Badge>
			{/if}
			{#if getFirstNode(nodes)?.id !== currentNodeId}
				<Button
					on:click={prev}
					size="xs2"
					color="light"
					startIcon={{ icon: ArrowLeft }}
					disabled={isDebugging($debuggingComponents, id)}
				>
					Prev
				</Button>
			{/if}
			<Button
				on:click={next}
				size="xs2"
				color="dark"
				endIcon={{ icon: ArrowRight }}
				disabled={isNextDisabled ||
					currentNodeId === lastNodeId ||
					isDebugging($debuggingComponents, id)}
			>
				Next
			</Button>
		</div>
	{/if}
{:else if $app.subgrids}
	{#each Object.values(nodes) ?? [] as _node, i}
		<SubGridEditor visible={false} {id} subGridId={`${id}-${i}`} />
	{/each}
{/if}
