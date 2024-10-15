<script lang="ts">
	import NodeWrapper from './NodeWrapper.svelte'
	import { Repeat, X } from 'lucide-svelte'
	import { getContext } from 'svelte'
	import type { Writable } from 'svelte/store'
	import type { GraphEventHandlers } from '../../graphBuilder'
	import { twMerge } from 'tailwind-merge'
	import { NODE } from '../../util'
	import type { FlowModule } from '$lib/gen'
	import MapItem from '../../../flows/map/MapItem.svelte'

	export let data: {
		bgColor: string
		eventHandlers: GraphEventHandlers
		modules: FlowModule[]
	}

	const { selectedId } = getContext<{ selectedId: Writable<string> }>('FlowGraphContext')

	/* async function deleteSchedulePoll(e) {
		data.eventHandlers.delete(e.detail, '')
	} */
	let componentHeight = NODE.height
	let hover = false

	console.log('Modules', data.modules)

	let triggerScriptModule: FlowModule | undefined = undefined

	function getTriggerScriptModule() {
		triggerScriptModule = data.modules.find((mod) => mod.isTrigger)
		console.log('Trigger Module', triggerScriptModule)
	}

	$: getTriggerScriptModule(), data.modules
</script>

<NodeWrapper wrapperClass="!shadow-none">
	<div
		class="flex flex-row"
		style="width: {NODE.width}px; min-height: {NODE.height}px; transform: translate(0, calc({NODE.height}px - {componentHeight}px));"
	>
		<button
			class={twMerge(
				'flex flex-row gap-2 rounded-sm p-2 border absolute grow w-full items-center',
				$selectedId === 'schedulePoll'
					? 'outline outline-offset-0 outline-2 outline-slate-500 dark:outline-gray-400'
					: ''
			)}
			on:click={() => ($selectedId = 'schedulePoll')}
			bind:clientHeight={componentHeight}
			on:mouseenter={() => (hover = true)}
			on:mouseleave={() => (hover = false)}
		>
			<div class="flex flex-row gap-2">
				<Repeat class="w-4 h-4 text-secondary" />
			</div>
			{#if triggerScriptModule}
				<div
					class="text-2xs text-secondary min-w-0 font-normal text-center rounded-sm shrink shadow-md w-full border bg-surface"
				>
					<MapItem
						mod={triggerScriptModule}
						insertable={false}
						bgColor={'#ffffff'}
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
			{:else}
				<div> loading ... </div>
			{/if}

			<button
				class="absolute -top-[10px] -right-[10px] rounded-full h-[20px] w-[20px] trash center-center text-secondary
	outline-[1px] outline dark:outline-gray-500 outline-gray-300 bg-surface duration-150 hover:bg-red-400 hover:text-white
	 {hover || $selectedId === 'schedulePoll' ? '' : '!hidden'}"
				title="Delete"
				on:click|preventDefault|stopPropagation={() => {
					data.eventHandlers.removeSchedulePoll()
					console.log('remove schedule Poll')
				}}
			>
				<X class="mx-[3px]" size={12} strokeWidth={2} />
			</button>
		</button>
	</div>
</NodeWrapper>
