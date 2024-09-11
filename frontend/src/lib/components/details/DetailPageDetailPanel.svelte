<script lang="ts">
	import { Tabs, Tab, TabContent } from '$lib/components/common'
	import { CalendarCheck2, MailIcon, Route, Terminal, Webhook } from 'lucide-svelte'

	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import HighlightTheme from '../HighlightTheme.svelte'
	import FlowViewerInner from '../FlowViewerInner.svelte'

	export let triggerSelected: 'webhooks' | 'email' | 'schedule' | 'cli' | 'triggers' = 'webhooks'
	export let flow_json: any | undefined = undefined
	export let hasStepDetails: boolean = false

	export let isOperator: boolean = false

	export let selected: string

	$: if (hasStepDetails) {
		selected = 'flow_step'
	}

	// When we no longer have a selected flow step, switch to saved inputs
	$: !hasStepDetails && selected === 'flow_step' && (selected = 'saved_inputs')
</script>

<HighlightTheme />

<Splitpanes horizontal class="h-full">
	<Pane size={100}>
		<Tabs bind:selected>
			<Tab value="saved_inputs">Saved inputs</Tab>
			{#if !isOperator}
				<Tab value="details">Details & Triggers</Tab>
			{/if}
			{#if flow_json}
				<Tab value="raw">Raw</Tab>
			{/if}
			{#if hasStepDetails}
				<Tab value="flow_step">Step</Tab>
			{/if}
			<svelte:fragment slot="content">
				<div class="overflow-hidden" style="height:calc(100% - 32px);">
					<TabContent value="saved_inputs" class="flex flex-col flex-1 h-full">
						<slot name="save_inputs" />
					</TabContent>
					<TabContent value="details" class="flex flex-col flex-1 h-full">
						<Splitpanes horizontal class="h-full">
							<Pane size={50} minSize={20}>
								<slot name="details" />
							</Pane>
							<Pane size={50} minSize={20}>
								<Tabs bind:selected={triggerSelected}>
									<Tab value="webhooks">
										<span class="flex flex-row gap-2 items-center">
											<Webhook size={14} />
											Webhooks
										</span>
									</Tab>
									<Tab value="schedule">
										<span class="flex flex-row gap-2 items-center">
											<CalendarCheck2 size={14} />
											Schedules
										</span>
									</Tab>
									<Tab value="triggers">
										<span class="flex flex-row gap-2 items-center">
											<Route size={14} />
											Triggers
										</span>
									</Tab>
									<Tab value="email">
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
										{:else if triggerSelected === 'triggers'}
											<slot name="triggers" />
										{:else if triggerSelected === 'email'}
											<slot name="email" />
										{:else if triggerSelected === 'schedule'}
											<slot name="schedule" />
										{:else if triggerSelected === 'cli'}
											<slot name="cli" />
										{/if}
									</div>
								</div>
							</Pane>
						</Splitpanes>
					</TabContent>
					<TabContent value="raw" class="flex flex-col flex-1 h-full overflow-auto">
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
