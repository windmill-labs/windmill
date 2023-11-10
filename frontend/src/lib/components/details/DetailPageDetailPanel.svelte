<script lang="ts">
	import { Tabs, Tab, TabContent, Button } from '$lib/components/common'
	import { copyToClipboard } from '$lib/utils'
	import { CalendarCheck2, Clipboard, Terminal, Webhook } from 'lucide-svelte'
	import { Highlight } from 'svelte-highlight'
	import json from 'svelte-highlight/languages/json'
	import { Pane, Splitpanes } from 'svelte-splitpanes'

	let triggerSelected: 'webhooks' | 'schedule' | 'cli' = 'webhooks'
	export let flow_json: any | undefined = undefined
	export let hasStepDetails: boolean = false

	export let isOperator: boolean = false

	let selected = 'saved_inputs'

	$: if (hasStepDetails) {
		selected = 'flow_step'
	}

	// When we no longer have a selected flow step, switch to saved inputs
	$: !hasStepDetails && selected === 'flow_step' && (selected = 'saved_inputs')
</script>

<Splitpanes horizontal class="h-full">
	<Pane size={100}>
		<Tabs {selected}>
			<Tab value="saved_inputs">Saved inputs</Tab>
			{#if !isOperator}
				<Tab value="details">Details & Triggers</Tab>
			{/if}
			{#if flow_json}
				<Tab value="flow_json">JSON</Tab>
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
					<TabContent value="flow_json" class="flex flex-col flex-1 h-full overflow-auto">
						<div class="relative pt-2">
							<Button
								on:click={() => copyToClipboard(JSON.stringify(flow_json, null, 4))}
								color="light"
								variant="border"
								size="xs"
								startIcon={{ icon: Clipboard }}
								btnClasses="absolute top-2 right-2 w-min"
							>
								Copy content
							</Button>
							<Highlight language={json} code={JSON.stringify(flow_json, null, 4)} />
						</div>
					</TabContent>
					<TabContent value="flow_step" class="flex flex-col flex-1 h-full">
						<slot name="flow_step" />
					</TabContent>
				</div>
			</svelte:fragment>
		</Tabs>
	</Pane>
</Splitpanes>
