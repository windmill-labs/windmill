<script lang="ts">
	import { Tab } from '$lib/components/common'
	import TabContent from '$lib/components/common/tabs/TabContent.svelte'
	import Tabs from '$lib/components/common/tabs/Tabs.svelte'
	import type { FlowModule } from '$lib/gen'
	import FlowModuleEarlyStop from './FlowModuleEarlyStop.svelte'
	import FlowModuleSuspend from './FlowModuleSuspend.svelte'
	import FlowRetries from './FlowRetries.svelte'

	export let flowModule: FlowModule
	export let parentModule: FlowModule | undefined
	export let previousModuleId: string | undefined

	let selected: string = 'retries'
</script>

{#if flowModule}
	<Tabs bind:selected>
		<Tab value="retries">Retries</Tab>
		<Tab value="early-stop">Early Stop</Tab>
		<Tab value="suspend">Sleep/Suspend</Tab>

		<svelte:fragment slot="content">
			<div class="overflow-hidden bg-white" style="height:calc(100% - 32px);">
				<TabContent value="retries" class="flex flex-col flex-1 h-full">
					<div class="p-4 overflow-y-auto">
						<FlowRetries bind:flowModule />
					</div>
				</TabContent>

				<TabContent value="early-stop" class="flex flex-col flex-1 h-full">
					<div class="p-4 overflow-y-auto">
						<FlowModuleEarlyStop bind:flowModule {parentModule} {previousModuleId} />
					</div>
				</TabContent>

				<TabContent value="suspend" class="flex flex-col flex-1 h-full">
					<div class="p-4 overflow-y-auto">
						<FlowModuleSuspend bind:flowModule />
					</div>
				</TabContent>
			</div>
		</svelte:fragment>
	</Tabs>
{/if}
