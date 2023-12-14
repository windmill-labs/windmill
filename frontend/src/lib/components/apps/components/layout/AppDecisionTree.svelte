<script lang="ts">
	import { getContext } from 'svelte'
	import SubGridEditor from '../../editor/SubGridEditor.svelte'
	import type { AppViewerContext, ComponentCustomCSS } from '../../types'
	import { initCss } from '../../utils'
	import InitializeComponent from '../helpers/InitializeComponent.svelte'
	import { InputValue } from '../helpers'
	import { twMerge } from 'tailwind-merge'
	import ResolveStyle from '../helpers/ResolveStyle.svelte'
	import type { DecisionTreeNode } from '../../editor/component'
	import Button from '$lib/components/common/button/Button.svelte'
	import { ArrowLeft, ArrowRight } from 'lucide-svelte'

	export let id: string
	export let componentContainerHeight: number
	export let customCss: ComponentCustomCSS<'decisiontreecomponent'> | undefined = undefined
	export let render: boolean
	export let nodes: DecisionTreeNode[]

	const { app, focusedGrid, selectedComponent, connectingInput, componentControl } =
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
	$: isNextDisabled = resolvedNext?.[currentNodeId] === false

	$componentControl[id] = {
		setTab: (conditionIndex: number) => {
			if (conditionIndex === -1) {
				$focusedGrid = {
					parentComponentId: id,
					subGridIndex: 0
				}
			} else {
				currentNodeId = nodes[conditionIndex].id
				$focusedGrid = {
					parentComponentId: id,
					subGridIndex: conditionIndex
				}
			}
		}
	}
</script>

{#if Object.keys(resolvedConditions).length === nodes.length}
	{#each nodes ?? [] as node (node.id)}
		{#each node.next ?? [] as next, conditionIndex}
			{#if next.condition}
				<InputValue
					key={`condition-${node.id}-${conditionIndex}`}
					{id}
					input={next.condition}
					bind:value={resolvedConditions[node.id][conditionIndex]}
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
			<Button on:click={prev} size="xs2" color="light" startIcon={{ icon: ArrowLeft }}>Prev</Button>
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
