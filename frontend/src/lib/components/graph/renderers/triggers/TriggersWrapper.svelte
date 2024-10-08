<script lang="ts">
	import { Calendar, Mail, Webhook } from 'lucide-svelte'
	import TriggerButton from './TriggerButton.svelte'
	import { NODE } from '../../util'
	import { HttpTriggerService, ScheduleService, type HttpTrigger, type Schedule } from '$lib/gen'
	import { userStore, workspaceStore } from '$lib/stores'
	import Popover from '$lib/components/Popover.svelte'
	import TriggerCount from './TriggerCount.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import { createEventDispatcher, onMount } from 'svelte'
	import { Route } from 'lucide-svelte'
	import { canWrite } from '$lib/utils'
	import { getContext } from 'svelte'
	import type { FlowEditorContext } from '../../../flows/types'

	let schedules: Schedule[] | undefined = undefined
	let triggers: (HttpTrigger & { canWrite: boolean })[] | undefined = undefined

	export let path: string
	export let isEditor: boolean
	export let newFlow: boolean

	let primaryScheduleExists: boolean = false

	const dispatch = createEventDispatcher()

	async function loadSchedules() {
		if (!path) return
		try {
			primaryScheduleExists = await ScheduleService.existsSchedule({
				workspace: $workspaceStore ?? '',
				path
			})

			schedules = (
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
			triggers = (
				await HttpTriggerService.listHttpTriggers({
					workspace: $workspaceStore ?? '',
					path,
					isFlow: true
				})
			).map((x) => {
				return { canWrite: canWrite(x.path, x.extra_perms!, $userStore), ...x }
			})
		} catch (e) {
			console.error('impossible to load http routes')
		}
	}

	onMount(() => {
		if (!newFlow) {
			loadSchedules()
			loadTriggers()
			console.log('triggerWrapper mounted')
		}
	})

	const flowEditorContext = getContext<FlowEditorContext>('FlowEditorContext')
	const selectedId = flowEditorContext?.selectedId ?? { subscribe: () => () => {} }
	const selectedTrigger = flowEditorContext?.selectedTrigger ?? { subscribe: () => () => {} }
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
					<div class="flex flex-row items-center text-2xs font-normal">
						Triggers

						<Tooltip small wrapperClass="center-center">
							The flow can be triggered by webhooks, emails, schedules or routes. Click on the icons
							to see the triggers.
						</Tooltip>
					</div>
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
						<TriggerCount count={triggers?.length} />
						<Route size={12} />
					</TriggerButton>
				</Popover>
				<Popover>
					<svelte:fragment slot="text">
						{#if newFlow}
							Deploy the flow to add schedules triggers
						{:else}
							See all schedules triggers
						{/if}
					</svelte:fragment>
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
						<TriggerCount count={(schedules?.length ?? 0) + (primaryScheduleExists ? 1 : 0)} />
						<Calendar size={12} />
					</TriggerButton>
				</Popover>
			</button>
		</div>
	</div>
{/if}
