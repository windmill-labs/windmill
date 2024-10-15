<script lang="ts">
	import NodeWrapper from './NodeWrapper.svelte'
	import FlowModuleSchemaItem from '../../../flows/map/FlowModuleSchemaItem.svelte'
	import { Repeat } from 'lucide-svelte'
	import { getContext, createEventDispatcher } from 'svelte'
	import type { Writable } from 'svelte/store'
	import type { GraphEventHandlers } from '../../graphBuilder'
	import { twMerge } from 'tailwind-merge'
	import { NODE } from '../../util'
	import InsertModuleButton from '../../../flows/map/InsertModuleButton.svelte'

	const dispatch = createEventDispatcher()

	export let data: {
		bgColor: string
		eventHandlers: GraphEventHandlers
	}

	const { selectedId } = getContext<{ selectedId: Writable<string> }>('FlowGraphContext')
	$: itemProps = {
		selected: $selectedId === 'stepid'
	}

	/* async function deleteSchedulePoll(e) {
		data.eventHandlers.delete(e.detail, '')
	} */
</script>

<NodeWrapper>
	<button
		class={twMerge(
			'flex flex-col gap-2  rounded-sm p-1 z-10 border',
			$selectedId === 'schedulePoll'
				? 'outline outline-offset-0  outline-2  outline-slate-500 dark:outline-gray-400'
				: ''
		)}
		style="width: {NODE.width}px; min-height: {NODE.height}px;"
		on:click={() => data.eventHandlers.select('schedulePoll')}
	>
		<div
			class="text-2xs text-secondary font-normal text-center rounded-sm w-[80%] border bg-surface"
		>
			<div class="flex flex-row gap-2 p-2">
				<div>
					<Repeat size={16} />
				</div>

				<div class="flex grow justify-center items-center">
					<span class="text-center">Schedule Poll</span>
				</div>
			</div>
		</div>
		<div
			class="text-2xs text-secondary font-normal text-center rounded-sm w-[80%] ml-auto border bg-surface"
		>
			<FlowModuleSchemaItem
				deletable={true}
				label={`Step`}
				id="step id"
				on:changeId
				on:move={() => dispatch('move')}
				on:delete
				on:click={() => ($selectedId = 'stepid')}
				{...itemProps}
				bgColor={'#ffffff'}
			/>
		</div>

		<div class="text-2xs text-secondary font-normal rounded-sm w-[80%] ml-auto">
			<div class="flex justify-center items-center w-full">
				<InsertModuleButton />
			</div>
		</div>
	</button>
</NodeWrapper>
