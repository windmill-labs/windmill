<script lang="ts">
	import Tab from '$lib/components/common/tabs/Tab.svelte'
	import { Tabs } from '$lib/components/common'
	import WebhooksPanel from '$lib/components/triggers/webhook/WebhooksPanel.svelte'
	import EmailTriggerPanel from '$lib/components/details/EmailTriggerPanel.svelte'
	import RoutesPanel from '$lib/components/triggers/http/RoutesPanel.svelte'
	import RunPageSchedules from '$lib/components/RunPageSchedules.svelte'
	import { canWrite } from '$lib/utils'
	import { userStore } from '$lib/stores'
	import FlowCard from '../flows/common/FlowCard.svelte'
	import { getContext, onDestroy, createEventDispatcher } from 'svelte'
	import type { TriggerContext } from '$lib/components/triggers'
	import ScheduledPollPanel from './scheduled/ScheduledPollPanel.svelte'
	import WebsocketTriggersPanel from './websocket/WebsocketTriggersPanel.svelte'
	import PostgresTriggersPanel from './postgres/PostgresTriggersPanel.svelte'
	import ToggleButtonGroup from '../common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '../common/toggleButton-v2/ToggleButton.svelte'
	import { KafkaIcon, MqttIcon, NatsIcon, AwsIcon, GoogleCloudIcon } from '../icons'
	import KafkaTriggersPanel from './kafka/KafkaTriggersPanel.svelte'
	import NatsTriggersPanel from './nats/NatsTriggersPanel.svelte'
	import MqttTriggersPanel from './mqtt/MqttTriggersPanel.svelte'
	import SqsTriggerPanel from './sqs/SqsTriggerPanel.svelte'
	import GcpTriggerPanel from './gcp/GcpTriggerPanel.svelte'

	export let noEditor: boolean
	export let newItem = false
	export let currentPath: string
	export let fakeInitialPath: string
	export let hash: string | undefined = undefined
	export let initialPath: string
	export let schema: any
	export let isFlow: boolean
	export let canHavePreprocessor: boolean = false
	export let hasPreprocessor: boolean = false
	export let args: Record<string, any> = {}
	let eventStreamType: 'kafka' | 'nats' | 'sqs' | 'mqtt' | 'gcp' = 'kafka'

	$: {
		if (
			$selectedTrigger === 'kafka' ||
			$selectedTrigger === 'nats' ||
			$selectedTrigger === 'sqs' ||
			$selectedTrigger === 'mqtt' ||
			$selectedTrigger === 'gcp'
		) {
			eventStreamType = $selectedTrigger
		}
	}

	const { selectedTrigger, simplifiedPoll } = getContext<TriggerContext>('TriggerContext')

	const dispatch = createEventDispatcher()
	onDestroy(() => {
		dispatch('exitTriggers')
	})
</script>

