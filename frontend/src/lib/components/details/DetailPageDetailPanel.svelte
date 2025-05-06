<script lang="ts">
	import { Tabs, Tab, TabContent } from '$lib/components/common'

	import HighlightTheme from '../HighlightTheme.svelte'
	import FlowViewerInner from '../FlowViewerInner.svelte'

	export let flow_json: any | undefined = undefined

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
				<TabContent value="triggers" class="h-full">
					<slot name="triggers" />
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
