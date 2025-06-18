<script lang="ts">
	import { createBubbler, stopPropagation } from 'svelte/legacy'

	const bubble = createBubbler()
	import Dropdown from '$lib/components/DropdownV2.svelte'
	import { classNames } from '$lib/utils'
	import { createEventDispatcher, getContext } from 'svelte'
	import type { AppViewerContext } from '../types'
	import type { DecisionTreeNode } from './component'
	import { isDebugging } from './settingsPanel/decisionTree/utils'
	import { X, Bug } from 'lucide-svelte'

	interface Props {
		nodes?: DecisionTreeNode[]
		id: string
		isSmall?: boolean
		componentIsDebugging?: boolean
	}

	let { nodes = [], id, isSmall = false, componentIsDebugging = $bindable(false) }: Props = $props()

	$effect(() => {
		componentIsDebugging = isDebugging($debuggingComponents, id)
	})

	const { componentControl, debuggingComponents, worldStore } =
		getContext<AppViewerContext>('AppViewerContext')
	const dispatch = createEventDispatcher()

	let currentNodeId: string = $state($worldStore.outputsById[id]?.currentNodeId?.peak() ?? 'a')

	function subscribeToCurrentNode(id: string) {
		return $worldStore.outputsById[id]?.currentNodeId?.subscribe(
			{
				id: `id-${id}-${currentNodeId}`,
				next: (value) => {
					currentNodeId = value
				}
			},
			currentNodeId
		)
	}

	let subscription = subscribeToCurrentNode(id)

	function onDebugNode(debuggedNodeIndex: number | undefined) {
		if (debuggedNodeIndex === undefined) {
			return
		}

		if (debuggedNodeIndex !== undefined && nodes[debuggedNodeIndex]?.id === undefined) {
			currentNodeId = nodes[0]?.id ?? ''
			$componentControl?.[id]?.setTab?.(0)

			$debuggingComponents = Object.fromEntries(
				Object.entries($debuggingComponents).filter(([key]) => key !== id)
			)
		}
	}

	$effect(() => {
		onDebugNode($debuggingComponents[id])
	})

	let renderCount: number = $state(0)
	let lastNodes: DecisionTreeNode[] = nodes

	function onNodesChange(newNodes: DecisionTreeNode[]) {
		if (JSON.stringify(newNodes) !== JSON.stringify(lastNodes)) {
			lastNodes = newNodes

			if (subscription) {
				subscription?.()
			}
			subscription = subscribeToCurrentNode(id)

			renderCount++
		}
	}

	$effect(() => {
		onNodesChange(nodes)
	})

	async function getDropdownItems() {
		return [
			// Debug node items
			...(nodes ?? []).map((node, index) => ({
				displayName: `Debug node ${node.label}`,
				action: () => {
					$componentControl?.[id]?.setTab?.(index)
					$debuggingComponents[id] = index
				},
				type: 'action' as const
			})),
			// Reset debug mode item
			{
				displayName: 'Reset debug mode',
				action: () => {
					$componentControl?.[id]?.setTab?.(0)
					$debuggingComponents = Object.fromEntries(
						Object.entries($debuggingComponents).filter(([key]) => key !== id)
					)
				},
				type: 'delete' as const
			}
		]
	}
</script>

{#key renderCount}
	<Dropdown items={getDropdownItems} class="w-fit h-auto" usePointerDownOutside={true}>
		{#snippet buttonReplacement()}
			<button
				title={'Debug tabs'}
				class={classNames(
					'px-1 text-2xs font-bold rounded cursor-pointer w-fit h-full',
					componentIsDebugging
						? ' hover:bg-red-300 hover:text-red-800'
						: 'text-blue-600 hover:bg-blue-300 hover:text-blue-800'
				)}
				onclick={() => dispatch('triggerInlineEditor')}
				onpointerdown={stopPropagation(bubble('pointerdown'))}
			>
				{#if componentIsDebugging}
					<div class="flex flex-row items-center gap-2">
						{`${isSmall ? '' : 'Debugging node'} ${nodes[$debuggingComponents[id] ?? 0]?.id}`}
						<!-- svelte-ignore node_invalid_placement_ssr -->
						<button
							onclick={() => {
								$componentControl?.[id]?.setTab?.(0)

								$debuggingComponents = Object.fromEntries(
									Object.entries($debuggingComponents).filter(([key]) => key !== id)
								)
							}}
						>
							<X size={11} />
						</button>
					</div>
				{:else if isSmall}
					<div class="flex h-full w-fit items-center"><Bug size={11} /></div>
				{:else}<div class="whitespace-nowrap h-full"
						>{`Debug nodes (current node: ${currentNodeId})`}</div
					>{/if}
			</button>
		{/snippet}
	</Dropdown>
{/key}
