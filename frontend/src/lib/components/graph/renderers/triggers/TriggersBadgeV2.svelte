<script lang="ts">
	import { Calendar, Mail, Webhook, Unplug, Database } from 'lucide-svelte'
	import { Loader2 } from 'lucide-svelte'
	import { createEventDispatcher, onMount, type ComponentType } from 'svelte'
	import { Route } from 'lucide-svelte'
	import { getContext } from 'svelte'
	import { type TriggerContext } from '$lib/components/triggers'
	import { FlowService, ScriptService } from '$lib/gen'
	import { enterpriseLicense, workspaceStore } from '$lib/stores'
	import { MqttIcon, NatsIcon, KafkaIcon, AwsIcon, GoogleCloudIcon } from '$lib/components/icons'
	import type { Trigger, TriggerType } from '$lib/components/triggers/utils'
	import { Menu, Menubar, MeltButton, MenuItem, Tooltip } from '$lib/components/meltComponents'
	import { twMerge } from 'tailwind-merge'
	import SchedulePollIcon from '$lib/components/icons/SchedulePollIcon.svelte'

	const { selectedTriggerV2, triggersCount } = getContext<TriggerContext>('TriggerContext')

	interface Props {
		path: string
		newItem: boolean
		isFlow: boolean
		selected: boolean
		showOnlyWithCount: boolean
		triggers: Trigger[]
		numberOfTriggers?: number
		small?: boolean
		allwaysUseDropdown?: boolean
	}

	let {
		path,
		newItem,
		isFlow,
		selected,
		showOnlyWithCount,
		triggers,
		numberOfTriggers = $bindable(),
		small = true,
		allwaysUseDropdown = false
	}: Props = $props()

	let menuOpen = $state(false)

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
		poll: { icon: SchedulePollIcon }
	}

	function camelCaseToWords(s: string) {
		const result = s.replace(/([A-Z])/g, ' $1')
		return result.charAt(0).toUpperCase() + result.slice(1).toLowerCase()
	}

	const itemClass = twMerge(
		'text-secondary text-left font-normal w-full block px-4 py-2 text-2xs data-[highlighted]:bg-surface-hover data-[highlighted]:text-primary'
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

	// Extract unique trigger types for display, only keep the first 5
	let allTriggerTypes = $derived(Object.keys(triggersGrouped) as TriggerType[])
	let triggersToDisplay = $derived(allTriggerTypes.slice(0, 7))
	let extraTriggers = $derived(
		allTriggerTypes.length > 7
			? allTriggerTypes.slice(8).flatMap((type) => triggersGrouped[type])
			: []
	)

	$effect(() => {
		if (allTriggerTypes) {
			numberOfTriggers = allTriggerTypes?.length
		}
	})
</script>

<Menubar class={twMerge('flex flex-row gap-1 items-center', small ? '' : 'gap-2')}>
	{#snippet children({ createMenu })}
		{#each triggersToDisplay as type}
			{@const isSelected = selected && $selectedTriggerV2 && $selectedTriggerV2.type === type}
			{@const singleItem =
				type === 'webhook' ||
				type === 'email'! ||
				(!allwaysUseDropdown && triggersGrouped[type] && triggersGrouped[type].length === 1)}
			<Tooltip disablePopup={menuOpen} on:click={(e) => e.stopPropagation()}>
				{#snippet text()}
					{camelCaseToWords(type)}
				{/snippet}
				{#if singleItem}
					{@render triggerButton({ type, isSelected, singleItem })}
				{:else}
					<Menu
						{createMenu}
						usePointerDownOutside
						placement="bottom"
						menuClass={'min-w-56 w-fit'}
						class="h-fit"
						bind:open={menuOpen}
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
				menuClass={'min-w-56 w-fit'}
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
					{#each extraTriggers as trigger}
						{@render triggerItem({ trigger, item })}
					{/each}
				{/snippet}
			</Menu>
		{/if}
	{/snippet}
</Menubar>

{#snippet triggerButton({ type, isSelected, meltElement = undefined, singleItem = false })}
	{@const { icon: SvelteComponent, countKey } = triggerTypeConfig[type]}
	{#if (!showOnlyWithCount || ((countKey && ($triggersCount?.[countKey] ?? 0)) || 0) > 0) && !triggerTypeConfig[type].disabled}
		<MeltButton
			class={twMerge(
				'hover:bg-surface-hover rounded-md shadow-sm text-xs relative center-center cursor-pointer bg-surface',
				'dark:outline outline-1 outline-offset-[-1px] outline-tertiary/20 group',
				isSelected ? 'outline-tertiary outline' : '',
				small ? 'w-[23px] h-[23px]' : 'p-2'
			)}
			on:click={() => {
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
							'absolute -right-1 -top-1 z-10 bg-surface-secondary-inverse bg-opacity-40 group-hover:bg-opacity-80 transition-all duration-100',
							'rounded-sm shadow-lg flex center-center text-primary-inverse font-mono',
							small ? 'h-3 w-3 text-[8px]' : 'h-4 w-4 text-xs'
						)}
					>
						{#if count === undefined}
							<Loader2 class="animate-spin text-2xs" />
						{:else}
							{count}
						{/if}
					</div>
				{/if}
			{/if}
			<SvelteComponent size={small ? 12 : 14} />
		</MeltButton>
	{/if}
{/snippet}

{#snippet triggerItem({ trigger, item })}
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
{/snippet}
