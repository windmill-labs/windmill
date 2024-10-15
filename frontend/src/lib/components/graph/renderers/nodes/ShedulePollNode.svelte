<script lang="ts">
	import NodeWrapper from './NodeWrapper.svelte'
	import { Repeat, X } from 'lucide-svelte'
	import { getContext } from 'svelte'
	import type { Writable } from 'svelte/store'
	import type { GraphEventHandlers } from '../../graphBuilder'
	import { twMerge } from 'tailwind-merge'
	import { NODE } from '../../util'
	import InsertModuleButton from '../../../flows/map/InsertModuleButton.svelte'
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

	let forloopModules: FlowModule[] = []
	let triggerModule: FlowModule | undefined = undefined

	function getSchedulePollModules() {
		let forloopModule = data.modules.find((mod) => mod.value.type === 'forloopflow')
		if (forloopModule) {
			forloopModules = forloopModule.value.modules
		}
		console.log('Forloop Modules', forloopModules)
		if (data.modules.length > 0) {
			triggerModule = data.modules[0]
		}
		console.log('Trigger Module', triggerModule)
	}

	$: getSchedulePollModules()
</script>

<NodeWrapper wrapperClass="!shadow-none">
	<div
		style="width: {NODE.width}px; min-height: {NODE.height}px; transform: translate(0, calc({NODE.height}px - {componentHeight}px));"
	>
		<button
			class={twMerge(
				'flex flex-col gap-2 rounded-sm p-1 border absolute grow w-full',
				$selectedId === 'schedulePoll'
					? 'outline outline-offset-0 outline-2 outline-slate-500 dark:outline-gray-400'
					: ''
			)}
			on:click={() => ($selectedId = 'schedulePoll')}
			bind:clientHeight={componentHeight}
			on:mouseenter={() => (hover = true)}
			on:mouseleave={() => (hover = false)}
		>
			{#if triggerModule}
				<div
					class="text-2xs text-secondary font-normal text-center rounded-sm shadow-md w-[80%] border bg-surface"
				>
					<MapItem
						mod={triggerModule}
						insertable={true}
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
							if (triggerModule) {
								data.eventHandlers.move(triggerModule, data.modules)
							}
						}}
						on:newBranch={(e) => {
							if (triggerModule) {
								data.eventHandlers.newBranch(triggerModule)
							}
						}}
						on:select={(e) => {
							data.eventHandlers.select(e.detail)
						}}
					/>
				</div>
			{/if}
			{#each forloopModules as mod}
				<div class="text-2xs text-secondary font-normal w-[80%] ml-auto shadow-sm">
					<MapItem
						{mod}
						insertable={true}
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
							data.eventHandlers.move(mod, data.modules)
						}}
						on:newBranch={(e) => {
							data.eventHandlers.newBranch(mod)
						}}
						on:select={(e) => {
							data.eventHandlers.select(e.detail)
						}}
					/>
				</div>
			{/each}

			<div class="text-2xs text-secondary font-normal w-[80%] ml-auto">
				<div class="flex justify-center items-center w-full">
					<InsertModuleButton />
				</div>
			</div>
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
