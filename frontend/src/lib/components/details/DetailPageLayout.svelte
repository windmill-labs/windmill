<script lang="ts">
	import { Tabs, Tab, TabContent } from '$lib/components/common'
	import SplitPanesWrapper from '$lib/components/splitPanes/SplitPanesWrapper.svelte'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import DetailPageDetailPanel from './DetailPageDetailPanel.svelte'
	import type { ScheduleTrigger, TriggerContext } from '../triggers'
	import { setContext } from 'svelte'
	import { writable, type Writable } from 'svelte/store'
	import type { TriggersCount } from '$lib/gen'
	import DetailPageTriggerPanel from './DetailPageTriggerPanel.svelte'

	export let isOperator: boolean = false
	export let flow_json: any | undefined = undefined
	export let selected: string
	export let triggersCount: Writable<TriggersCount | undefined>

	let mobileTab: 'form' | 'detail' = 'form'

	let clientWidth = window.innerWidth

	const primaryScheduleStore = writable<ScheduleTrigger | undefined | false>(undefined)
	const selectedTriggerStore = writable<
		| 'webhooks'
		| 'emails'
		| 'schedules'
		| 'cli'
		| 'routes'
		| 'websockets'
		| 'postgres'
		| 'scheduledPoll'
		| 'kafka'
		| 'nats'
		| 'mqtt'
		| 'sqs'
		| 'gcp'
	>('webhooks')

	const simplifiedPoll = writable(false)
	setContext<TriggerContext>('TriggerContext', {
		selectedTrigger: selectedTriggerStore,
		primarySchedule: primaryScheduleStore,
		triggersCount,
		simplifiedPoll,
		defaultValues: writable(undefined),
		captureOn: writable(undefined),
		showCaptureHint: writable(undefined)
	})
</script>

<main class="h-screen w-full" bind:clientWidth>
	{#if clientWidth >= 768}
		<div class="h-full w-full">
			<slot name="header" />
			<SplitPanesWrapper>
				<Splitpanes>
					<Pane size={65} minSize={50}>
						<slot name="form" />
					</Pane>
					<Pane size={35} minSize={15}>
						<DetailPageDetailPanel
							simplfiedPoll={$simplifiedPoll}
							bind:triggerSelected={$selectedTriggerStore}
							bind:selected
							{isOperator}
							{flow_json}
						>
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
							<slot slot="script" name="script" />
							<slot slot="save_inputs" name="save_inputs" />
							<slot slot="flow_step" name="flow_step" />
						</DetailPageDetailPanel>
					</Pane>
				</Splitpanes>
			</SplitPanesWrapper>
		</div>
	{:else}
		<div class="h-full w-full">
			<slot name="header" />
			<Tabs bind:selected={mobileTab}>
				<Tab value="form">Run form</Tab>
				<Tab value="saved_inputs">Inputs</Tab>
				{#if !isOperator}
					<Tab value="triggers">Triggers</Tab>
				{/if}
				{#if flow_json}
					<Tab value="raw">Export</Tab>
				{:else}
					<Tab value="script">Script</Tab>
				{/if}

				<svelte:fragment slot="content">
					<div class="h-full">
						<TabContent value="form" class="flex flex-col flex-1 h-full">
							<slot name="form" />
						</TabContent>

						<TabContent value="saved_inputs" class="flex flex-col flex-1 h-full">
							<slot name="save_inputs" />
						</TabContent>
						<TabContent value="triggers" class="flex flex-col flex-1 h-full">
							<DetailPageTriggerPanel
								simplfiedPoll={$simplifiedPoll}
								bind:triggerSelected={$selectedTriggerStore}
							>
								<slot slot="webhooks" name="webhooks" />
								<slot slot="routes" name="routes" />
								<slot slot="script" name="script" />
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
						<TabContent value="script" class="flex flex-col flex-1 h-full">
							<slot name="script" />
						</TabContent>
					</div>
				</svelte:fragment>
			</Tabs>
		</div>
	{/if}
</main>
