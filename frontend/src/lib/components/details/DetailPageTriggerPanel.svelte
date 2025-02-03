<script lang="ts">
	import { Tabs, TabFlex } from '$lib/components/common'
	import {
		CalendarCheck2,
		MailIcon,
		Route,
		Terminal,
		Webhook,
		Unplug,
		PlugZap
	} from 'lucide-svelte'

	import HighlightTheme from '../HighlightTheme.svelte'
	import KafkaIcon from '../icons/KafkaIcon.svelte'
	import NatsIcon from '../icons/NatsIcon.svelte'
	import ToggleButtonGroup from '../common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '../common/toggleButton-v2/ToggleButton.svelte'

	export let triggerSelected:
		| 'webhooks'
		| 'emails'
		| 'schedules'
		| 'cli'
		| 'routes'
		| 'websockets'
		| 'kafka'
		| 'postgres'
		| 'nats'
		| 'scheduledPoll' = 'webhooks'
	export let simplfiedPoll: boolean = false

	export let eventStreamType: 'kafka' | 'nats' = 'kafka'

	const tabs = [
		{ value: 'webhooks', label: 'Webhooks', icon: Webhook, otherValues: [] },
		{ value: 'schedules', label: 'Schedules', icon: CalendarCheck2, otherValues: [] },
		{ value: 'routes', label: 'HTTP', icon: Route, otherValues: [] },
		{ value: 'websockets', label: 'WebSockets', icon: Unplug, otherValues: [] },
		{ value: 'postgres', label: 'Postgres', icon: Unplug, otherValues: [] },
		{ value: 'kafka', label: 'Event streams', icon: PlugZap, otherValues: ['nats'] },
		{ value: 'emails', label: 'Email', icon: MailIcon, otherValues: [] },
		{ value: 'cli', label: 'CLI', icon: Terminal, otherValues: [] }
	]

	$: {
		if (triggerSelected === 'kafka' || triggerSelected === 'nats') {
			eventStreamType = triggerSelected
		}
	}
</script>

<HighlightTheme />

{#if !simplfiedPoll}
	<div class="flex flex-col h-full">
		<Tabs bind:selected={triggerSelected} wrapperClass="flex-none w-full">
			{#each tabs as tab}
				<TabFlex
					value={tab.value}
					otherValues={tab.otherValues}
					icon={tab.icon}
					label={tab.label}
					selected={triggerSelected === tab.value}
				/>
			{/each}

			<svelte:fragment slot="content">
				<div class="min-h-0 grow overflow-y-auto">
					{#if triggerSelected === 'webhooks'}
						<slot name="webhooks" />
					{:else if triggerSelected === 'routes'}
						<slot name="routes" />
					{:else if triggerSelected === 'emails'}
						<slot name="emails" />
					{:else if triggerSelected === 'schedules'}
						<slot name="schedules" />
					{:else if triggerSelected === 'websockets'}
						<slot name="websockets" />
					{:else if triggerSelected === 'postgres'}
						<slot name="postgres" />
					{:else if triggerSelected === 'kafka' || triggerSelected === 'nats'}
						<div class="m-1.5">
							<ToggleButtonGroup bind:selected={eventStreamType}>
								<ToggleButton value="kafka" label="Kafka" icon={KafkaIcon} />
								<ToggleButton value="nats" label="NATS" icon={NatsIcon} />
							</ToggleButtonGroup>
						</div>
						{#if eventStreamType === 'kafka'}
							<slot name="kafka" />
						{:else if eventStreamType === 'nats'}
							<slot name="nats" />
						{/if}
					{:else if triggerSelected === 'cli'}
						<slot name="cli" />
					{/if}
				</div>
			</svelte:fragment>
		</Tabs>
	</div>
{:else}
	<slot name="schedules" />
{/if}
