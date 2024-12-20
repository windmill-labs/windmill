<script lang="ts">
	import Tab from '$lib/components/common/tabs/Tab.svelte'
	import { Tabs } from '$lib/components/common'
	import WebhooksPanel from '$lib/components/triggers/WebhooksPanel.svelte'
	import EmailTriggerPanel from '$lib/components/details/EmailTriggerPanel.svelte'
	import RoutesPanel from '$lib/components/triggers/RoutesPanel.svelte'
	import RunPageSchedules from '$lib/components/RunPageSchedules.svelte'
	import { canWrite } from '$lib/utils'
	import { userStore } from '$lib/stores'
	import FlowCard from '../flows/common/FlowCard.svelte'
	import { getContext, onDestroy, createEventDispatcher } from 'svelte'
	import type { TriggerContext } from '$lib/components/triggers'
	import WebsocketTriggersPanel from './WebsocketTriggersPanel.svelte'
	import ScheduledPollPanel from './ScheduledPollPanel.svelte'
	import KafkaTriggersPanel from './KafkaTriggersPanel.svelte'

	export let noEditor: boolean
	export let newItem = false
	export let currentPath: string
	export let hash: string | undefined = undefined
	export let initialPath: string
	export let schema: any
	export let isFlow: boolean
	export let canHavePreprocessor: boolean = false
	export let hasPreprocessor: boolean = false

	const { selectedTrigger, simplifiedPoll } = getContext<TriggerContext>('TriggerContext')

	const dispatch = createEventDispatcher()

	onDestroy(() => {
		dispatch('exitTriggers')
	})
</script>

<FlowCard {noEditor} title="Triggers">
	{#if !$simplifiedPoll}
		<Tabs bind:selected={$selectedTrigger}>
			<Tab value="webhooks" selectedClass="text-primary font-semibold">Webhooks</Tab>
			<Tab value="schedules" selectedClass="text-primary text-sm font-semibold">Schedules</Tab>
			<Tab value="routes" selectedClass="text-primary text-sm font-semibold">HTTP</Tab>
			<Tab value="websockets" selectedClass="text-primary text-sm font-semibold">Websockets</Tab>
			<Tab value="kafka" selectedClass="text-primary text-sm font-semibold">Kafka</Tab>
			<Tab value="emails" selectedClass="text-primary text-sm font-semibold">Email</Tab>
			{#if isFlow}
				<Tab value="scheduledPoll" selectedClass="text-primary text-sm font-semibold"
					>Scheduled Poll</Tab
				>
			{/if}

			<svelte:fragment slot="content">
				{#if $selectedTrigger === 'webhooks'}
					<div class="p-4">
						<WebhooksPanel
							on:applyArgs
							on:addPreprocessor
							scopes={isFlow ? [`run:flow/${currentPath}`] : [`run:script/${currentPath}`]}
							path={currentPath}
							{hash}
							{isFlow}
							args={{}}
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
							token=""
							scopes={isFlow ? [`run:flow/${currentPath}`] : [`run:script/${currentPath}`]}
							path={currentPath}
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
							{newItem}
							path={currentPath}
							{isFlow}
							isEditor={true}
						/>
					</div>
				{:else if $selectedTrigger === 'websockets'}
					<div class="p-4">
						<WebsocketTriggersPanel
							on:applyArgs
							on:addPreprocessor
							{newItem}
							path={currentPath}
							{isFlow}
							isEditor={true}
							{canHavePreprocessor}
							{hasPreprocessor}
						/>
					</div>
				{:else if $selectedTrigger === 'kafka'}
					<div class="p-4">
						<KafkaTriggersPanel
							on:applyArgs
							on:addPreprocessor
							{newItem}
							path={currentPath}
							{isFlow}
							isEditor={true}
							{canHavePreprocessor}
							{hasPreprocessor}
						/>
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
			</svelte:fragment>
		</Tabs>
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
