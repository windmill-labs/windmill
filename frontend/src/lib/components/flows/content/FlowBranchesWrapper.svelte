<script lang="ts">
	import { Alert, Badge, Button, Tab } from '$lib/components/common'
	import TabContent from '$lib/components/common/tabs/TabContent.svelte'
	import Tabs from '$lib/components/common/tabs/Tabs.svelte'
	import HighlightCode from '$lib/components/HighlightCode.svelte'
	import Toggle from '$lib/components/Toggle.svelte'

	import type { FlowModule } from '$lib/gen'
	import { language } from '$lib/sql'
	import { getContext } from 'svelte'
	import FlowCard from '../common/FlowCard.svelte'
	import type { FlowEditorContext } from '../types'
	import FlowModuleEarlyStop from './FlowModuleEarlyStop.svelte'
	import FlowModuleSuspend from './FlowModuleSuspend.svelte'
	// import FlowRetries from './FlowRetries.svelte'

	const { selectedId } = getContext<FlowEditorContext>('FlowEditorContext')

	export let flowModule: FlowModule
	export let parentModule: FlowModule | undefined
	export let previousModule: FlowModule | undefined

	let selected: string = 'early-stop'
	type Branch = { id: string; summary?: string; expr?: string }
	let branches: Branch[] = []
	$: if (flowModule.value.type === 'branchone') {
		branches = flowModule.value.branches.map((branch, i) => ({
			id: `${flowModule.id}-branch-${i}`,
			summary: branch.summary,
			expr: branch.expr
		}))
	} else if (flowModule.value.type === 'branchall') {
		branches = flowModule.value.branches.map((branch, i) => ({
			id: `${flowModule.id}-branch-${i}`,
			summary: branch.summary,
			expr: undefined
		}))
	}
</script>

<div class="h-full flex flex-col">
	<FlowCard title={flowModule.value.type == 'branchall' ? 'Run all branches' : 'Run one branch'}>
		<div class="flex flex-col h-full">
			<div class="border">
				{#if flowModule.value.type == 'branchall'}
					<Alert type="info" title="All branches will be run">
						Configure branches to skip errors and to not result in that step itself failing.
						<br />
						<br />
						The result of this step is the list of the result of each branch.
					</Alert>
				{:else}
					<Alert type="info" title="Only one branch will be run">
						Only one branch is ran, the first one that match its predicate, if none do, the default
						branch is chosen.
						<br />
						<br />
						The result of this step is the result of the branch.
					</Alert>
				{/if}

				<div class="p-2">
					<h3
						>{branches.length + (flowModule.value.type === 'branchone' ? 1 : 0)} branch{branches.length +
							(flowModule.value.type === 'branchone' ? 1 : 0) >
						1
							? 'es'
							: ''}</h3
					>
					<div class="flex flex-col gap-y-4 py-2">
						{#if flowModule.value.type === 'branchone'}
							<div class="flex flex-row">
								<Badge large={true} color="blue">Default branch</Badge>
							</div>
						{/if}
						{#each branches as branch, i}
							{#if flowModule.value.type === 'branchone'}
								<div class="flex flex-row gap-x-2 items-center">
									<div class="max-w-sm w-full text-sm text-gray-600">
										{branch.summary && branch.summary != '' ? branch.summary : 'No summary'}</div
									>
									<div>
										<Button color="dark" size="sm" on:click={() => ($selectedId = branch.id)}>
											Set branch {i} predicate</Button
										>
									</div>
									<div class="w-full text-2xs">
										<HighlightCode language="deno" code={branch.expr} />
									</div>
								</div>
							{:else}
								<div class="flex flex-row gap-x-2">
									<div class="max-w-sm w-full text-sm text-gray-600">
										{branch.summary && branch.summary != '' ? branch.summary : 'No summary'}
									</div>
									<div>
										<Button color="dark" size="sm" on:click={() => ($selectedId = branch.id)}>
											Set branch {i} summary</Button
										>
									</div>
								</div>
							{/if}
						{/each}
					</div>
					{#if flowModule.value.type == 'branchall'}
						<div class="mt-6 mb-2 text-sm font-bold">Run in parallel</div>
						<Toggle
							bind:checked={flowModule.value.parallel}
							options={{
								right: 'All branches run in parallel'
							}}
						/>
					{/if}
				</div>
			</div>
			{#if flowModule}
				<Tabs bind:selected>
					<!-- <Tab value="retries">Retries</Tab> -->
					<Tab value="early-stop">Early Stop</Tab>
					<Tab value="suspend">Sleep/Suspend</Tab>

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
									<FlowModuleSuspend previousModuleId={previousModule?.id} bind:flowModule />
								</div>
							</TabContent>
						</div>
					</svelte:fragment>
				</Tabs>
			{/if}
		</div>
	</FlowCard>
</div>