<FlowCard {noEditor} title="Triggers">
	{#if !$simplifiedPoll}
		<div class="h-full flex flex-col">
			<Tabs bind:selected={$selectedTrigger} wrapperClass="overflow-hidden shrink-0">
				<Tab value="webhooks" selectedClass="text-primary font-semibold">Webhooks</Tab>
				<Tab value="schedules" selectedClass="text-primary text-sm font-semibold">Schedules</Tab>
				<Tab value="routes" selectedClass="text-primary text-sm font-semibold">HTTP</Tab>
				<Tab value="websockets" selectedClass="text-primary text-sm font-semibold">WebSockets</Tab>
				<Tab value="postgres" selectedClass="text-primary text-sm font-semibold">Postgres</Tab>
				<Tab
					value="kafka"
					otherValues={['nats', 'sqs', 'mqtt', 'gcp']}
					selectedClass="text-primary text-sm font-semibold"
				>
					Event streams
				</Tab>
				<Tab value="emails" selectedClass="text-primary text-sm font-semibold">Email</Tab>
				{#if isFlow}
					<Tab value="scheduledPoll" selectedClass="text-primary text-sm font-semibold"
						>Scheduled Poll</Tab
					>
				{/if}

				<svelte:fragment slot="content">
					<div class="min-h-0 grow overflow-y-auto">
						{#if $selectedTrigger === 'webhooks'}
							<div class="p-4">
								<WebhooksPanel
									on:applyArgs
									on:addPreprocessor
									on:updateSchema
									on:testWithArgs
									scopes={isFlow ? [`run:flow/${currentPath}`] : [`run:script/${currentPath}`]}
									path={initialPath || fakeInitialPath}
									{hash}
									{isFlow}
									{args}
									token=""
									{newItem}
									isEditor={true}
									{canHavePreprocessor}
									{hasPreprocessor}
								/>
							</div>
						{:else if $selectedTrigger === 'emails'}
							<div class="p-4">
								<EmailTriggerPanel
									on:applyArgs
									on:addPreprocessor
									on:updateSchema
									on:testWithArgs
									token=""
									scopes={isFlow ? [`run:flow/${currentPath}`] : [`run:script/${currentPath}`]}
									path={initialPath || fakeInitialPath}
									{isFlow}
									isEditor={true}
									{canHavePreprocessor}
									{hasPreprocessor}
									{newItem}
								/>
							</div>
						{:else if $selectedTrigger === 'routes'}
							<div class="p-4">
								<RoutesPanel
									on:applyArgs
									on:addPreprocessor
									on:updateSchema
									on:testWithArgs
									{newItem}
									{args}
									path={initialPath || fakeInitialPath}
									{isFlow}
									isEditor={true}
									{canHavePreprocessor}
									{hasPreprocessor}
								/>
							</div>
						{:else if $selectedTrigger === 'websockets'}
							<div class="p-4">
								<WebsocketTriggersPanel
									on:applyArgs
									on:addPreprocessor
									on:updateSchema
									on:testWithArgs
									{newItem}
									path={initialPath || fakeInitialPath}
									{isFlow}
									isEditor={true}
									{canHavePreprocessor}
									{hasPreprocessor}
								/>
							</div>
						{:else if $selectedTrigger === 'postgres'}
							<div class="p-4">
								<PostgresTriggersPanel
									on:applyArgs
									on:addPreprocessor
									on:updateSchema
									on:testWithArgs
									{newItem}
									path={initialPath || fakeInitialPath}
									{isFlow}
									{canHavePreprocessor}
									{hasPreprocessor}
									isEditor={true}
								/>
							</div>
						{:else if $selectedTrigger === 'kafka' || $selectedTrigger === 'nats' || $selectedTrigger === 'sqs' || $selectedTrigger === 'mqtt' || $selectedTrigger === 'gcp'}
							<div class="p-4 flex flex-col gap-2">
								<ToggleButtonGroup bind:selected={eventStreamType} let:item>
									<ToggleButton value="kafka" label="Kafka" icon={KafkaIcon} {item} />
									<ToggleButton value="nats" label="NATS" icon={NatsIcon} {item} />
									<ToggleButton value="mqtt" label="MQTT" icon={MqttIcon} {item} />
									<ToggleButton value="sqs" label="SQS" icon={AwsIcon} {item} />
									<ToggleButton value="gcp" label="GCP" icon={GoogleCloudIcon} {item} />
								</ToggleButtonGroup>
								{#if eventStreamType === 'kafka'}
									<KafkaTriggersPanel
										on:applyArgs
										on:addPreprocessor
										on:updateSchema
										on:testWithArgs
										{newItem}
										path={initialPath || fakeInitialPath}
										{isFlow}
										isEditor={true}
										{canHavePreprocessor}
										{hasPreprocessor}
									/>
								{:else if eventStreamType === 'nats'}
									<NatsTriggersPanel
										on:applyArgs
										on:addPreprocessor
										{newItem}
										path={initialPath || fakeInitialPath}
										{isFlow}
										isEditor={true}
										{canHavePreprocessor}
										{hasPreprocessor}
									/>
								{:else if eventStreamType === 'sqs'}
									<SqsTriggerPanel
										on:applyArgs
										on:addPreprocessor
										on:updateSchema
										on:testWithArgs
										{newItem}
										path={initialPath || fakeInitialPath}
										{isFlow}
										isEditor={true}
										{canHavePreprocessor}
										{hasPreprocessor}
									/>
								{:else if eventStreamType === 'mqtt'}
									<MqttTriggersPanel
										on:applyArgs
										on:addPreprocessor
										on:updateSchema
										on:testWithArgs
										{newItem}
										path={initialPath || fakeInitialPath}
										{isFlow}
										isEditor={true}
										{canHavePreprocessor}
										{hasPreprocessor}
									/>
								{:else if eventStreamType === 'gcp'}
									<GcpTriggerPanel
										on:applyArgs
										on:addPreprocessor
										on:updateSchema
										on:testWithArgs
										{newItem}
										path={initialPath || fakeInitialPath}
										{isFlow}
										isEditor={true}
										{canHavePreprocessor}
										{hasPreprocessor}
									/>
								{/if}
							</div>
						{:else if $selectedTrigger === 'schedules'}
							<div class="p-4">
								<RunPageSchedules
									{schema}
									{isFlow}
									path={initialPath}
									{newItem}
									can_write={canWrite(currentPath, {}, $userStore)}
								/>
							</div>
						{:else if $selectedTrigger === 'scheduledPoll'}
							<div class="p-4">
								<ScheduledPollPanel />
							</div>
						{/if}
					</div>
				</svelte:fragment>
			</Tabs>
		</div>
	{:else}
		<div class="px-4 pb-2">
			<RunPageSchedules
				{schema}
				{isFlow}
				path={initialPath}
				{newItem}
				can_write={canWrite(currentPath, {}, $userStore)}
			/>
		</div>
	{/if}
</FlowCard>
