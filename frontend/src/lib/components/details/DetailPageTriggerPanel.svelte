<script lang="ts">
	import { run } from 'svelte/legacy'

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
	import { MqttIcon, NatsIcon, KafkaIcon, AwsIcon } from '../icons'
	import GoogleCloudIcon from '../icons/GoogleCloudIcon.svelte'

	interface Props {
		triggerSelected?:
			| 'webhooks'
			| 'emails'
			| 'schedules'
			| 'cli'
			| 'routes'
			| 'websockets'
			| 'kafka'
			| 'postgres'
			| 'nats'
			| 'sqs'
			| 'mqtt'
			| 'gcp'
			| 'scheduledPoll'
		simplfiedPoll?: boolean
		eventStreamType?: 'kafka' | 'nats' | 'sqs' | 'mqtt' | 'gcp'
		webhooks?: import('svelte').Snippet
		routes?: import('svelte').Snippet
		emails?: import('svelte').Snippet
		schedules?: import('svelte').Snippet
		websockets?: import('svelte').Snippet
		postgres?: import('svelte').Snippet
		kafka?: import('svelte').Snippet
		nats?: import('svelte').Snippet
		sqs?: import('svelte').Snippet
		mqtt?: import('svelte').Snippet
		gcp?: import('svelte').Snippet
		cli?: import('svelte').Snippet
	}

	let {
		triggerSelected = $bindable('webhooks'),
		simplfiedPoll = false,
		eventStreamType = $bindable('kafka'),
		webhooks,
		routes,
		emails,
		schedules,
		websockets,
		postgres,
		kafka,
		nats,
		sqs,
		mqtt,
		gcp,
		cli
	}: Props = $props()

	run(() => {
		if (
			triggerSelected === 'kafka' ||
			triggerSelected === 'nats' ||
			triggerSelected === 'sqs' ||
			triggerSelected === 'mqtt' ||
			triggerSelected === 'gcp'
		) {
			eventStreamType = triggerSelected
		}
	})
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
			<Tab value="kafka" otherValues={['nats', 'sqs', 'mqtt', 'gcp']}>
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

			{#snippet content()}
				<div class="min-h-0 grow overflow-y-auto">
					{#if triggerSelected === 'webhooks'}
						{@render webhooks?.()}
					{:else if triggerSelected === 'routes'}
						{@render routes?.()}
					{:else if triggerSelected === 'emails'}
						{@render emails?.()}
					{:else if triggerSelected === 'schedules'}
						{@render schedules?.()}
					{:else if triggerSelected === 'websockets'}
						{@render websockets?.()}
					{:else if triggerSelected === 'postgres'}
						{@render postgres?.()}
					{:else if triggerSelected === 'kafka' || triggerSelected === 'nats' || triggerSelected === 'sqs' || triggerSelected === 'mqtt' || triggerSelected === 'gcp'}
						<div class="m-1.5">
							<ToggleButtonGroup bind:selected={eventStreamType}>
								{#snippet children({ item })}
									<ToggleButton value="kafka" label="Kafka" icon={KafkaIcon} {item} />
									<ToggleButton value="nats" label="NATS" icon={NatsIcon} {item} />
									<ToggleButton value="mqtt" label="MQTT" icon={MqttIcon} {item} />
									<ToggleButton value="sqs" label="SQS" icon={AwsIcon} {item} />
									<ToggleButton value="gcp" label="GCP Pub/Sub" icon={GoogleCloudIcon} {item} />
								{/snippet}
							</ToggleButtonGroup>
						</div>
						{#if eventStreamType === 'kafka'}
							{@render kafka?.()}
						{:else if eventStreamType === 'nats'}
							{@render nats?.()}
						{:else if eventStreamType === 'sqs'}
							{@render sqs?.()}
						{:else if eventStreamType === 'mqtt'}
							{@render mqtt?.()}
						{:else if eventStreamType === 'gcp'}
							{@render gcp?.()}
						{/if}
					{:else if triggerSelected === 'cli'}
						{@render cli?.()}
					{/if}
				</div>
			{/snippet}
		</Tabs>
	</div>
{:else}
	{@render schedules?.()}
{/if}
