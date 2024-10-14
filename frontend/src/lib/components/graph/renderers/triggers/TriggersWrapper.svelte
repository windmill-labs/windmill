<script lang="ts">
	import { Calendar, Mail, Webhook, Repeat } from 'lucide-svelte'
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
	import InsertTriggersButton from '../../../flows/map/InsertTriggersButton.svelte'
	import TopLevelNode from '$lib/components/flows/pickers/TopLevelNode.svelte'
	import type { TriggerContext } from '../../../triggers'
	export let path: string
	export let isEditor: boolean
	export let newFlow: boolean

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

	const { triggerModule } = getContext<TriggerContext>('TriggerContext') ?? {}

	import { writable } from 'svelte/store'
	import { pickScript } from '$lib/components/flows/flowStateUtils'

	async function pickScriptEventHandler(path: string, summary: string, id: string) {
		var module
		var state
		;[module, state] = await pickScript(path, summary, id)
		$triggerModule = module
	}
	const modifiableTriggers = writable<
		{ type: 'routes' | 'webhooks' | 'schedule' | 'email' | 'schedule_poll' }[]
	>([{ type: 'schedule' }])

	function addTrigger(triggerType: 'routes' | 'webhooks' | 'schedule' | 'email' | 'schedule_poll') {
		modifiableTriggers.update((triggers) => [...triggers, { type: triggerType }])
	}

	/* function removeTrigger(
		triggerType: 'routes' | 'webhooks' | 'schedule' | 'email' | 'schedule_poll'
	) {
		modifiableTriggers.update((triggers) => triggers.filter((t) => t.type !== triggerType))
	} */
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
				on:click={() => ($selectedId = 'triggers')}
			>
				<div class="flex flex-col">
					<div class="flex flex-row items-center text-2xs font-normal"> Triggers </div>
				</div>
				{#if $modifiableTriggers.some((trigger) => trigger.type === 'webhooks')}
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
									$selectedId = 'webhooks'
								} else {
									dispatch('triggerDetail', 'webhooks')
								}
							}}
							disabled={newFlow}
						>
							<Webhook size={12} />
						</TriggerButton>
					</Popover>
				{/if}
				{#if $modifiableTriggers.some((trigger) => trigger.type === 'email')}
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
									$selectedId = 'mail'
								} else {
									dispatch('triggerDetail', 'mail')
								}
							}}
							disabled={newFlow}
						>
							<Mail size={12} />
						</TriggerButton>
					</Popover>
				{/if}
				{#if $modifiableTriggers.some((trigger) => trigger.type === 'routes')}
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
									$selectedId = 'routes'
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
				{#if $modifiableTriggers.some((trigger) => trigger.type === 'schedule')}
					<Popover>
						<svelte:fragment slot="text">See all schedules triggers</svelte:fragment>
						<TriggerButton
							on:click={() => {
								if (isEditor) {
									$selectedTrigger = 'schedules'
									$selectedId = 'schedules'
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
				{/if}
				{#if $modifiableTriggers.some((trigger) => trigger.type === 'schedule_poll')}
					<Popover>
						<svelte:fragment slot="text">See all schedules poll triggers</svelte:fragment>
						<TriggerButton
							on:click={() => {
								if (isEditor) {
									$selectedTrigger = 'schedule_poll'
									$selectedId = 'schedule_poll'
								} else {
									dispatch('triggerDetail', 'schedule_poll')
								}
							}}
							disabled={newFlow}
						>
							<Repeat size={12} />
						</TriggerButton>
					</Popover>
				{/if}

				<!-- index={data.index ?? 0}
					allowTrigger={data.enableTrigger}
					modules={data?.modules ?? []} -->
				<InsertTriggersButton
					on:new={(e) => {
						// console.log('new', e)
						$selectedId = 'triggers'
						$selectedTrigger = 'schedule_poll'
						addTrigger('schedule_poll')
					}}
					on:pickScript={(e) => {
						pickScriptEventHandler(e.detail.path, e.detail.summary, e.detail.id)
						// console.log('pickScript', e)
						close(null)
					}}
				>
					<svelte:fragment slot="topLevelNodes" let:close>
						{#each ['schedule', 'routes', 'email', 'webhooks', 'schedule_poll'] as triggersType}
							{#if !$modifiableTriggers.some((trigger) => trigger.type === triggersType)}
								{#if triggersType === 'schedule'}
									<TopLevelNode
										label="Schedule"
										on:select={() => {
											close(null)
											addTrigger('schedule')
										}}
									/>
								{:else if triggersType === 'routes'}
									<TopLevelNode
										label="Http route"
										on:select={() => {
											close(null)
											addTrigger('routes')
										}}
									/>
								{:else if triggersType === 'email'}
									<TopLevelNode
										label="Email"
										on:select={() => {
											close(null)
											addTrigger('email')
										}}
									/>
								{:else if triggersType === 'webhooks'}
									<TopLevelNode
										label="Webhook"
										on:select={() => {
											close(null)
											addTrigger('webhooks')
										}}
									/>
								{:else if triggersType === 'schedule_poll'}
									<TopLevelNode label="Schedule poll" />
								{/if}
							{/if}
						{/each}
					</svelte:fragment>
				</InsertTriggersButton>
			</button>
		</div>
	</div>
{/if}
