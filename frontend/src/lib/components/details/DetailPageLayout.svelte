<script lang="ts">
	import { Tabs, Tab, TabContent } from '$lib/components/common'
	import SplitPanesWrapper from '$lib/components/splitPanes/SplitPanesWrapper.svelte'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import DetailPageDetailPanel from './DetailPageDetailPanel.svelte'

	export let isOperator: boolean = false
	export let flow_json: any | undefined = undefined
	export let hasStepDetails: boolean = false

	let mobileTab: 'form' | 'detail' = 'form'

	let clientWidth = window.innerWidth
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
						<DetailPageDetailPanel {isOperator} {flow_json} {hasStepDetails}>
							<slot slot="webhooks" name="webhooks" />
							<slot slot="schedule" name="schedule" />
							<slot slot="cli" name="cli" />
							<slot slot="details" name="details" />
							<slot slot="save_inputs" name="save_inputs" />
							<slot slot="flow_step" name="flow_step" />
						</DetailPageDetailPanel>
					</Pane>
				</Splitpanes>
			</SplitPanesWrapper>
		</div>
	{:else}
		<div class="h-full">
			<slot name="header" />
			<Tabs bind:selected={mobileTab}>
				<Tab value="form">Run form</Tab>
				<Tab value="detail">Details</Tab>
				<svelte:fragment slot="content">
					<TabContent value="form" class="flex flex-col flex-1 h-full">
						<slot name="form" />
					</TabContent>
					<TabContent value="detail" class="flex flex-col flex-1 h-full">
						<DetailPageDetailPanel {isOperator} {hasStepDetails}>
							<slot slot="webhooks" name="webhooks" />
							<slot slot="schedule" name="schedule" />
							<slot slot="cli" name="cli" />
							<slot slot="details" name="details" />
							<slot slot="save_inputs" name="save_inputs" />
							<slot slot="flow_step" name="flow_step" />
						</DetailPageDetailPanel>
					</TabContent>
				</svelte:fragment>
			</Tabs>
		</div>
	{/if}
</main>
