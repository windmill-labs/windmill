<script lang="ts">
	import { Calendar, Mail, Webhook } from 'lucide-svelte'
	import TriggerButton from './TriggerButton.svelte'

	import Popover from '$lib/components/Popover.svelte'
	import TriggerCount from './TriggerCount.svelte'
	import { createEventDispatcher, onMount } from 'svelte'
	import { Route } from 'lucide-svelte'
	import { getContext } from 'svelte'
	import { type TriggerContext } from '$lib/components/triggers'
	import { FlowService, ScriptService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'

	const { selectedTrigger, triggersCount } = getContext<TriggerContext>('TriggerContext')

	export let path: string
	export let newItem: boolean
	export let isFlow: boolean
	export let selected: boolean
	export let showOnlyWithCount: boolean
	const dispatch = createEventDispatcher()

	onMount(() => {
		if (!newItem) {
			loadCount()
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

{#if !showOnlyWithCount || ($triggersCount?.webhook_count ?? 0) > 0}
	<Popover>
		<svelte:fragment slot="text">Webhooks</svelte:fragment>
		<TriggerButton
			on:click={() => {
				$selectedTrigger = 'webhooks'
				dispatch('select')
			}}
			selected={selected && $selectedTrigger === 'webhooks'}
		>
			<TriggerCount count={$triggersCount?.webhook_count} />
			<Webhook size={12} />
		</TriggerButton>
	</Popover>
{/if}

{#if !showOnlyWithCount || ($triggersCount?.schedule_count ?? 0) > 0}
	<Popover>
		<svelte:fragment slot="text">Schedules</svelte:fragment>
		<TriggerButton
			on:click={() => {
				$selectedTrigger = 'schedules'
				dispatch('select')
			}}
			selected={selected && $selectedTrigger === 'schedules'}
		>
			<TriggerCount count={$triggersCount?.schedule_count} />
			<Calendar size={12} />
		</TriggerButton>
	</Popover>
{/if}

{#if !showOnlyWithCount || ($triggersCount?.http_routes_count ?? 0) > 0}
	<Popover>
		<svelte:fragment slot="text">HTTP Routes</svelte:fragment>
		<TriggerButton
			on:click={() => {
				$selectedTrigger = 'routes'
				dispatch('select')
			}}
			selected={selected && $selectedTrigger === 'routes'}
		>
			<TriggerCount count={$triggersCount?.http_routes_count} />
			<Route size={12} />
		</TriggerButton>
	</Popover>
{/if}

{#if !showOnlyWithCount || ($triggersCount?.email_count ?? 0) > 0}
	<Popover>
		<svelte:fragment slot="text">Emails</svelte:fragment>
		<TriggerButton
			on:click={() => {
				$selectedTrigger = 'emails'
				dispatch('select')
			}}
			selected={selected && $selectedTrigger === 'emails'}
		>
			<TriggerCount count={$triggersCount?.email_count} />
			<Mail size={12} />
		</TriggerButton>
	</Popover>
{/if}
