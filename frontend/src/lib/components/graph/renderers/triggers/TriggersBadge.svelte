<script lang="ts">
	import { Calendar, Mail, Webhook, Unplug } from 'lucide-svelte'
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
	export let triggersToDisplay: ('webhooks' | 'schedules' | 'routes' | 'websockets' | 'emails')[] =
		['emails']
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

	$: triggerTypeConfig = {
		webhooks: { icon: Webhook, countKey: 'webhook_count' },
		schedules: { icon: Calendar, countKey: 'schedule_count' },
		routes: { icon: Route, countKey: 'http_routes_count' },
		websockets: { icon: Unplug, countKey: 'websocket_count' },
		emails: { icon: Mail, countKey: 'email_count' }
	}

	$: filteredTriggerTypes = triggersToDisplay.map((type) => ({
		type,
		...triggerTypeConfig[type]
	}))
</script>

{#each filteredTriggerTypes as { type, icon, countKey }}
	{#if !showOnlyWithCount || ($triggersCount?.[countKey] ?? 0) > 0}
		<Popover>
			<svelte:fragment slot="text">{type.charAt(0).toUpperCase() + type.slice(1)}</svelte:fragment>
			<TriggerButton
				on:click={() => {
					$selectedTrigger = type
					dispatch('select')
				}}
				selected={selected && $selectedTrigger === type}
			>
				<TriggerCount count={$triggersCount?.[countKey]} />
				<svelte:component this={icon} size={12} />
			</TriggerButton>
		</Popover>
	{/if}
{/each}
