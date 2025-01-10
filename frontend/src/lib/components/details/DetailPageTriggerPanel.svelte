<script lang="ts">
	import { Tabs, Tab } from '$lib/components/common'
	import { CalendarCheck2, MailIcon, Route, Terminal, Webhook, Unplug } from 'lucide-svelte'

	import HighlightTheme from '../HighlightTheme.svelte'
	import KafkaIcon from '../icons/KafkaIcon.svelte'

	export let triggerSelected:
		| 'webhooks'
		| 'emails'
		| 'schedules'
		| 'cli'
		| 'routes'
		| 'websockets'
		| 'kafka'
		| 'scheduledPoll' = 'webhooks'
	export let simplfiedPoll: boolean = false
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
					Websockets
				</span>
			</Tab>
			<Tab value="kafka">
				<span class="flex flex-row gap-2 items-center text-xs">
					<KafkaIcon size={12} />
					Kafka
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
					{:else if triggerSelected === 'kafka'}
						<slot name="kafka" />
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
