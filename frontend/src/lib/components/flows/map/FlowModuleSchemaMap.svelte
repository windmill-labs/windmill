<script lang="ts">
	import type { FlowEditorContext } from '../types'
	import { getContext } from 'svelte'
	import {
		createBranchAll,
		createBranches,
		createLoop,
		deleteFlowStateById,
		emptyModule,
		idMutex
	} from '$lib/components/flows/flowStateUtils'
	import { flowStateStore, type FlowModuleState } from '../flowState'
	import type { FlowModule } from '$lib/gen'
	import FlowErrorHandlerItem from './FlowErrorHandlerItem.svelte'
	import RemoveStepConfirmationModal from '../content/RemoveStepConfirmationModal.svelte'
	import { emptyFlowModuleState } from '../utils'
	import MapItem from './MapItem.svelte'
	import FlowSettingsItem from './FlowSettingsItem.svelte'
	import FlowInputsItem from './FlowInputsItem.svelte'
	import InsertModuleButton from './InsertModuleButton.svelte'
	import { slide } from 'svelte/transition'

	export let root: boolean = false
	export let modules: FlowModule[] | undefined

	let indexToRemove: number | undefined = undefined
	const { select, selectedId } = getContext<FlowEditorContext>('FlowEditorContext')

	async function insertNewModuleAtIndex(
		index: number,
		kind: 'script' | 'forloop' | 'branchone' | 'branchall' | 'flow' | 'trigger' | 'approval'
	): Promise<void> {
		await idMutex.runExclusive(async () => {
			var module = emptyModule(kind == 'flow')
			var state = emptyFlowModuleState()
			if (kind == 'forloop') {
				;[module, state] = await createLoop(module.id)
			} else if (kind == 'branchone') {
				;[module, state] = await createBranches(module.id)
			} else if (kind == 'branchall') {
				;[module, state] = await createBranchAll(module.id)
			}
			const flowModule = module
			if (!modules) return
			modules.splice(index, 0, flowModule)
			modules = modules
			$flowStateStore[flowModule.id] = state
			if (kind == 'trigger') {
				flowModule.summary = 'Trigger'
			} else if (kind == 'approval') {
				flowModule.summary = 'Approval'
			}
			select(flowModule.id)
		})
	}

	function removeAtIndex(index: number): void {
		select('settings-graph')
		if (!modules) return
		const [removedModule] = modules.splice(index, 1)
		modules = modules

		const leaves = findLeaves(removedModule)

		leaves.forEach((leafId: string) => deleteFlowStateById(leafId))
	}

	function findLeaves(flowModule: FlowModule): string[] {
		const id = flowModule.id

		if (flowModule.value.type === 'forloopflow') {
			return [id, ...flowModule.value.modules.map((fm) => findLeaves(fm)).flat()]
		}

		if (flowModule.value.type === 'branchall') {
			return [
				id,
				...flowModule.value.branches
					.map((branch) => branch.modules.map((mod) => findLeaves(mod)).flat())
					.flat()
			]
		}

		if (flowModule.value.type === 'branchone') {
			return [
				id,
				...flowModule.value.branches
					.map((branch) => {
						return branch.modules.map((mod) => findLeaves(mod)).flat()
					})
					.flat(),
				...flowModule.value.default.map((mod) => findLeaves(mod)).flat()
			]
		}

		return [id]
	}

	$: confirmationModalOpen = indexToRemove !== undefined
</script>

<div class="flex flex-col h-full relative">
	{#if root}
		<div
			class="z-10 sticky top-0 bg-gray-50 flex-initial inline-flex px-3 py-2 items-center h-full max-h-12 border-b border-gray-300"
		>
			<FlowSettingsItem />
		</div>
	{/if}
	<ul class="w-full flex-auto   {root ? 'px-2 pb-2 pt-3' : ''} py-1">
		{#if root}
			<li>
				<FlowInputsItem />
			</li>
		{/if}

		{#if modules}
			{#each modules as mod, index (mod.id ?? index)}
				<div transition:slide|local>
					<MapItem
						{index}
						bind:mod
						on:delete={(event) => {
							if (event.detail.detail.shiftKey || mod.value.type === 'identity') {
								removeAtIndex(index)
							} else {
								indexToRemove = index
							}
						}}
						on:insert={(e) => {
							insertNewModuleAtIndex(index, e.detail)
						}}
					/>
				</div>
			{/each}
		{/if}
		<InsertModuleButton
			trigger={modules?.length == 0}
			on:new={(e) => insertNewModuleAtIndex(modules?.length ?? 0, e.detail)}
		/>
	</ul>
	{#if root}
		<div class="sticky bottom-0 bg-gray-50 flex-none px-4 py-1 pb-2 border-t">
			<FlowErrorHandlerItem />
		</div>
	{/if}
</div>

<RemoveStepConfirmationModal
	bind:open={confirmationModalOpen}
	on:canceled={() => {
		indexToRemove = undefined
	}}
	on:confirmed={() => {
		if (indexToRemove !== undefined) {
			removeAtIndex(indexToRemove)
			indexToRemove = undefined
		}
		$selectedId = 'settings-graph'
	}}
/>
