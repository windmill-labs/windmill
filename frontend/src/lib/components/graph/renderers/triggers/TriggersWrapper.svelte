<script lang="ts">
	import { Square, Maximize2, Minimize2 } from 'lucide-svelte'

	import { NODE } from '../../util'
	import Popover from '$lib/components/Popover.svelte'

	import { createEventDispatcher } from 'svelte'

	import TriggersBadge from './TriggersBadge.svelte'
	import InsertModuleButton from '../../../flows/map/InsertModuleButton.svelte'
	import type { FlowModule } from '$lib/gen'
	import type { GraphEventHandlers } from '../../graphBuilder'
	import { twMerge } from 'tailwind-merge'
	import MapItem from '../../../flows/map/MapItem.svelte'
	import { fade } from 'svelte/transition'
	import { getStateColor } from '../../util'

	export let path: string
	export let newItem: boolean
	export let isFlow: boolean
	export let selected: boolean
	export let darkMode: boolean
	export let data: {
		modules: FlowModule[]
		index: number
		eventHandlers: GraphEventHandlers
		disableAi: boolean
		flowIsSimplifiable: boolean
	}

	const dispatch = createEventDispatcher()

	let simplifiedTriggers = false
	let triggerScriptModule: FlowModule | undefined = undefined
	//$: triggerScriptModule = data.modules.find((mod) => mod.isTrigger)

	$: data.eventHandlers.simplifyFlow(simplifiedTriggers)

	$: items = [
		{
			id: 1,
			type: 'text',
			data: { text: 'Triggers' },
			display: !simplifiedTriggers
		},
		{
			id: 2,
			type: 'triggersBadge',
			data: {},
			display: !simplifiedTriggers
		},
		{
			id: 3,
			type: 'insertButton',
			data: {},
			display: !data.flowIsSimplifiable || simplifiedTriggers,
			grow: true
		}
	]

	$: visibleItems = items.filter((item) => item.display)
</script>

<div style={`width: ${NODE.width}px;`} class="center-center">
	<button
		class=" w-full border rounded-sm bg-surface shadow-md center-center items-center max-w-full
			{selected ? 'outline outline-offset-0  outline-2  outline-slate-500 dark:outline-gray-400' : ''}"
		on:click={() => dispatch('select')}
	>
		<div class="flex flex-row w-min-0 gap-2.5 w-fit max-w-full px-2 py-1">
			{#each visibleItems as item (item.id)}
				<div class="grow {item.grow ? 'grow' : 'shrink-0'} min-w-0 center-center">
					{#if item.type === 'triggersBadge'}
						<TriggersBadge
							showOnlyWithCount={false}
							{path}
							{newItem}
							{isFlow}
							{selected}
							on:select
						/>
					{:else if item.type === 'text'}
						<div class="grow min-w-0 items-center text-2xs font-normal">
							{item.data.text}
						</div>
					{:else if item.type === 'insertButton'}
						{#if !triggerScriptModule}
							<InsertModuleButton
								disableAi={data.disableAi}
								on:new={(e) => {
									dispatch('new', e.detail)
									simplifiedTriggers = true
								}}
								on:pickScript={(e) => {
									dispatch('pickScript', e.detail)
									simplifiedTriggers = true
								}}
								kind="trigger"
								index={data?.index ?? 0}
								modules={data?.modules ?? []}
								buttonClasses={twMerge(
									'bg-surface-secondary hover:bg-surface-hover rounded-md border text-xs',
									'w-6 h-6',
									'relative center-center',
									newItem ? 'cursor-not-allowed bg-surface-disabled' : 'cursor-pointer',
									'flex-shrink-0'
								)}
							/>
						{:else if simplifiedTriggers}
							<div
								class="text-2xs text-secondary min-w-0 font-normal text-center rounded-sm grow shadow-md w-full border"
							>
								<MapItem
									mod={triggerScriptModule}
									insertable={false}
									bgColor={getStateColor(undefined, darkMode, true, false)}
									modules={data.modules ?? []}
									moving={''}
									flowJobs={undefined}
									on:delete={(e) => {
										data.eventHandlers.delete(e.detail, '')
									}}
									on:insert={(e) => {
										data.eventHandlers.insert(e.detail)
									}}
									on:changeId={(e) => {
										data.eventHandlers.changeId(e.detail)
									}}
									on:move={(e) => {
										if (triggerScriptModule) {
											data.eventHandlers.move(triggerScriptModule, data.modules)
										}
									}}
									on:newBranch={(e) => {
										if (triggerScriptModule) {
											data.eventHandlers.newBranch(triggerScriptModule)
										}
									}}
									on:select={(e) => {
										data.eventHandlers.select(e.detail)
									}}
								/>
							</div>
						{/if}
					{/if}
				</div>
			{/each}

			{#if data.flowIsSimplifiable}
				<div class="absolute -top-5 right-0 text-2xs">
					<Popover notClickable placement="auto">
						<button
							class="absolute top-[12px] -right-[10px] rounded-full h-[20px] w-[20px] trash center-center text-secondary
	outline-[1px] outline dark:outline-gray-500 outline-gray-300 bg-surface duration-150 hover:bg-nord-950 hover:text-white"
							on:click|preventDefault|stopPropagation={() =>
								(simplifiedTriggers = !simplifiedTriggers)}
						>
							{#if simplifiedTriggers}
								<Maximize2 size={12} strokeWidth={2} />
							{:else}
								<Minimize2 size={12} strokeWidth={2} />
							{/if}
						</button>
						<svelte:fragment slot="text"
							>{simplifiedTriggers
								? 'Expand schedule poll nodes'
								: 'Collapse schedule poll nodes'}</svelte:fragment
						>
					</Popover>
				</div>
			{/if}

			{#if simplifiedTriggers}
				<div class="absolute text-sm right-4 -bottom-3 flex flex-row gap-1 z-10">
					<Popover notClickable>
						<div
							transition:fade|local={{ duration: 200 }}
							class="center-center bg-surface rounded border border-gray-400 text-secondary px-1 py-0.5"
						>
							<Square size={12} />
						</div>
						<svelte:fragment slot="text">Early stop/break</svelte:fragment>
					</Popover>
				</div>
			{/if}
		</div>
	</button>
</div>
