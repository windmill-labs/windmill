<script lang="ts">
	import type { FlowEditorContext } from '../types'
	import { getContext } from 'svelte'
	import { emptyModule } from '$lib/components/flows/flowStateUtils'
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

	function insertNewModuleAtIndex(index: number): void {
		const flowModule = emptyModule()
		modules.splice(index, 0, flowModule)
		modules = modules
		$flowStateStore[flowModule.id] = emptyFlowModuleState()

		// TODO: Should find a way to select the newly inserted
	}

	function removeAtIndex(index: number): void {
		select('settings')
		const [removedModule] = modules.splice(index, 1)
		modules = modules

		flowStateStore.update((fss) => {
			delete fss[removedModule.id]
			return fss
		})
	}

	$: confirmationModalOpen = indexToRemove !== undefined
</script>

<div class="flex flex-col justify-between">
	<ul class="w-full">
		{#if root}
			<FlowSettingsItem />
			<FlowInputsItem />
		{/if}

		{#each modules as mod, index (index)}
			<MapItem
				{color}
				bind:mod
				{index}
				on:delete={(event) => {
					if (event.detail.event.shiftKey || isEmptyFlowModule(mod)) {
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
		{#if root}
			<FlowErrorHandlerItem />
		{/if}
	</ul>
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

	.line {
		background: repeating-linear-gradient(to bottom, transparent 0 4px, #bbb 4px 8px) 50%/1px 100%
			no-repeat;
		width: 2rem;
	}
</style>
