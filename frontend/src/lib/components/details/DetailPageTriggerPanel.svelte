<script lang="ts">
	import { Tabs, Tab } from '$lib/components/common'
	import { CalendarCheck2, MailIcon, Route, Terminal, Webhook } from 'lucide-svelte'

	import HighlightTheme from '../HighlightTheme.svelte'

	export let triggerSelected: 'webhooks' | 'emails' | 'schedules' | 'cli' | 'routes' = 'webhooks'
</script>

<HighlightTheme />

<Tabs bind:selected={triggerSelected}>
	<Tab value="webhooks">
		<span class="flex flex-row gap-2 items-center">
			<Webhook size={14} />
			Webhooks
		</span>
	</Tab>
	<Tab value="schedules">
		<span class="flex flex-row gap-2 items-center">
			<CalendarCheck2 size={14} />
			Schedules
		</span>
	</Tab>
	<Tab value="routes">
		<span class="flex flex-row gap-2 items-center">
			<Route size={14} />
			HTTP
		</span>
	</Tab>
	<Tab value="emails">
		<span class="flex flex-row gap-2 items-center">
			<MailIcon size={14} />
			Email
		</span>
	</Tab>
	<Tab value="cli">
		<span class="flex flex-row gap-2 items-center">
			<Terminal size={14} />
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
		{:else if triggerSelected === 'cli'}
			<slot name="cli" />
		{/if}
	</div>
</div>
