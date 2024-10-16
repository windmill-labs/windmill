<script lang="ts">
	import { Calendar, Mail, Webhook, Square, Maximize2, Minimize2 } from 'lucide-svelte'
	import TriggerButton from './TriggerButton.svelte'
	import { NODE } from '../../util'
	import { HttpTriggerService, ScheduleService } from '$lib/gen'
	import { userStore, workspaceStore } from '$lib/stores'
	import Popover from '$lib/components/Popover.svelte'
	import TriggerCount from './TriggerCount.svelte'
	import { createEventDispatcher, onMount } from 'svelte'
	import { Route } from 'lucide-svelte'
	import { canWrite } from '$lib/utils'
	import { getContext } from 'svelte'
	import type { FlowEditorContext } from '../../../flows/types'
	import InsertModuleButton from '../../../flows/map/InsertTriggersButton.svelte'
	import type { FlowModule } from '$lib/gen'
	import type { GraphEventHandlers } from '../../graphBuilder'
	import { twMerge } from 'tailwind-merge'
	import MapItem from '../../../flows/map/MapItem.svelte'
	import { fade } from 'svelte/transition'
	import { getStateColor } from '../../util'

	export let darkMode: boolean
	export let path: string
	export let isEditor: boolean
	export let newFlow: boolean
	export let data: {
		modules: FlowModule[]
		index: number
		eventHandlers: GraphEventHandlers
		disableAi: boolean
		flowIsSimplifiable: boolean
	}

	const dispatch = createEventDispatcher()

	async function loadSchedules() {
		if (!path) return
		try {
			$primarySchedule = await ScheduleService.existsSchedule({
				workspace: $workspaceStore ?? '',
				path
			})

			$schedules = (
				await ScheduleService.listSchedules({
					workspace: $workspaceStore ?? '',
					path: path,
					isFlow: true
				})
			).filter((s) => s.path != path)
		} catch (e) {
			console.error('impossible to load schedules')
		}
	}

	async function loadTriggers() {
		try {
			$httpTriggers = (
				await HttpTriggerService.listHttpTriggers({
					workspace: $workspaceStore ?? '',
					path,
					isFlow: true
				})
			).map((x) => {
				return { canWrite: canWrite(x.path, x.extra_perms!, $userStore), ...x }
			})
		} catch (e) {
			console.error('impossible to load routes')
		}
	}

	onMount(() => {
		if (!newFlow) {
			loadSchedules()
			loadTriggers()
			console.log('triggerWrapper mounted')
		}
	})

	const { httpTriggers, selectedId, selectedTrigger, schedules, primarySchedule } =
		getContext<FlowEditorContext>('FlowEditorContext') ?? {}

	let simplifiedTriggers = false

	let triggerScriptModule: FlowModule | undefined = undefined
	$: triggerScriptModule = data.modules.find((mod) => mod.isTrigger)

	$: console.log('triggerScriptModule', triggerScriptModule)

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
			type: 'popover',
			data: { triggerType: 'mail', icon: Mail },
			display: !simplifiedTriggers
		},
		{
			id: 3,
			type: 'popover',
			data: { triggerType: 'webhooks', icon: Webhook },
			display: !simplifiedTriggers
		},
		{
			id: 4,
			type: 'popover',
			data: { triggerType: 'routes', icon: Route, count: $httpTriggers?.length },
			display: !simplifiedTriggers
		},
		{
			id: 5,
			type: 'popover',
			data: {
				triggerType: 'schedules',
				icon: Calendar,
				count: ($schedules?.length ?? 0) + ($primarySchedule ? 1 : 0)
			},
			display: true
		},
		{
			id: 6,
			type: 'insertButton',
			data: {},
			display: !data.flowIsSimplifiable || simplifiedTriggers,
			grow: true
		}
	]

	$: visibleItems = items.filter((item) => item.display)
</script>

{#if isEditor}
	<div style={`width: ${NODE.width}px;`} class="center-center">
		<button
			class=" w-full border rounded-sm bg-surface shadow-md center-center items-center max-w-full
			{$selectedId?.startsWith('triggers')
				? 'outline outline-offset-0  outline-2  outline-slate-500 dark:outline-gray-400'
				: ''}"
			on:click={() => ($selectedId = 'triggers')}
		>
			<div class="flex flex-row w-min-0 gap-2.5 w-fit max-w-full px-2 py-1">
				{#each visibleItems as item (item.id)}
					<div class="grow {item.grow ? 'grow' : 'shrink-0'} min-w-0 center-center">
						{#if item.type === 'popover'}
							<Popover>
								<svelte:fragment slot="text">
									{#if newFlow}
										Deploy the flow to see {item.data.triggerType} triggers
									{:else}
										See default {item.data.triggerType} triggers
									{/if}
								</svelte:fragment>
								<TriggerButton
									on:click={() => {
										if (isEditor) {
											$selectedTrigger = item.data.triggerType ?? ''
											$selectedId = 'triggers'
										} else {
											dispatch('triggerDetail', item.data.triggerType)
										}
									}}
									disabled={newFlow}
								>
									{#if item.data.count}
										<TriggerCount count={item.data.count} />
									{/if}
									<svelte:component this={item.data.icon} size={12} />
								</TriggerButton>
							</Popover>
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
										newFlow ? 'cursor-not-allowed bg-surface-disabled' : 'cursor-pointer',
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
{/if}
