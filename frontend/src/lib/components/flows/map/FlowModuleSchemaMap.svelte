<script lang="ts">
	import type { FlowEditorContext } from '../types'
	import { getContext } from 'svelte'
	import FlowModuleSchemaItem from './FlowModuleSchemaItem.svelte'
	import Icon from 'svelte-awesome'
	import {
		faCalendarAlt,
		faCodeBranch,
		faPen,
		faPlus,
		faSliders,
		faTrashAlt
	} from '@fortawesome/free-solid-svg-icons'
	import { emptyModule } from '$lib/components/flows/flowStateUtils'
	import { classNames, emptySchema } from '$lib/utils'

	import { flowStateStore, type FlowModuleState } from '../flowState'

	import type { FlowModule } from '$lib/gen'

	import FlowErrorHandlerItem from './FlowErrorHandlerItem.svelte'
	import RemoveStepConfirmationModal from '../content/RemoveStepConfirmationModal.svelte'

	import { emptyFlowModuleState, isEmptyFlowModule } from '../utils'
	import { flowModuleMap } from '../flowModuleMap'
	import MapItem from './MapItem.svelte'

	export let partialPath: number[] = []
	export let modules: FlowModule[]

	const { select, selectedId, schedule } = getContext<FlowEditorContext>('FlowEditorContext')

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

	let idToRemove: string | undefined = undefined
	$: confirmationModalOpen = idToRemove !== undefined

	$: settingsClass = classNames(
		'border w-full rounded-md p-2 bg-white text-sm cursor-pointer flex items-center mb-4',
		$selectedId.includes('settings') ? 'outline outline-offset-1 outline-2  outline-slate-900' : ''
	)
</script>

<div class="flex flex-col justify-between">
	<ul class="w-full">
		{#if partialPath.length === 0}
			<div on:click={() => select('settings')} class={settingsClass}>
				<Icon data={faSliders} class="mr-2" />
				<span
					class="text-xs font-bold flex flex-row justify-between w-full flex-wrap gap-2 items-center"
				>
					Settings
					<span
						class={classNames('badge', $schedule?.enabled ? 'badge-on' : 'badge-off')}
						on:click|stopPropagation={() => select('settings-schedule')}
					>
						{$schedule.cron}
						<Icon class={$schedule.cron ? 'ml-2' : ''} data={faCalendarAlt} scale={0.8} />
					</span>
				</span>
			</div>

			<FlowModuleSchemaItem
				on:click={() => select('inputs')}
				isFirst
				hasLine
				selected={$selectedId === 'inputs'}
			>
				<div slot="icon">
					<Icon data={faPen} scale={0.8} />
				</div>
				<div slot="content" class="flex flex-row text-xs ">Inputs</div>
			</FlowModuleSchemaItem>
		{/if}

		{#each modules as mod, index (index)}
			<MapItem bind:mod {index} {partialPath} />
		{/each}

		<button
			on:click={() => insertAtIndex(modules.length)}
			type="button"
			class="text-gray-900 bg-white border m-0.5 border-gray-300 focus:outline-none hover:bg-gray-100 focus:ring-4 focus:ring-gray-200 font-medium rounded-full text-sm w-6 h-6 flex items-center justify-center"
		>
			<Icon data={faPlus} scale={0.8} />
		</button>
	</ul>
	{#if partialPath.length === 0}
		<FlowErrorHandlerItem />
	{/if}
</div>

<RemoveStepConfirmationModal
	bind:open={confirmationModalOpen}
	on:canceled={() => {
		idToRemove = undefined
	}}
	on:confirmed={() => {
		if (idToRemove !== undefined) {
			removeById(idToRemove)
			idToRemove = undefined
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
