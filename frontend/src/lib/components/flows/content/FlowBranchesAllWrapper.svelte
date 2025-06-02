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
	import FlowModuleMock from './FlowModuleMock.svelte'
	import FlowModuleDeleteAfterUse from './FlowModuleDeleteAfterUse.svelte'
	import { enterpriseLicense } from '$lib/stores'
	import FlowModuleSkip from './FlowModuleSkip.svelte'

	export let noEditor: boolean
	export let flowModule: FlowModule
	export let previousModule: FlowModule | undefined
	export let parentModule: FlowModule | undefined

	let value = flowModule.value as BranchAll
	$: value = flowModule.value as BranchAll

	let selected = 'early-stop'
</script>

<div class="h-full flex flex-col w-full" id="flow-editor-branch-all-wrapper">
	<FlowCard {noEditor} title={value.type == 'branchall' ? 'Run all branches' : 'Run one branch'}>
		<SplitPanesWrapper>
			<Splitpanes horizontal>
				<Pane size={flowModule ? 60 : 100}>
					{#if !noEditor}
						<Alert
							type="info"
							title="All branches will be run"
							tooltip="Branch all"
							documentationLink="https://www.windmill.dev/docs/flows/flow_branches#branch-all"
							class="m-4"
						>
							The result of this step is the list of the result of each branch.
						</Alert>
					{/if}
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
						<p class="text-sm">Add branches and steps directly on the graph.</p>
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
						<Tabs bind:selected id={`flow-editor-branch-all-${flowModule.id}`}>
							<Tab value="early-stop">Early Stop/Break</Tab>
							<Tab value="skip">Skip</Tab>
							<Tab value="suspend">Suspend/Approval/Prompt</Tab>
							<Tab value="sleep">Sleep</Tab>
							<Tab value="mock">Mock</Tab>
							<Tab value="lifetime">Lifetime</Tab>
							<svelte:fragment slot="content">
								<div class="overflow-hidden bg-surface">
									<TabContent value="early-stop" class="flex flex-col flex-1 h-full">
										<div class="p-4 overflow-y-auto">
											<FlowModuleEarlyStop bind:flowModule />
										</div>
									</TabContent>
									<TabContent value="skip" class="flex flex-col flex-1 h-full">
										<div class="p-4 overflow-y-auto">
											<FlowModuleSkip bind:flowModule {parentModule} {previousModule} />
										</div>
									</TabContent>
									<TabContent value="suspend" class="flex flex-col flex-1 h-full">
										<div class="p-4 overflow-y-auto">
											<FlowModuleSuspend previousModuleId={previousModule?.id} bind:flowModule />
										</div>
									</TabContent>
									<TabContent value="sleep" class="flex flex-col flex-1 h-full">
										<div class="p-4 overflow-y-auto">
											<FlowModuleSleep previousModuleId={previousModule?.id} bind:flowModule />
										</div>
									</TabContent>
									<TabContent value="mock" class="flex flex-col flex-1 h-full">
										<div class="p-4 overflow-y-auto">
											<FlowModuleMock bind:flowModule />
										</div>
									</TabContent>
									<TabContent value="lifetime" class="flex flex-col flex-1 h-full">
										<div class="p-4 overflow-y-auto">
											<FlowModuleDeleteAfterUse bind:flowModule disabled={!$enterpriseLicense} />
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
