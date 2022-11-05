<script lang="ts">
	import type { FlowEditorContext } from '../types'
	import { getContext } from 'svelte'
	import { deleteFlowStateById, emptyModule, idMutex } from '$lib/components/flows/flowStateUtils'
	import { flowStateStore } from '../flowState'
	import type { FlowModule } from '$lib/gen'
	import FlowErrorHandlerItem from './FlowErrorHandlerItem.svelte'
	import RemoveStepConfirmationModal from '../content/RemoveStepConfirmationModal.svelte'
	import { emptyFlowModuleState, isEmptyFlowModule } from '../utils'
	import MapItem from './MapItem.svelte'
	import FlowSettingsItem from './FlowSettingsItem.svelte'
	import FlowInputsItem from './FlowInputsItem.svelte'
	import InsertModuleButton from './InsertModuleButton.svelte'

	export let root: boolean = false
	export let color: 'blue' | 'orange' | 'indigo' = 'blue'
	export let modules: FlowModule[]

	let indexToRemove: number | undefined = undefined
	const { select } = getContext<FlowEditorContext>('FlowEditorContext')

	async function insertNewModuleAtIndex(index: number): Promise<void> {
		await idMutex.runExclusive(async () => {
			const flowModule = emptyModule()
			modules.splice(index, 0, flowModule)
			modules = modules
			$flowStateStore[flowModule.id] = emptyFlowModuleState()
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

<div class="flex flex-col justify-between w-full">
	<ul class="w-full">
		{#if root}
			<FlowSettingsItem />
			<FlowInputsItem />
		{/if}

		{#each modules as mod, index (mod.id ?? index)}
			<MapItem
				{color}
				{index}
				bind:mod
				on:delete={(event) => {
					if (event.detail.detail.shiftKey || isEmptyFlowModule(mod)) {
						removeAtIndex(index)
					} else {
						indexToRemove = index
					}
				}}
				on:insert={() => {
					insertNewModuleAtIndex(index)
				}}
			/>
		{/each}

		<InsertModuleButton on:click={() => insertNewModuleAtIndex(modules.length)} />
	</ul>
	{#if root}
		<FlowErrorHandlerItem />
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

<style>
	.badge {
		@apply whitespace-nowrap text-sm font-medium border px-2.5 py-0.5 rounded cursor-pointer flex items-center;
	}

	.badge-on {
		@apply bg-blue-100 text-blue-800 hover:bg-blue-200;
	}

	.badge-off {
		@apply bg-gray-100 text-gray-800 hover:bg-gray-200;
	}
</style>
