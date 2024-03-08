<script lang="ts">
	import ButtonDropdown from '$lib/components/common/button/ButtonDropdown.svelte'
	import { classNames } from '$lib/utils'
	import { MenuItem } from '@rgossiaux/svelte-headlessui'
	import { createEventDispatcher, getContext } from 'svelte'
	import type { AppViewerContext } from '../types'
	import type { DecisionTreeNode } from './component'
	import { isDebugging } from './settingsPanel/decisionTree/utils'
	import { X } from 'lucide-svelte'

	export let nodes: DecisionTreeNode[] = []
	export let id: string

	const { componentControl, debuggingComponents, worldStore } =
		getContext<AppViewerContext>('AppViewerContext')
	const dispatch = createEventDispatcher()

	let currentNodeId: string = $worldStore.outputsById[id]?.currentNodeId?.peak() ?? 'a'

	$worldStore.outputsById[id]?.currentNodeId?.subscribe(
		{
			id: id,
			next: (value) => {
				currentNodeId = value
			}
		},
		currentNodeId
	)

	$: if (nodes[$debuggingComponents[id] ?? 0]?.id === undefined) {
		currentNodeId = ''
		$componentControl?.[id]?.setTab?.(0)

		$debuggingComponents = Object.fromEntries(
			Object.entries($debuggingComponents).filter(([key]) => key !== id)
		)
	}
</script>

<button
	title={'Debug tabs'}
	class={classNames(
		'text-2xs py-0.5 font-bold w-fit border cursor-pointer rounded-sm',
		isDebugging($debuggingComponents, id)
			? 'bg-red-100 text-red-600 border-red-500 hover:bg-red-200 hover:text-red-800'
			: 'bg-indigo-100 text-indigo-600 border-indigo-500 hover:bg-indigo-200 hover:text-indigo-800'
	)}
	on:click={() => dispatch('triggerInlineEditor')}
	on:pointerdown|stopPropagation
>
	<ButtonDropdown hasPadding={false}>
		<svelte:fragment slot="buttonReplacement">
			<div class="px-1">
				{#if isDebugging($debuggingComponents, id)}
					<div class="flex flex-row items-center gap-2">
						{`Debugging node ${nodes[$debuggingComponents[id] ?? 0]?.id}`}
						<button
							on:click={() => {
								$componentControl?.[id]?.setTab?.(0)

								$debuggingComponents = Object.fromEntries(
									Object.entries($debuggingComponents).filter(([key]) => key !== id)
								)
							}}
						>
							<X size={14} />
						</button>
					</div>
				{:else}
					{`Debug nodes (current node: ${currentNodeId})`}
				{/if}
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
