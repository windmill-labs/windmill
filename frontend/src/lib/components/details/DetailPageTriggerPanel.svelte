<script lang="ts">
	import { Tabs, Tab } from '$lib/components/common'
	import {
		CalendarCheck2,
		MailIcon,
		Route,
		Terminal,
		Webhook,
		Unplug,
		PlugZap,

		Database

	} from 'lucide-svelte'

	import HighlightTheme from '../HighlightTheme.svelte'
	import ToggleButtonGroup from '../common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '../common/toggleButton-v2/ToggleButton.svelte'
	import { MqttIcon, NatsIcon, KafkaIcon } from '../icons'

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
		| 'mqtt'
		| 'scheduledPoll' = 'webhooks'
	export let simplfiedPoll: boolean = false

	export let eventStreamType: 'kafka' | 'nats' | 'mqtt' = 'kafka'

	$: {
		if (triggerSelected === 'kafka' || triggerSelected === 'nats' || triggerSelected === 'mqtt') {
			eventStreamType = triggerSelected
		}
	}
</script>

<HighlightTheme />

{#if !simplfiedPoll}
	<div class="flex flex-col h-full">
		<Tabs bind:selected={triggerSelected} wrapperClass="flex-none w-full">
			<Tab value="webhooks">
				<span class="flex flex-row gap-2 items-center text-xs">
					<Webhook size={12} />
					Webhooks
				</span>
			</Tab>
			<Tab value="schedules">
				<span class="flex flex-row gap-2 items-center text-xs">
					<CalendarCheck2 size={12} />
					Schedules
				</span>
			</Tab>
			<Tab value="routes">
				<span class="flex flex-row gap-2 items-center text-xs">
					<Route size={12} />
					HTTP
				</span>
			</Tab>
			<Tab value="websockets">
				<span class="flex flex-row gap-2 items-center text-xs">
					<Unplug size={12} />
					WebSockets
				</span>
			</Tab>
			<Tab value="postgres">
				<span class="flex flex-row gap-2 items-center text-xs">
					<Database size={12} />
					Postgres
				</span>
			</Tab>
			<Tab value="kafka" otherValues={['nats', 'mqtt']}>
				<span class="flex flex-row gap-2 items-center text-xs">
					<PlugZap size={12} />
					Event streams
				</span>
			</Tab>
			<Tab value="emails">
				<span class="flex flex-row gap-2 items-center text-xs">
					<MailIcon size={12} />
					Email
				</span>
			</Tab>
			<Tab value="cli">
				<span class="flex flex-row gap-2 items-center text-xs">
					<Terminal size={12} />
					CLI
				</span>
			</Tab>

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
					{:else if triggerSelected === 'kafka' || triggerSelected === 'nats' || triggerSelected === 'mqtt'}
						<div class="m-1.5">
							<ToggleButtonGroup bind:selected={eventStreamType}>
								<ToggleButton value="kafka" label="Kafka" icon={KafkaIcon} />
								<ToggleButton value="nats" label="NATS" icon={NatsIcon} />
								<ToggleButton value="mqtt" label="MQTT" icon={MqttIcon} />
							</ToggleButtonGroup>
						</div>
						{#if eventStreamType === 'kafka'}
							<slot name="kafka" />
						{:else if eventStreamType === 'nats'}
							<slot name="nats" />
						{:else if eventStreamType === 'mqtt'}
							<slot name="mqtt" />
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
