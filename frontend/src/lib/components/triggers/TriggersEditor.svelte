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
	import { getContext } from 'svelte'
	import type { TriggerContext } from '$lib/components/triggers'
	import WebsocketTriggersPanel from './WebsocketTriggersPanel.svelte'
	import ScheduledPollPanel from './ScheduledPollPanel.svelte'

	export let noEditor: boolean
	export let newItem = false
	export let currentPath: string
	export let initialPath: string
	export let schema: any
	export let isFlow: boolean

	const { selectedTrigger } = getContext<TriggerContext>('TriggerContext')
</script>

<FlowCard {noEditor} title="Triggers">
	<div class="pt-4">
		<Tabs bind:selected={$selectedTrigger}>
			<Tab value="webhooks" selectedClass="text-primary font-semibold">Webhooks</Tab>
			<Tab value="schedules" selectedClass="text-primary text-sm font-semibold">Schedules</Tab>
			<Tab value="routes" selectedClass="text-primary text-sm font-semibold">Routes</Tab>
			<Tab value="websockets" selectedClass="text-primary text-sm font-semibold">Websockets</Tab>
			<Tab value="emails" selectedClass="text-primary text-sm font-semibold">Email</Tab>
			<Tab value="scheduledPoll" selectedClass="text-primary text-sm font-semibold"
				>Scheduled Poll</Tab
			>

			<svelte:fragment slot="content">
				{#if $selectedTrigger === 'webhooks'}
					<div class="p-4">
						<WebhooksPanel
							scopes={isFlow ? [`run:flow/${currentPath}`] : [`run:script/${currentPath}`]}
							path={currentPath}
							{isFlow}
							args={{}}
							token=""
							{newItem}
						/>
					</div>
				{/if}

				{#if $selectedTrigger === 'emails'}
					<div class="p-4">
						<EmailTriggerPanel
							token=""
							scopes={isFlow ? [`run:flow/${currentPath}`] : [`run:script/${currentPath}`]}
							path={currentPath}
							{isFlow}
						/>
					</div>
				{/if}

				{#if $selectedTrigger === 'routes'}
					<div class="p-4">
						<RoutesPanel {newItem} path={currentPath} {isFlow} />
					</div>
				{/if}

				{#if $selectedTrigger === 'websockets'}
					<div class="p-4">
						<WebsocketTriggersPanel {newItem} path={currentPath} {isFlow} />
					</div>
				{/if}

				{#if $selectedTrigger === 'schedules'}
					<div class="p-4">
						<RunPageSchedules
							{schema}
							{isFlow}
							path={initialPath}
							{newItem}
							can_write={canWrite(currentPath, {}, $userStore)}
						/>
					</div>
				{/if}

				{#if $selectedTrigger === 'scheduledPoll'}
					<div class="p-4">
						<ScheduledPollPanel />
					</div>
				{/if}
			</svelte:fragment>
		</Tabs>
	</div>
</FlowCard>
