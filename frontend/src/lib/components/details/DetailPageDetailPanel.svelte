<script lang="ts">
	import { Tabs, Tab, TabContent } from '$lib/components/common'

	import HighlightTheme from '../HighlightTheme.svelte'
	import FlowViewerInner from '../FlowViewerInner.svelte'
	import DetailPageTriggerPanel from './DetailPageTriggerPanel.svelte'

	export let triggerSelected:
		| 'webhooks'
		| 'emails'
		| 'schedules'
		| 'cli'
		| 'routes'
		| 'websockets'
		| 'postgres'
		| 'scheduledPoll'
		| 'kafka'
		| 'mqtt'
		| 'sqs'
		| 'gcp'
		| 'nats' = 'webhooks'
	export let flow_json: any | undefined = undefined
	export let simplfiedPoll: boolean = false

	export let isOperator: boolean = false

	export let selected: string
</script>

<HighlightTheme />

<div class="flex flex-col h-full">
	<Tabs bind:selected wrapperClass="flex-none w-full">
		<Tab value="saved_inputs">Inputs library</Tab>
		{#if !isOperator}
			<Tab value="triggers">Triggers</Tab>
		{/if}
		{#if flow_json}
			<Tab value="raw">Export</Tab>
		{:else}
			<Tab value="script">Script</Tab>
		{/if}
		{#if selected == 'flow_step'}
			<Tab value="flow_step">Step</Tab>
		{/if}

		<svelte:fragment slot="content">
			<div class="min-h-0 grow">
				<TabContent value="saved_inputs" class="h-full">
					<slot name="save_inputs" />
				</TabContent>
				<TabContent value="script" class="h-full">
					<slot name="script" />
				</TabContent>
				<TabContent value="triggers" class="h-full pt-2">
					<DetailPageTriggerPanel {simplfiedPoll} bind:triggerSelected>
						<slot slot="webhooks" name="webhooks" />
						<slot slot="routes" name="routes" />
						<slot slot="websockets" name="websockets" />
						<slot slot="postgres" name="postgres" />
						<slot slot="kafka" name="kafka" />
						<slot slot="nats" name="nats" />
						<slot slot="mqtt" name="mqtt" />
						<slot slot="sqs" name="sqs" />
						<slot slot="gcp" name="gcp" />
						<slot slot="emails" name="emails" />
						<slot slot="schedules" name="schedules" />
						<slot slot="cli" name="cli" />
					</DetailPageTriggerPanel>
				</TabContent>
				{#if flow_json}
					<TabContent value="raw" class="flex flex-col flex-1 h-full overflow-auto p-2">
						<FlowViewerInner flow={flow_json} />
					</TabContent>
					<TabContent value="flow_step" class="flex flex-col flex-1 h-full">
						<slot name="flow_step" />
					</TabContent>
				{/if}
			</div>
		</svelte:fragment>
	</Tabs>
</div>
