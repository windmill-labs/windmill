<script lang="ts">
	import { Calendar, Mail, Webhook, Unplug, Database, Terminal } from 'lucide-svelte'
	import { type Component, type ComponentType } from 'svelte'
	import { Route } from 'lucide-svelte'
	import { getContext } from 'svelte'
	import { type TriggerContext } from '$lib/components/triggers'
	import { enterpriseLicense } from '$lib/stores'
	import { MqttIcon, NatsIcon, KafkaIcon, AwsIcon, GoogleCloudIcon } from '$lib/components/icons'
	import { type Trigger, type TriggerType } from '$lib/components/triggers/utils'
	import { Menu, Menubar, MeltButton, MenuItem, Tooltip } from '$lib/components/meltComponents'
	import { twMerge } from 'tailwind-merge'
	import SchedulePollIcon from '$lib/components/icons/SchedulePollIcon.svelte'
	import type { NativeServiceName } from '$lib/gen/types.gen'
	import TriggerLabel from '$lib/components/triggers/TriggerLabel.svelte'
	import CountBadge from '$lib/components/common/badge/CountBadge.svelte'
	import NextcloudIcon from '$lib/components/icons/NextcloudIcon.svelte'
	import GoogleIcon from '$lib/components/icons/GoogleIcon.svelte'

	const { triggersState, triggersCount } = getContext<TriggerContext>('TriggerContext')

	interface Props {
		path: string
		newItem: boolean
		isFlow: boolean
		selected: boolean
		showOnlyWithCount: boolean
		numberOfTriggers?: number
		small?: boolean
		vertical?: boolean
		limit?: number
		showDraft?: boolean
		onSelect?: (triggerIndex: number) => void
	}

	let {
		selected,
		showOnlyWithCount,
		// @ts-ignore - This is an output-only prop used with bind:
		numberOfTriggers = $bindable(0),
		small = true,
		vertical = false,
		limit,
		showDraft = true,
		onSelect
	}: Props = $props()

	let menuOpen = $state(false)
	let availableNativeServices = $state<
		Array<{ service: NativeServiceName; icon: any; config: any }>
	>([])

	let triggerTypeConfig = $derived(() => {
		const baseConfig: {
			[key in TriggerType]?: {
				icon: ComponentType | Component<any, {}, ''>
				countKey?: string
				disabled?: boolean
			}
		} = {
			webhook: { icon: Webhook, countKey: 'webhook_count' },
			schedule: { icon: Calendar, countKey: 'schedule_count' },
			http: { icon: Route, countKey: 'http_routes_count' },
			websocket: { icon: Unplug, countKey: 'websocket_count' },
			postgres: { icon: Database, countKey: 'postgres_count' },
			kafka: { icon: KafkaIcon, countKey: 'kafka_count', disabled: !$enterpriseLicense },
			default_email: { icon: Mail, countKey: 'default_email_count' },
			email: { icon: Mail, countKey: 'email_count' },
			nats: { icon: NatsIcon, countKey: 'nats_count', disabled: !$enterpriseLicense },
			mqtt: { icon: MqttIcon, countKey: 'mqtt_count', disabled: !$enterpriseLicense },
			sqs: { icon: AwsIcon, countKey: 'sqs_count', disabled: !$enterpriseLicense },
			gcp: { icon: GoogleCloudIcon, countKey: 'gcp_count', disabled: !$enterpriseLicense },
			poll: { icon: SchedulePollIcon },
			cli: { icon: Terminal },
			nextcloud: { icon: NextcloudIcon, countKey: 'nextcloud_count' },
			google: { icon: GoogleIcon, countKey: 'google_count' }
		}

		// Add native trigger services that are available
		for (const { service, icon } of availableNativeServices) {
			baseConfig[service as TriggerType] = { icon }
		}

		return baseConfig as {
			[key in TriggerType]: {
				icon: ComponentType | Component<any, {}, ''>
				countKey?: string
				disabled?: boolean
			}
		}
	})

	let allTypes = $derived([
		'webhook',
		'schedule',
		'http',
		'websocket',
		'postgres',
		'kafka',
		'default_email',
		'nats',
		'mqtt',
		'sqs',
		'gcp',
		'email',
		'poll',
		'cli',
		'nextcloud',
		'google'
	])

	function camelCaseToWords(s: string) {
		const result = s.replace(/([A-Z])/g, ' $1')
		return result.charAt(0).toUpperCase() + result.slice(1).toLowerCase()
	}

	const itemClass = twMerge(
		'text-secondary text-left font-normal w-full block px-4 py-2 text-2xs data-[highlighted]:bg-surface-hover data-[highlighted]:text-primary flex flex-row items-center flex-nowrap'
	)

	// Group triggers by their mapped type
	let triggersGrouped = $derived.by(() => {
		const triggersWithIndex = triggersState.triggers.map((trigger, index) => ({
			...trigger,
			index
		}))
		const triggersFiltered = showDraft
			? triggersWithIndex
			: triggersWithIndex.filter((trigger) => !trigger.isDraft)
		return triggersFiltered.reduce(
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
	})

	const noTriggers = $derived(triggersState.triggers.length === 0)

	// Extract unique trigger types for display, only keep the first
	let allTriggerTypes = $derived.by(() => {
		const types = !noTriggers ? (Object.keys(triggersGrouped) as TriggerType[]) : allTypes
		//filter out types if showOnlyTriggersWithCount is true and there are no triggers for that type
		return types.filter(
			(type) =>
				(!showOnlyTriggersWithCount ||
					((triggerTypeConfig()[type]?.countKey &&
						($triggersCount?.[triggerTypeConfig()[type].countKey] ?? 0)) ||
						0) > 0) &&
				!triggerTypeConfig()[type]?.disabled
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
	let showOnlyTriggersWithCount = $derived(showOnlyWithCount || triggersState.triggers.length === 0)

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
			{@const isSelected =
				selected && triggersState.selectedTrigger && triggersState.selectedTrigger?.type === type}
			{@const singleItem =
				type === 'webhook' ||
				type === 'email' ||
				type === 'cli' ||
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
						{#snippet triggr({ trigger })}
							{@render triggerButton({
								type,
								isSelected,
								meltElement: trigger
							})}
						{/snippet}

						{#snippet children({ item })}
							{#if triggersGrouped[type] && triggersGrouped[type].length > 0}
								{#each triggersGrouped[type] as trigger}
									{@render triggerItem({ triggerIndex: trigger.index, item })}
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
				{#snippet triggr({ trigger })}
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
							{@render triggerItem({ triggerIndex: trigger.index, item })}
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
	{@const { icon: SvelteComponent, countKey } = triggerTypeConfig()[type] || {
		icon: Database,
		countKey: undefined
	}}

	<MeltButton
		aiId={`trigger-button-${type}`}
		aiDescription={`Trigger button for ${type}`}
		class={twMerge(
			'hover:bg-surface-hover rounded-md text-xs relative center-center cursor-pointer border border-border-light',
			'group',
			isSelected ? 'bg-surface-accent-selected text-accent border-border-selected' : '',
			small ? 'w-[23px] h-[23px]' : 'p-2'
		)}
		on:click={(e) => {
			e.stopPropagation()
			e.preventDefault()
			if (singleItem) {
				onSelect?.(triggersGrouped[type][0].index)
			}
		}}
		{meltElement}
	>
		{#if countKey}
			{@const count = $triggersCount?.[countKey] ?? 0}
			<CountBadge {count} {small} alwaysVisible={noTriggers} />
		{/if}
		<SvelteComponent size={small ? 12 : 14} />
	</MeltButton>
{/snippet}

{#snippet triggerItem({ triggerIndex, item })}
	<MenuItem
		{item}
		class={itemClass}
		onClick={() => {
			onSelect?.(triggerIndex)
		}}
	>
		<TriggerLabel trigger={triggersState.triggers[triggerIndex]} />
	</MenuItem>
{/snippet}

{#snippet simpleTriggerItem({ item, type })}
	{@const { icon: SvelteComponent, countKey } = triggerTypeConfig()[type] || {
		icon: Database,
		countKey: undefined
	}}
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
