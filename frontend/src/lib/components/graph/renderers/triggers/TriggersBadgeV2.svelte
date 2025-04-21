<script lang="ts">
	import { Calendar, Mail, Webhook, Unplug, Database } from 'lucide-svelte'
	import Popover from '$lib/components/Popover.svelte'
	import TriggerCount from './TriggerCount.svelte'
	import { createEventDispatcher, onMount, type ComponentType } from 'svelte'
	import { Route } from 'lucide-svelte'
	import { getContext } from 'svelte'
	import { type TriggerContext } from '$lib/components/triggers'
	import { FlowService, ScriptService } from '$lib/gen'
	import { enterpriseLicense, workspaceStore } from '$lib/stores'
	import { MqttIcon, NatsIcon, KafkaIcon, AwsIcon } from '$lib/components/icons'
	import type { Trigger, TriggerType } from '$lib/components/triggers/utils'
	import { Menu, Menubar, MeltButton, MenuItem } from '$lib/components/meltComponents'
	import { twMerge } from 'tailwind-merge'

	const { selectedTriggerV2, triggersCount } = getContext<TriggerContext>('TriggerContext')

	export let path: string
	export let newItem: boolean
	export let isFlow: boolean
	export let selected: boolean
	export let showOnlyWithCount: boolean
	export let triggers: Trigger[] = []

	// Group triggers by their mapped type
	$: triggersGrouped = triggers.reduce(
		(acc, trigger) => {
			const configType = trigger.type

			if (!acc[configType]) {
				acc[configType] = []
			}
			acc[configType].push(trigger)
			return acc
		},
		{} as Record<TriggerType, Trigger[]>
	)

	// Extract unique trigger types for display
	$: triggersToDisplay = Object.keys(triggersGrouped) as TriggerType[]

	const dispatch = createEventDispatcher<{
		select: Trigger | undefined
	}>()

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

	const triggerTypeConfig: {
		[key in TriggerType]: { icon: ComponentType; countKey?: string }
	} = {
		webhook: { icon: Webhook, countKey: 'webhook_count' },
		schedule: { icon: Calendar, countKey: 'schedule_count' },
		http: { icon: Route, countKey: 'http_routes_count' },
		websocket: { icon: Unplug, countKey: 'websocket_count' },
		postgres: { icon: Database, countKey: 'postgres_count' },
		kafka: { icon: KafkaIcon, countKey: 'kafka_count' },
		email: { icon: Mail, countKey: 'email_count' },
		nats: { icon: NatsIcon, countKey: 'nats_count' },
		mqtt: { icon: MqttIcon, countKey: 'mqtt_count' },
		sqs: { icon: AwsIcon, countKey: 'sqs_count' }
	}

	function camelCaseToWords(s: string) {
		const result = s.replace(/([A-Z])/g, ' $1')
		return result.charAt(0).toUpperCase() + result.slice(1).toLowerCase()
	}

	const itemClass = twMerge(
		'text-secondary text-left font-normal w-full block px-4 py-2 text-2xs data-[highlighted]:bg-surface-hover data-[highlighted]:text-primary'
	)
</script>

<Menubar let:createMenu class="flex flex-row gap-1 items-center">
	{#each triggersToDisplay as type}
		{@const { icon, countKey } = triggerTypeConfig[type]}
		{@const isSelected = selected && $selectedTriggerV2 && $selectedTriggerV2.type === type}
		<Popover disablePopup={true} on:click={(e) => e.stopPropagation()}>
			<svelte:fragment slot="text">{camelCaseToWords(type)}</svelte:fragment>
			{#if triggersGrouped[type] && triggersGrouped[type].length === 1}
				<button
					class={twMerge(
						'hover:bg-surface-hover rounded-md border text-xs w-[23px] h-[23px] relative center-center cursor-pointer bg-surface',
						isSelected ? 'outline-1 outline-tertiary outline' : 'outline-0'
					)}
					on:click={() => {
						dispatch('select', triggersGrouped[type][0])
					}}
				>
					{#if countKey}
						<TriggerCount count={$triggersCount?.[countKey]} />
					{/if}
					<svelte:component this={icon} size={12} />
				</button>
			{:else}
				<Menu
					{createMenu}
					usePointerDownOutside
					placement="bottom-start"
					menuClass={'min-w-56 w-fit'}
					class="h-fit"
					let:item
				>
					<svelte:fragment slot="trigger" let:trigger>
						{#if (!showOnlyWithCount || ((countKey && $triggersCount?.[countKey]) || 0) > 0) && !(type === 'sqs' && !$enterpriseLicense) && !(type === 'kafka' && !$enterpriseLicense) && !(type === 'nats' && !$enterpriseLicense) && !(type === 'mqtt')}
							<MeltButton
								class={twMerge(
									'hover:bg-surface-hover rounded-md border text-xs w-[23px] h-[23px] relative center-center cursor-pointer bg-surface',
									isSelected ? 'outline-1 outline-tertiary outline' : 'outline-0'
								)}
								meltElement={trigger}
							>
								{#if countKey}
									<TriggerCount count={$triggersCount?.[countKey]} />
								{/if}
								<svelte:component this={icon} size={12} />
							</MeltButton>
						{/if}
					</svelte:fragment>

					{#if triggersGrouped[type] && triggersGrouped[type].length > 0}
						{#each triggersGrouped[type] as trigger}
							<MenuItem
								{item}
								class={itemClass}
								on:click={() => {
									dispatch('select', trigger)
								}}
							>
								<span class={trigger.isDraft ? 'text-frost-400 italic' : ''}>
									{trigger.isDraft ? `New ${trigger.type.replace(/s$/, '')} trigger` : trigger.path}
								</span>

								{#if trigger.isPrimary}
									<span
										class="ml-2 bg-blue-50 dark:bg-blue-900/40 px-1.5 py-0.5 rounded text-xs text-blue-700 dark:text-blue-100"
									>
										Primary
									</span>
								{/if}

								{#if trigger.isDraft}
									<span
										class="ml-2 text-2xs bg-frost-100 dark:bg-frost-900 text-frost-800 dark:text-frost-100 px-1.5 py-0.5 rounded"
									>
										Draft
									</span>
								{/if}
							</MenuItem>
						{/each}
					{:else}
						<div class="text-xs text-gray-400 p-2">No {camelCaseToWords(type)} triggers</div>
					{/if}
				</Menu>
			{/if}
		</Popover>
	{/each}
</Menubar>
