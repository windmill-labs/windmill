<script lang="ts">
	import { Alert, Badge, Tab } from '$lib/components/common'
	import TabContent from '$lib/components/common/tabs/TabContent.svelte'
	import Tabs from '$lib/components/common/tabs/Tabs.svelte'

	import type { BranchOne, FlowModule } from '$lib/gen'
	import FlowCard from '../common/FlowCard.svelte'
	import BranchPredicateEditor from './BranchPredicateEditor.svelte'
	import FlowModuleEarlyStop from './FlowModuleEarlyStop.svelte'
	import FlowModuleSleep from './FlowModuleSleep.svelte'
	import FlowModuleSuspend from './FlowModuleSuspend.svelte'
	// import FlowRetries from './FlowRetries.svelte'

	export let flowModule: FlowModule
	export let previousModule: FlowModule | undefined

	let value = flowModule.value as BranchOne
	$: value = flowModule.value as BranchOne

	let selected = 'early-stop'
</script>

<div class="h-full">
	<FlowCard title="Run one branch">
		<div class="flex flex-col h-full overflow-auto">
			<div class="border">
				<Alert notRounded type="info" title="Only one branch will be run">
					The result of this step is the result of the branch.
				</Alert>

				<div class="p-2">
					<h3 class="my-4"
						>{value.branches.length + 1} branch{value.branches.length + 1 > 1 ? 'es' : ''}</h3
					>
					<div class="flex flex-col gap-y-4 py-2">
						<div class="flex flex-row gap-2 text-sm border border-gray-400 p-2">
							<Badge large={true} color="blue">Default branch</Badge>
							<p class="italic text-gray-600"
								>If none of the predicates' expressions evaluated in-order match, this branch is
								chosen</p
							>
						</div>
						{#each value.branches as branch}
							<div class="flex flex-col gap-x-2 p-2 items-center border border-gray-400">
								<input
									class="w-full"
									type="text"
									bind:value={branch.summary}
									placeholder="Summary"
								/>
								<BranchPredicateEditor {branch} parentModule={flowModule} {previousModule} />
							</div>
						{/each}
					</div>
				</div>
			</div>
			{#if flowModule}
				<Tabs bind:selected>
					<!-- <Tab value="retries">Retries</Tab> -->
					<Tab value="early-stop">Early Stop/Break</Tab>
					<Tab value="suspend">Suspend</Tab>
					<Tab value="sleep">Sleep</Tab>

					<svelte:fragment slot="content">
						<div class="overflow-hidden bg-white">
							<!-- <TabContent value="retries" class="flex flex-col flex-1 h-full">
								<div class="p-4 overflow-y-auto">
									<FlowRetries bind:flowModule />
								</div>
							</TabContent> -->

							<TabContent value="early-stop" class="flex flex-col flex-1 h-full">
								<div class="p-4 overflow-y-auto">
									<FlowModuleEarlyStop bind:flowModule />
								</div>
							</TabContent>

							<TabContent value="suspend" class="flex flex-col flex-1 h-full">
								<div class="p-4 overflow-y-auto">
									<FlowModuleSuspend bind:flowModule />
								</div>
							</TabContent>
							<TabContent value="sleep" class="flex flex-col flex-1 h-full">
								<div class="p-4 overflow-y-auto">
									<FlowModuleSleep previousModuleId={previousModule?.id} bind:flowModule />
								</div>
							</TabContent>
						</div>
					</svelte:fragment>
				</Tabs>
			{/if}
		</div>
	</FlowCard>
</div>
