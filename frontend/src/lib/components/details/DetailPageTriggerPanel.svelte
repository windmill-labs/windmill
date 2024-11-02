<script lang="ts">
	import { Tabs, Tab } from '$lib/components/common'
	import { CalendarCheck2, MailIcon, Route, Terminal, Webhook, Unplug } from 'lucide-svelte'

	import HighlightTheme from '../HighlightTheme.svelte'

	export let triggerSelected:
		| 'webhooks'
		| 'emails'
		| 'schedules'
		| 'cli'
		| 'routes'
		| 'websockets' = 'webhooks'
</script>

<HighlightTheme />

<Tabs bind:selected={triggerSelected}>
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
</Tabs>

<div class="h-[calc(100%-32px)]">
	<div class="h-full overflow-auto">
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
		{:else if triggerSelected === 'cli'}
			<slot name="cli" />
		{/if}
	</div>
</div>
