<script lang="ts">
	import { Tabs, Tab, TabContent } from '$lib/components/common'
	import SplitPanesWrapper from '$lib/components/splitPanes/SplitPanesWrapper.svelte'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import DetailPageDetailPanel from './DetailPageDetailPanel.svelte'

	export let isOperator: boolean = false
	export let flow_json: any | undefined = undefined
	export let selected: string

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
						<DetailPageDetailPanel bind:selected {isOperator} {flow_json}>
							<slot slot="script" name="script" />
							<slot slot="save_inputs" name="save_inputs" />
							<slot slot="flow_step" name="flow_step" />
							<slot slot="triggers" name="triggers" />
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
						<TabContent value="triggers" class="flex flex-col flex-1 h-full mt-[-2px]">
							<slot name="triggers" />
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
