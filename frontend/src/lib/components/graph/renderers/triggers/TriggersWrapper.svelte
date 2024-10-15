<script lang="ts">
	import { Calendar, Mail, Webhook } from 'lucide-svelte'
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
	import Toggle from '$lib/components/Toggle.svelte'

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

	$: data.eventHandlers.simplifyFlow(simplifiedTriggers)
</script>

{#if isEditor}
	<div style={`width: ${NODE.width}px;`}>
		<div class="flex flex-row mx-auto w-min">
			<button
				class="flex flex-row gap-2 px-2 border p-1 rounded-md bg-surface shadow-md items-center {$selectedId?.startsWith(
					'triggers'
				)
					? 'outline outline-offset-1 outline-2  outline-slate-900 dark:bg-white/5 dark:outline-slate-800/60 dark:border-gray-400'
					: ''}"
				on:click|self={() => ($selectedId = 'triggers')}
			>
				{#if !simplifiedTriggers}
					<div class="flex flex-col">
						<div class="flex flex-row items-center text-2xs font-normal"> Triggers </div>
					</div>

					<Popover>
						<svelte:fragment slot="text">
							{#if newFlow}
								Deploy the flow to see webhooks triggers
							{:else}
								See default webhooks triggers
							{/if}
						</svelte:fragment>
						<TriggerButton
							on:click={() => {
								if (isEditor) {
									$selectedTrigger = 'webhooks'
									$selectedId = 'triggers'
								} else {
									dispatch('triggerDetail', 'webhooks')
								}
							}}
							disabled={newFlow}
						>
							<Webhook size={12} />
						</TriggerButton>
					</Popover>

					<Popover>
						<svelte:fragment slot="text">
							{#if newFlow}
								Deploy the flow to see email triggers
							{:else}
								See all email triggers
							{/if}
						</svelte:fragment>
						<TriggerButton
							on:click={() => {
								if (isEditor) {
									$selectedTrigger = 'mail'
									$selectedId = 'triggers'
								} else {
									dispatch('triggerDetail', 'mail')
								}
							}}
							disabled={newFlow}
						>
							<Mail size={12} />
						</TriggerButton>
					</Popover>

					<Popover>
						<svelte:fragment slot="text">
							{#if newFlow}
								Deploy the flow to add routes triggers
							{:else}
								See all routes triggers
							{/if}
						</svelte:fragment>
						<TriggerButton
							on:click={() => {
								if (isEditor) {
									$selectedTrigger = 'routes'
									$selectedId = 'triggers'
								} else {
									dispatch('triggerDetail', 'routes')
								}
							}}
							disabled={newFlow}
						>
							<TriggerCount count={$httpTriggers?.length} />
							<Route size={12} />
						</TriggerButton>
					</Popover>
				{/if}

				<Popover>
					<svelte:fragment slot="text">See all schedules triggers</svelte:fragment>
					<TriggerButton
						on:click={() => {
							if (isEditor) {
								$selectedTrigger = 'schedules'
								$selectedId = 'triggers'
							} else {
								dispatch('triggerDetail', 'schedule')
							}
						}}
						disabled={newFlow}
					>
						<TriggerCount count={($schedules?.length ?? 0) + ($primarySchedule ? 1 : 0)} />
						<Calendar size={12} />
					</TriggerButton>
				</Popover>

				<!-- index={data.index ?? 0}
					allowTrigger={data.enableTrigger}
					modules={data?.modules ?? []} -->

				{#if !simplifiedTriggers && !data.flowIsSimplifiable}
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
							newFlow ? 'cursor-not-allowed bg-surface-disabled' : 'cursor-pointer'
						)}
					/>
				{/if}

				{#if triggerScriptModule && simplifiedTriggers}
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
				{/if}

				{#if data.flowIsSimplifiable}
					<Toggle
						size="xs"
						label="Simplify view"
						bind:checked={simplifiedTriggers}
						on:change={(e) => {
							e.stopPropagation()
						}}
					/>
				{/if}
			</button>
		</div>
	</div>
{/if}
