<script lang="ts">
	import { Tabs, Tab, TabContent } from '$lib/components/common'

	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import HighlightTheme from '../HighlightTheme.svelte'
	import FlowViewerInner from '../FlowViewerInner.svelte'
	import DetailPageTriggerPanel from './DetailPageTriggerPanel.svelte'

	export let triggerSelected:
		| 'webhooks'
		| 'emails'
		| 'schedules'
		| 'cli'
		| 'routes'
		| 'websockets' = 'webhooks'
	export let flow_json: any | undefined = undefined

	export let isOperator: boolean = false

	export let selected: string
</script>

<HighlightTheme />

<Splitpanes horizontal class="h-full">
	<Pane size={100}>
		<Tabs bind:selected>
			<Tab value="saved_inputs">Saved Inputs</Tab>
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
				<div class="overflow-hidden" style="height:calc(100% - 32px);">
					<TabContent value="saved_inputs" class="flex flex-col flex-1 h-full">
						<slot name="save_inputs" />
					</TabContent>
					<TabContent value="schema" class="flex flex-col flex-1 h-full">
						<slot name="schema" />
					</TabContent>
					<TabContent value="script" class="flex flex-col flex-1 h-full">
						<slot name="script" />
					</TabContent>
					<TabContent value="triggers" class="flex flex-col flex-1 h-full pt-2">
						<DetailPageTriggerPanel bind:triggerSelected>
							<slot slot="webhooks" name="webhooks" />
							<slot slot="routes" name="routes" />
							<slot slot="websockets" name="websockets" />
							<slot slot="emails" name="emails" />
							<slot slot="schedules" name="schedules" />
							<slot slot="cli" name="cli" />
						</DetailPageTriggerPanel>
					</TabContent>
					<TabContent value="raw" class="flex flex-col flex-1 h-full overflow-auto p-2">
						<FlowViewerInner flow={flow_json} />
					</TabContent>
					<TabContent value="flow_step" class="flex flex-col flex-1 h-full">
						<slot name="flow_step" />
					</TabContent>
				</div>
			</svelte:fragment>
		</Tabs>
	</Pane>
</Splitpanes>
