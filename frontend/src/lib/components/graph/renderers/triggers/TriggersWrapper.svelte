<script lang="ts">
	import { Calendar, Mail, Webhook } from 'lucide-svelte'
	import TriggerButton from './TriggerButton.svelte'
	import { NODE } from '../../util'

	import Popover from '$lib/components/Popover.svelte'
	import TriggerCount from './TriggerCount.svelte'
	import { createEventDispatcher, onMount } from 'svelte'
	import { Route } from 'lucide-svelte'
	import { getContext } from 'svelte'
	import { type TriggerContext } from '$lib/components/triggers'
	import { FlowService, ScriptService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import type { Writable } from 'svelte/store'

	const { selectedTrigger, triggersCount } = getContext<TriggerContext>('TriggerContext')
	const { selectedId } = getContext<{
		selectedId: Writable<string | undefined>
	}>('FlowGraphContext')

	export let path: string
	export let newItem: boolean
	export let isFlow: boolean

	const dispatch = createEventDispatcher()

	// async function loadSchedules() {
	// 	if (!path) return
	// 	try {
	// 		let primaryScheduleExists = await ScheduleService.existsSchedule({
	// 			workspace: $workspaceStore ?? '',
	// 			path
	// 		})

	// 		if (primaryScheduleExists) {
	// 			$primarySchedule = await ScheduleService.getSchedule({
	// 				workspace: $workspaceStore ?? '',
	// 				path
	// 			})
	// 		} else {
	// 			$primarySchedule = false
	// 		}

	// 		$schedules = (
	// 			await ScheduleService.listSchedules({
	// 				workspace: $workspaceStore ?? '',
	// 				path: path,
	// 				isFlow: true
	// 			})
	// 		).filter((s) => s.path != path)
	// 	} catch (e) {
	// 		console.error('impossible to load schedules')
	// 	}
	// }

	// async function loadTriggers() {
	// 	try {
	// 		$httpTriggers = (
	// 			await HttpTriggerService.listHttpTriggers({
	// 				workspace: $workspaceStore ?? '',
	// 				path,
	// 				isFlow: true
	// 			})
	// 		).map((x) => {
	// 			return { canWrite: canWrite(x.path, x.extra_perms!, $userStore), ...x }
	// 		})
	// 	} catch (e) {
	// 		console.error('impossible to load http routes')
	// 	}
	// }

	onMount(() => {
		if (!newItem) {
			loadCount()
			// loadSchedules()
			// loadTriggers()
		}
	})

	async function loadCount() {
		if (isFlow) {
			$triggersCount = await FlowService.getTriggersCountOfFlow({
				workspace: $workspaceStore!,
				path
			})
		} else {
			$triggersCount = await ScriptService.getTriggersCountOfScript({
				workspace: $workspaceStore!,
				path
			})
		}
	}
</script>

<div style={`width: ${NODE.width}px;`}>
	<div class="flex flex-row mx-auto w-min">
		<button
			class="flex flex-row gap-2 px-2 border p-1 rounded-md bg-surface shadow-md items-center {$selectedId?.startsWith(
				'triggers'
			)
				? 'outline outline-offset-1 outline-2  outline-slate-900 dark:bg-white/5 dark:outline-slate-800/60 dark:border-gray-400'
				: ''}"
			on:click={() => {
				$selectedId = 'triggers'
				dispatch('select')
			}}
		>
			<div class="flex flex-col">
				<div class="flex flex-row items-center text-2xs font-normal"> Triggers </div>
			</div>
			<Popover>
				<svelte:fragment slot="text">
					{#if newItem}
						Deploy the flow to see webhooks triggers
					{:else}
						See default webhooks triggers
					{/if}
				</svelte:fragment>
				<TriggerButton
					on:click={() => {
						$selectedTrigger = 'webhooks'
						$selectedId = 'triggers'
						dispatch('select')
					}}
					selected={$selectedId == 'triggers' && $selectedTrigger === 'webhooks'}
				>
					<TriggerCount count={$triggersCount?.webhook_count} />
					<Webhook size={12} />
				</TriggerButton>
			</Popover>
			<Popover>
				<svelte:fragment slot="text">See all schedules triggers</svelte:fragment>
				<TriggerButton
					on:click={() => {
						$selectedTrigger = 'schedules'
						$selectedId = 'triggers'
						dispatch('select')
					}}
					selected={$selectedId == 'triggers' && $selectedTrigger === 'schedules'}
				>
					<TriggerCount count={$triggersCount?.schedule_count} />
					<Calendar size={12} />
				</TriggerButton>
			</Popover>
			<Popover>
				<svelte:fragment slot="text">
					{#if newItem}
						Deploy the flow to add routes triggers
					{:else}
						See all routes triggers
					{/if}
				</svelte:fragment>
				<TriggerButton
					on:click={() => {
						$selectedTrigger = 'routes'
						$selectedId = 'triggers'
						dispatch('select')
					}}
					selected={$selectedId == 'triggers' && $selectedTrigger === 'routes'}
				>
					<TriggerCount count={$triggersCount?.http_routes_count} />
					<Route size={12} />
				</TriggerButton>
			</Popover>
			<Popover>
				<svelte:fragment slot="text">
					{#if newItem}
						Deploy the flow to see email triggers
					{:else}
						See all email triggers
					{/if}
				</svelte:fragment>
				<TriggerButton
					on:click={() => {
						$selectedTrigger = 'emails'
						$selectedId = 'triggers'
						dispatch('select')
					}}
					selected={$selectedId == 'triggers' && $selectedTrigger === 'emails'}
				>
					<TriggerCount count={$triggersCount?.email_count} />
					<Mail size={12} />
				</TriggerButton>
			</Popover>
		</button>
	</div>
</div>
