<script lang="ts">
	import { Alert, Badge } from '$lib/components/common'
	import Toggle from '$lib/components/Toggle.svelte'

	import type { BranchAll, FlowModule } from '$lib/gen'
	import FlowCard from '../common/FlowCard.svelte'
	import FlowModuleAdvancedSettings from './FlowModuleAdvancedSettings.svelte'
	import { useUiIntent } from '$lib/components/copilot/chat/flow/useUiIntent'

	interface Props {
		noEditor: boolean
		flowModule: FlowModule
		previousModule: FlowModule | undefined
		parentModule: FlowModule | undefined
	}

	let { noEditor, flowModule = $bindable(), previousModule, parentModule }: Props = $props()

	let value = $state(flowModule.value as BranchAll)
	$effect(() => {
		value = flowModule.value as BranchAll
	})

	let advancedSettings: FlowModuleAdvancedSettings | undefined = $state(undefined)

	// UI Intent handling for AI tool control: forward the requested tab to the
	// matching Run-settings accordion row (keys match the old tab names).
	useUiIntent(`branchall-${flowModule.id}`, {
		openTab: (tab) => {
			advancedSettings?.openSetting(tab)
		}
	})
</script>

<div class="h-full flex flex-col w-full" id="flow-editor-branch-all-wrapper">
	<FlowCard {noEditor} title={value.type == 'branchall' ? 'Run all branches' : 'Run one branch'}>
		<div class="flex h-full min-h-0 flex-col gap-6 overflow-auto p-4">
			{#if !noEditor}
				<Alert
					type="info"
					title="All branches will be run"
					tooltip="Branch all"
					documentationLink="https://www.windmill.dev/docs/flows/flow_branches#branch-all"
				>
					The result of this step is the list of the result of each branch.
				</Alert>
			{/if}

			<section class="w-full">
				<h3 class="mb-4">{value.branches.length} branch{value.branches.length > 1 ? 'es' : ''}</h3>
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
			</section>

			<section>
				<FlowModuleAdvancedSettings
					embedded
					loopSubset
					bind:this={advancedSettings}
					bind:flowModule
					{parentModule}
					{previousModule}
					selectedId={flowModule.id}
				/>
			</section>
		</div>
	</FlowCard>
</div>
