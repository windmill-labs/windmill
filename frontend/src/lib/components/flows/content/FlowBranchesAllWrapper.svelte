<script lang="ts">
	import { Alert, Badge, Tab } from '$lib/components/common'
	import TabContent from '$lib/components/common/tabs/TabContent.svelte'
	import Tabs from '$lib/components/common/tabs/Tabs.svelte'
	import Toggle from '$lib/components/Toggle.svelte'

	import type { BranchAll, FlowModule } from '$lib/gen'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import SplitPanesWrapper from '../../splitPanes/SplitPanesWrapper.svelte'
	import FlowCard from '../common/FlowCard.svelte'
	import FlowModuleEarlyStop from './FlowModuleEarlyStop.svelte'
	import FlowModuleSleep from './FlowModuleSleep.svelte'
	import FlowModuleSuspend from './FlowModuleSuspend.svelte'

	export let flowModule: FlowModule
	export let previousModule: FlowModule | undefined

	let value = flowModule.value as BranchAll
	$: value = flowModule.value as BranchAll

	let selected = 'early-stop'
</script>

<div class="h-full flex flex-col w-full">
	<FlowCard title={value.type == 'branchall' ? 'Run all branches' : 'Run one branch'}>
		<SplitPanesWrapper>
			<Splitpanes horizontal>
				<Pane size={flowModule ? 60 : 100}>
					<Alert notRounded type="info" title="All branches will be run" class="m-2">
						The result of this step is the list of the result of each branch.
					</Alert>

					<div class="p-4 mt-4 w-full">
						<h3 class="mb-4"
							>{value.branches.length} branch{value.branches.length > 1 ? 'es' : ''}</h3
						>
						<div class="flex flex-col gap-y-4 py-2 w-full">
							{#each value.branches as branch, i}
								<div class="flex flex-row gap-x-4 w-full items-center">
									<div class="grow flex gap-2">
										<Badge large={true} color="blue">Branch {i + 1}</Badge>
										<input type="text" bind:value={branch.summary} placeholder="Summary" />
									</div>
									<div class="w-min-sm">
										<Toggle
											bind:checked={branch.skip_failure}
											options={{
												right: 'Skip failure'
											}}
										/>
									</div>
								</div>
							{/each}
						</div>
						<div class="mt-6 mb-2 text-sm font-bold">Run in parallel</div>
						<Toggle
							bind:checked={value.parallel}
							options={{
								right: 'All branches run in parallel'
							}}
						/>
					</div>
				</Pane>
				{#if flowModule}
					<Pane size={40}>
						<Tabs bind:selected>
							<Tab value="early-stop">Early Stop/Break</Tab>
							<Tab value="suspend">Suspend</Tab>
							<Tab value="sleep">Sleep</Tab>
							<svelte:fragment slot="content">
								<div class="overflow-hidden bg-white">
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
					</Pane>
				{/if}
			</Splitpanes>
		</SplitPanesWrapper>
	</FlowCard>
</div>
