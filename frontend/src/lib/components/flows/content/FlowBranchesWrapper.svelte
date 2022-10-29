<script lang="ts">
	import { Alert, Tab } from '$lib/components/common'
	import TabContent from '$lib/components/common/tabs/TabContent.svelte'
	import Tabs from '$lib/components/common/tabs/Tabs.svelte'
	import type { FlowModule } from '$lib/gen'
	import FlowCard from '../common/FlowCard.svelte'
	import FlowModuleEarlyStop from './FlowModuleEarlyStop.svelte'
	import FlowModuleSuspend from './FlowModuleSuspend.svelte'
	import FlowRetries from './FlowRetries.svelte'

	export let flowModule: FlowModule
	export let type: 'branchall' | 'branchone'
	export let parentModule: FlowModule | undefined
	export let previousModuleId: string | undefined

	let selected: string = 'retries'
</script>

<div class="h-full flex flex-col">
	<FlowCard title={type == 'branchall' ? 'Run all branches' : 'Run one branch'}>
		<div class="flex flex-col h-full">
			<div class="border">
				{#if type == 'branchall'}
					<Alert type="info" title="All branches will be run in order">
						Branches run in sequence, every branch is run to the end one after the other.
						<br />
						<br />
						Configure branches to skip errors and to not result in that step itself failing.
						<br />
						<br />
						The result of this step is the list of the result of each branch.
						<br />
						<br />
						Since this is a step containing all branches as embedded flows, this step can be retried,
						stopped early, can be made to sleep or to suspend after execution.
					</Alert>
				{:else}
					<Alert type="info" title="Only one branch will be run">
						Only one branch is ran, the first one that match its predicate, if none do, the default
						branch is chosen.
						<br />
						<br />
						The result of this step is the result of the branch.
						<br />
						<br />
						Since this is a step containing all branches as embedded flows, this step can be retried,
						stopped early, can be made to sleep or to suspend after execution.
					</Alert>
				{/if}
			</div>
			{#if flowModule}
				<Tabs bind:selected>
					<Tab value="retries">Retries</Tab>
					<Tab value="early-stop">Early Stop</Tab>
					<Tab value="suspend">Sleep/Suspend</Tab>

					<svelte:fragment slot="content">
						<div class="overflow-hidden bg-white">
							<TabContent value="retries" class="flex flex-col flex-1 h-full">
								<div class="p-4 overflow-y-auto">
									<FlowRetries bind:flowModule />
								</div>
							</TabContent>

							<TabContent value="early-stop" class="flex flex-col flex-1 h-full">
								<div class="p-4 overflow-y-auto">
									<FlowModuleEarlyStop {previousModuleId} bind:flowModule {parentModule} />
								</div>
							</TabContent>

							<TabContent value="suspend" class="flex flex-col flex-1 h-full">
								<div class="p-4 overflow-y-auto">
									<FlowModuleSuspend {previousModuleId} bind:flowModule />
								</div>
							</TabContent>
						</div>
					</svelte:fragment>
				</Tabs>
			{/if}
		</div>
	</FlowCard>
</div>
