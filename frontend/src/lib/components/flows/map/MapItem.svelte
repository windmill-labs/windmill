<script lang="ts">
	import type { FlowEditorContext } from '../types'
	import { getContext } from 'svelte'
	import FlowModuleSchemaItem from './FlowModuleSchemaItem.svelte'
	import Icon from 'svelte-awesome'
	import { faPlus } from '@fortawesome/free-solid-svg-icons'
	import { emptyModule, loadFlowModuleState } from '$lib/components/flows/flowStateUtils'
	import { flowStateStore } from '../flowState'
	import type { FlowModule } from '$lib/gen'
	import { emptyFlowModuleState, isEmptyFlowModule } from '../utils'
	import { flowModuleMap } from '../flowModuleMap'
	import FlowModuleSchemaMap from './FlowModuleSchemaMap.svelte'
	import { flowStore } from '../flowStore'

	// Props
	export let partialPath: number[] = []
	export let mod: FlowModule
	export let index: number

	// Reactive statements
	$: {
		// WARNING: THIS STATEMENT BINDS THE FLOWMODULE TO THE FLOW MODULE MAP
		$flowModuleMap[mod.id] = { flowModule: mod }
	}

	// Local variable
	const { select, selectedId } = getContext<FlowEditorContext>('FlowEditorContext')
	let idToRemove: string | undefined = undefined

	// Components methodsß
	function insertAtIndex(index: number): void {
		const flowModule = emptyModule()

		// Insert at the right place
		modules.splice(index, 0, flowModule)
		modules = modules

		flowStateStore.update((fss) => {
			fss[flowModule.id] = emptyFlowModuleState()
			return fss
		})

		select(flowModule.id)
	}

	function removeById(id: string): void {
		select('settings')

		/*

		modules.splice(index, 1)
		moduleStates.splice(index, 1)
		moduleStates = moduleStates
		modules = modules
		*/
	}
</script>

{#if mod}
	<button
		on:click={() => insertAtIndex(index)}
		type="button"
		class="text-gray-900 m-0.5 my-0.5 bg-white border border-gray-300 focus:outline-none hover:bg-gray-100 focus:ring-4 focus:ring-gray-200 font-medium rounded-full text-sm w-6 h-6 flex items-center justify-center"
	>
		<Icon data={faPlus} scale={0.8} />
	</button>
	{#if mod.value.type === 'forloopflow'}
		<li>
			<FlowModuleSchemaItem
				deletable
				on:delete={() => removeById(mod.id)}
				on:click={() => select(['loop', String(index)].join('-'))}
				selected={$selectedId === ['loop', String(index)].join('-')}
				retry={mod.retry?.constant != undefined || mod.retry?.exponential != undefined}
				earlyStop={mod.stop_after_if != undefined}
				suspend={Boolean(mod.suspend)}
			>
				<div slot="icon">
					<span>{index + 1}</span>
				</div>
				<div slot="content" class="truncate block w-full">
					<span>{mod.summary || 'For loop'}</span>
				</div>
			</FlowModuleSchemaItem>

			<div class="flex text-xs">
				<div class="line mr-2" />

				<div class="w-full my-2">
					<FlowModuleSchemaMap partialPath={[...partialPath, index]} modules={mod.value.modules} />
				</div>
			</div>
		</li>
	{:else}
		<li>
			<FlowModuleSchemaItem
				on:click={() => select(mod.id)}
				color={partialPath.length === 0 ? 'blue' : 'orange'}
				selected={$selectedId === mod.id}
				deletable
				on:delete={(event) => {
					if (event.detail.event.shiftKey || isEmptyFlowModule(mod)) {
						removeById(mod.id)
					} else {
						idToRemove = mod.id
					}
				}}
				retry={mod.retry?.constant != undefined || mod.retry?.exponential != undefined}
				earlyStop={mod.stop_after_if != undefined}
				suspend={Boolean(mod.suspend)}
			>
				<div slot="icon">
					<span>{index + 1}</span>
				</div>
				<div slot="content" class="w-full truncate block text-xs">
					<span>
						{mod.summary ||
							(`path` in mod.value ? mod.value.path : undefined) ||
							(mod.value.type === 'rawscript' ? `Inline ${mod.value.language}` : 'Select a script')}
					</span>
				</div>
			</FlowModuleSchemaItem>
		</li>
	{/if}
{/if}
