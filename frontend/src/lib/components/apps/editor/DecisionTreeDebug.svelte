<script lang="ts">
	import ButtonDropdown from '$lib/components/common/button/ButtonDropdown.svelte'
	import { classNames } from '$lib/utils'
	import { MenuItem } from '@rgossiaux/svelte-headlessui'
	import { createEventDispatcher, getContext } from 'svelte'
	import type { AppViewerContext } from '../types'
	import type { DecisionTreeNode } from './component'
	import { isDebugging } from './settingsPanel/decisionTree/utils'
	import { X, Bug } from 'lucide-svelte'

	export let nodes: DecisionTreeNode[] = []
	export let id: string
	export let isSmall = false
	export let componentIsDebugging = false

	$: componentIsDebugging = isDebugging($debuggingComponents, id)

	const { componentControl, debuggingComponents, worldStore } =
		getContext<AppViewerContext>('AppViewerContext')
	const dispatch = createEventDispatcher()

	let currentNodeId: string = $worldStore.outputsById[id]?.currentNodeId?.peak() ?? 'a'

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

	$: onDebugNode($debuggingComponents[id])

	let renderCount: number = 0
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

	$: onNodesChange(nodes)
</script>

{#key renderCount}
	<button
		title={'Debug tabs'}
		class={classNames(
<<<<<<< HEAD
			'px-1 text-2xs font-bold rounded cursor-pointer w-fit h-full',
			componentIsDebugging
				? ' hover:bg-red-300 hover:text-red-800'
				: ' hover:bg-indigo-300 hover:text-indigo-800'
=======
			'px-1 text-2xs font-bold rounded cursor-pointer w-fit h-ful',
			isDebugging($debuggingComponents, id)
				? 'bg-red-100 text-red-600 border-red-500 hover:bg-red-200 hover:text-red-800'
				: 'text-indigo-600 hover:bg-indigo-300 hover:text-indigo-800'
>>>>>>> a8e880e12f7b32e814c58b1de87597194a6a71eb
		)}
		on:click={() => dispatch('triggerInlineEditor')}
		on:pointerdown|stopPropagation
	>
		<ButtonDropdown hasPadding={false}>
			<svelte:fragment slot="buttonReplacement">
<<<<<<< HEAD
				<div class="px-1 w-fit">
					{#if componentIsDebugging}
=======
				<div class="px-1 h-full">
					{#if isDebugging($debuggingComponents, id)}
>>>>>>> a8e880e12f7b32e814c58b1de87597194a6a71eb
						<div class="flex flex-row items-center gap-2">
							{`${isSmall ? '' : 'Debugging node'} ${nodes[$debuggingComponents[id] ?? 0]?.id}`}
							<button
								on:click={() => {
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
				</div>
			</svelte:fragment>
			<svelte:fragment slot="items">
				{#each nodes ?? [] as node, index}
					<MenuItem
						on:click={() => {
							$componentControl?.[id]?.setTab?.(index)

							$debuggingComponents[id] = index
						}}
					>
						<div
							class={classNames(
								'!text-tertiary text-left px-4 py-2 gap-2 cursor-pointer hover:bg-gray-100 !text-xs font-semibold'
							)}
						>
							{`Debug node ${node.label}`}
						</div>
					</MenuItem>
				{/each}
				<MenuItem
					on:click={() => {
						$componentControl?.[id]?.setTab?.(0)

						$debuggingComponents = Object.fromEntries(
							Object.entries($debuggingComponents).filter(([key]) => key !== id)
						)
					}}
				>
					<div
						class={classNames(
							'!text-red-600 text-left px-4 py-2 gap-2 cursor-pointer hover:bg-gray-100 !text-xs font-semibold'
						)}
					>
						{`Reset debug mode`}
					</div>
				</MenuItem>
			</svelte:fragment>
		</ButtonDropdown>
	</button>
{/key}
