<script lang="ts">
	import { preventDefault, stopPropagation } from 'svelte/legacy'

	import type { DecisionTreeNode } from '../component'
	import { twMerge } from 'tailwind-merge'
	import InsertDecisionTreeNode from './decisionTree/InsertDecisionTreeNode.svelte'
	import { X } from 'lucide-svelte'
	import NodeWrapper from '$lib/components/graph/renderers/nodes/NodeWrapper.svelte'

	interface Props {
		data: {
			node: DecisionTreeNode
			canDelete: boolean
			nodeCallbackHandler: (
				event: string,
				detail: string,
				graphNode: DecisionTreeNode | undefined,
				parentIds: string[],
				branchInsert: boolean
			) => void
			parentIds: string[]
			branchHeader: boolean
		}
	}

	let { data }: Props = $props()

	let open: boolean = false
</script>

<NodeWrapper>
	<div class="relative h-full">
		<div
			class={twMerge(
				'w-full h-full border border-gray-400',
				'flex flex-row gap-2 items-center justify-between rounded-sm overflow-hidden'
			)}
			style="width: 275px; height: 34px; background-color: {document.documentElement.classList.contains(
				'dark'
			)
				? '#2e3440'
				: '#dfe6ee'}"
		>
			<div class="ml-4 text-xs font-normal text-primary"> {data.node.label} </div>
		</div>

		{#if data.canDelete}
			<div class="w-[27px] absolute -top-[32px] left-[50%] right-[50%] -translate-x-1/2">
				<button
					title="Delete branch"
					onclick={stopPropagation(
						preventDefault(() => {
							data.nodeCallbackHandler(
								'removeBranch',
								data.node.id,
								data.node,
								data.parentIds,
								false
							)
						})
					)}
					type="button"
					class="text-primary bg-surface border mx-[1px] border-gray-300 dark:border-gray-500 focus:outline-none hover:bg-surface-hover focus:ring-4 focus:ring-gray-200 font-medium rounded-full text-sm w-[25px] h-[25px] flex items-center justify-center"
				>
					<X class="m-[5px]" size={15} />
				</button>
			</div>
		{/if}

		{#if data.node.id !== 'end'}
			<div
				class={twMerge(
					'absolute -bottom-10 left-1/2 transform -translate-x-1/2 flex items-center',
					open ? 'z-20' : ''
				)}
			>
				<InsertDecisionTreeNode
					on:node={() => {
						data.nodeCallbackHandler(
							'nodeInsert',
							data.node.id,
							data.node,
							data.parentIds ?? [],
							data.branchHeader
						)
					}}
					on:addBranch={() => {
						data.nodeCallbackHandler('addBranch', data.node.id, data.node, data.parentIds, true)
					}}
					canAddBranch={false}
				/>
			</div>
		{/if}
	</div>
</NodeWrapper>
