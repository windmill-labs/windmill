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
	import { emptyFlowModuleState, isEmptyFlowModule } from '../utils'
	import MapItem from './MapItem.svelte'
	import FlowSettingsItem from './FlowSettingsItem.svelte'
	import FlowInputsItem from './FlowInputsItem.svelte'
	import InsertModuleButton from './InsertModuleButton.svelte'
	import { slide } from 'svelte/transition'

	export let root: boolean = false
	export let modules: FlowModule[]

	let indexToRemove: number | undefined = undefined
	const { select } = getContext<FlowEditorContext>('FlowEditorContext')

	async function insertNewModuleAtIndex(
		index: number,
		kind: 'script' | 'forloop' | 'branchone' | 'branchall'
	): Promise<void> {
		await idMutex.runExclusive(async () => {
			var module = emptyModule()
			var state = emptyFlowModuleState()
			if (kind == 'forloop') {
				;[module, state] = await createLoop(module.id)
			} else if (kind == 'branchone') {
				;[module, state] = await createBranches(module.id)
			} else if (kind == 'branchall') {
				;[module, state] = await createBranchAll(module.id)
			}
			const flowModule = module
			modules.splice(index, 0, flowModule)
			modules = modules
			$flowStateStore[flowModule.id] = state
			select(flowModule.id)
		})
	}

	function removeAtIndex(index: number): void {
		select('settings')
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

<div class="flex flex-col h-full">
	{#if root}
		<div class="flex-initial px-3 py-2 border-b">
			<FlowSettingsItem />
		</div>
	{/if}
	<ul
		class="w-full flex-auto relative  {root
			? ' overflow-y-auto overflow-x-hidden px-2 my-2'
			: ''} py-1"
	>
		{#if root}
			<li>
				<FlowInputsItem />
			</li>
		{/if}

		{#each modules as mod, index (mod.id ?? index)}
			<div transition:slide|local>
				<MapItem
					bind:mod
					on:delete={(event) => {
						if (event.detail.detail.shiftKey || isEmptyFlowModule(mod)) {
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

		<InsertModuleButton on:new={(e) => insertNewModuleAtIndex(modules.length, e.detail)} />
	</ul>
	{#if root}
		<div class="flex-none px-4 py-2 border-t">
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
	}}
/>
