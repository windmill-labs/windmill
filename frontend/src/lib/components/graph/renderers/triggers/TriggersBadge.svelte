<script lang="ts">
	import { Calendar, Mail, Webhook, Unplug, Database, Terminal } from 'lucide-svelte'
	import { Loader2 } from 'lucide-svelte'
	import { createEventDispatcher, type ComponentType } from 'svelte'
	import { Route } from 'lucide-svelte'
	import { getContext } from 'svelte'
	import { type TriggerContext } from '$lib/components/triggers'
	import { enterpriseLicense } from '$lib/stores'
	import { MqttIcon, NatsIcon, KafkaIcon, AwsIcon, GoogleCloudIcon } from '$lib/components/icons'
	import type { Trigger, TriggerType } from '$lib/components/triggers/utils'
	import { Menu, Menubar, MeltButton, MenuItem, Tooltip } from '$lib/components/meltComponents'
	import { twMerge } from 'tailwind-merge'
	import SchedulePollIcon from '$lib/components/icons/SchedulePollIcon.svelte'
	import TriggerLabel from '$lib/components/triggers/TriggerLabel.svelte'

	const { selectedTrigger, triggersCount } = getContext<TriggerContext>('TriggerContext')

	interface Props {
		path: string
		newItem: boolean
		isFlow: boolean
		selected: boolean
		showOnlyWithCount: boolean
		triggers: Trigger[]
		numberOfTriggers?: number
		small?: boolean
		vertical?: boolean
		limit?: number
	}

	let {
		selected,
		showOnlyWithCount,
		triggers,
		// @ts-ignore - This is an output-only prop used with bind:
		numberOfTriggers = $bindable(0),
		small = true,
		vertical = false,
		limit
	}: Props = $props()

	let menuOpen = $state(false)

	const dispatch = createEventDispatcher<{
		select: Trigger | undefined
	}>()

	const triggerTypeConfig: {
		[key in TriggerType]: { icon: ComponentType; countKey?: string; disabled?: boolean }
	} = {
		webhook: { icon: Webhook, countKey: 'webhook_count' },
		schedule: { icon: Calendar, countKey: 'schedule_count' },
		http: { icon: Route, countKey: 'http_routes_count' },
		websocket: { icon: Unplug, countKey: 'websocket_count' },
		postgres: { icon: Database, countKey: 'postgres_count' },
		kafka: { icon: KafkaIcon, countKey: 'kafka_count', disabled: !$enterpriseLicense },
		email: { icon: Mail, countKey: 'email_count' },
		nats: { icon: NatsIcon, countKey: 'nats_count', disabled: !$enterpriseLicense },
		mqtt: { icon: MqttIcon, countKey: 'mqtt_count', disabled: !$enterpriseLicense },
		sqs: { icon: AwsIcon, countKey: 'sqs_count', disabled: !$enterpriseLicense },
		gcp: { icon: GoogleCloudIcon, countKey: 'gcp_count', disabled: !$enterpriseLicense },
		poll: { icon: SchedulePollIcon },
		cli: { icon: Terminal }
	}

	const allTypes = [
		'webhook',
		'schedule',
		'http',
		'websocket',
		'postgres',
		'kafka',
		'email',
		'nats',
		'mqtt',
		'sqs',
		'gcp',
		'poll',
		'cli'
	]

	function camelCaseToWords(s: string) {
		const result = s.replace(/([A-Z])/g, ' $1')
		return result.charAt(0).toUpperCase() + result.slice(1).toLowerCase()
	}

	const itemClass = twMerge(
		'text-secondary text-left font-normal w-full block px-4 py-2 text-2xs data-[highlighted]:bg-surface-hover data-[highlighted]:text-primary flex flex-row items-center flex-nowrap'
	)

	// Group triggers by their mapped type
	let triggersGrouped = $derived(
		triggers.reduce(
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
	)

	let noTriggers = $derived(triggers.length === 0)

	// Extract unique trigger types for display, only keep the first
	let allTriggerTypes = $derived.by(() => {
		const types = !noTriggers ? (Object.keys(triggersGrouped) as TriggerType[]) : allTypes
		//filter out types if showOnlyTriggersWithCount is true and there are no triggers for that type
		return types.filter(
			(type) =>
				(!showOnlyTriggersWithCount ||
					((triggerTypeConfig[type].countKey &&
						($triggersCount?.[triggerTypeConfig[type].countKey] ?? 0)) ||
						0) > 0) &&
				!triggerTypeConfig[type].disabled
		)
	})
	let triggersToDisplay = $derived(limit ? allTriggerTypes.slice(0, limit) : allTriggerTypes)
	let extraTriggers = $derived(
		limit && allTriggerTypes.length > limit
			? allTriggerTypes.slice(limit).flatMap((type) => triggersGrouped[type])
			: []
	)
	// Fallback for when there are no triggers in the store but we have the types
	let extraTriggersType = $derived(
		limit && allTriggerTypes.length > limit ? allTriggerTypes.slice(limit) : []
	)
	let showOnlyTriggersWithCount = $derived(showOnlyWithCount || triggers.length === 0)

	$effect(() => {
		if (allTriggerTypes) {
			numberOfTriggers = allTriggerTypes?.length
		}
	})
</script>

<Menubar
	class={twMerge(
		'items-center justify-center',
		vertical ? 'flex flex-col gap-2' : 'flex flex-row gap-1'
	)}
>
	{#snippet children({ createMenu })}
		{#each triggersToDisplay as type}
			{@const isSelected = selected && $selectedTrigger && $selectedTrigger.type === type}
			{@const singleItem =
				type === 'webhook' ||
				type === 'email'! ||
				(triggersGrouped[type] && triggersGrouped[type].length === 1)}
			<Tooltip
				disablePopup={menuOpen}
				placement={vertical ? 'right' : 'bottom'}
				on:click={(e) => e.stopPropagation()}
			>
				{#snippet text()}
					{camelCaseToWords(type)}
				{/snippet}
				{#if singleItem}
					{@render triggerButton({ type, isSelected, singleItem })}
				{:else}
					<Menu
						{createMenu}
						usePointerDownOutside
						placement={vertical ? 'right-start' : 'bottom'}
						menuClass={'max-w-56'}
						class="h-fit"
						bind:open={menuOpen}
						disabled={!triggersGrouped[type]}
					>
						{#snippet trigger({ trigger })}
							{@render triggerButton({
								type,
								isSelected,
								meltElement: trigger
							})}
						{/snippet}

						{#snippet children({ item })}
							{#if triggersGrouped[type] && triggersGrouped[type].length > 0}
								{#each triggersGrouped[type] as trigger}
									{@render triggerItem({ trigger, item })}
								{/each}
							{:else}
								<div class="text-xs text-gray-400 p-2">No {camelCaseToWords(type)} triggers</div>
							{/if}
						{/snippet}
					</Menu>
				{/if}
			</Tooltip>
		{/each}
		{#if extraTriggers.length > 0}
			<Menu
				{createMenu}
				usePointerDownOutside
				placement="bottom"
				menuClass={'w-56'}
				class="h-fit center-center mr-1"
				bind:open={menuOpen}
			>
				{#snippet trigger({ trigger })}
					<MeltButton
						class="w-[23px] h-[23px] rounded-md center-center text-[12px] hover:bg-slate-300 transition-all duration-100 font-normal text-secondary hover:text-primary"
						meltElement={trigger}
					>
						+{extraTriggers.length}
					</MeltButton>
				{/snippet}

				{#snippet children({ item })}
					{#if extraTriggers.length > 0}
						{#each extraTriggers as trigger}
							{@render triggerItem({ trigger, item })}
						{/each}
					{:else if extraTriggersType.length > 0}
						{#each extraTriggersType as type}
							{@render simpleTriggerItem({ item, type })}
						{/each}
					{/if}
				{/snippet}
			</Menu>
		{/if}
	{/snippet}
</Menubar>

{#snippet triggerButton({ type, isSelected, meltElement = undefined, singleItem = false })}
	{@const { icon: SvelteComponent, countKey } = triggerTypeConfig[type]}

	<MeltButton
		class={twMerge(
			'hover:bg-surface-hover rounded-md shadow-sm text-xs relative center-center cursor-pointer bg-slate-100 dark:bg-slate-700',
			'dark:outline dark:outline-1 outline-tertiary/20 group',
			isSelected ? 'outline-tertiary outline' : '',
			small ? 'w-[23px] h-[23px] outline-[1.5px]' : 'p-2 outline-[2px]'
		)}
		on:click={(e) => {
			e.stopPropagation()
			e.preventDefault()
			if (singleItem) {
				dispatch('select', triggersGrouped[type][0])
			}
		}}
		{meltElement}
	>
		{#if countKey}
			{@const count = $triggersCount?.[countKey] ?? 0}
			{#if count > 0}
				<div
					class={twMerge(
						// Base styles that apply in all cases
						'absolute z-10 rounded-full overflow-hidden',
						'flex center-center group-hover:text-primary-inverse font-mono text-transparent',
						'bg-tertiary group-hover:bg-primary transition-all duration-[100ms]',
						noTriggers ? 'bg-primary text-primary-inverse scale-100' : '',

						// Hover effects
						'group-hover:scale-110',

						// Size variants based on small prop
						small
							? '-right-[-2px] -top-[-2px] h-[4px] w-[4px] group-hover:-right-0.5 group-hover:-top-0.5 group-hover:h-3 group-hover:w-3 group-hover:text-[8px]'
							: '-right-[-2px] -top-[-2px] h-[6px] w-[6px] group-hover:-right-1 group-hover:-top-1 group-hover:h-4 group-hover:w-4 group-hover:text-xs',

						// Special case for no triggers
						noTriggers && small ? 'h-3 w-3 text-[8px] -right-0.5 -top-0.5' : '',
						noTriggers && !small ? 'h-4 w-4 text-xs -right-1 -top-1' : ''
					)}
				>
					{#if count === undefined}
						<Loader2 class="animate-spin text-2xs" />
					{:else}
						<span
							class={twMerge(
								'opacity-0 group-hover:opacity-100 transition-opacity duration-[50ms] delay-[100ms]',
								noTriggers ? 'opacity-100' : ''
							)}>{count}</span
						>
					{/if}
				</div>
			{/if}
		{/if}
		<SvelteComponent size={small ? 12 : 14} />
	</MeltButton>
{/snippet}

{#snippet triggerItem({ trigger, item })}
	<MenuItem
		{item}
		class={itemClass}
		on:click={(e) => {
			dispatch('select', trigger)
		}}
	>
		<TriggerLabel {trigger} />
	</MenuItem>
{/snippet}

{#snippet simpleTriggerItem({ item, type })}
	{@const { icon: SvelteComponent, countKey } = triggerTypeConfig[type]}
	<MenuItem {item} class={itemClass}>
		<div class="flex flex-row items-center gap-2">
			<SvelteComponent size={14} />
			{camelCaseToWords(type)}
			{#if countKey}
				<div class="text-xs text-gray-400">
					{$triggersCount?.[countKey] ?? 0}
				</div>
			{/if}
		</div>
	</MenuItem>
{/snippet}
