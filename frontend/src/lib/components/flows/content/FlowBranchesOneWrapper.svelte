<script lang="ts">
	import { Alert, Badge } from '$lib/components/common'

	import type { BranchOne, FlowModule } from '$lib/gen'
	import FlowCard from '../common/FlowCard.svelte'
	import BranchPredicateEditor from './BranchPredicateEditor.svelte'
	import FlowModuleAdvancedSettings from './FlowModuleAdvancedSettings.svelte'
	import { useUiIntent } from '$lib/components/copilot/chat/flow/useUiIntent'

	interface Props {
		flowModule: FlowModule
		previousModule: FlowModule | undefined
		parentModule: FlowModule | undefined
		noEditor: boolean
		enableAi?: boolean
	}

	let {
		flowModule = $bindable(),
		previousModule,
		parentModule,
		noEditor,
		enableAi = false
	}: Props = $props()

	let value = $state(flowModule.value as BranchOne)
	$effect(() => {
		value = flowModule.value as BranchOne
	})

	let advancedSettings: FlowModuleAdvancedSettings | undefined = $state(undefined)

	// UI Intent handling for AI tool control: forward the requested tab to the
	// matching Run-settings accordion row (keys match the old tab names).
	useUiIntent(`branchone-${flowModule.id}`, {
		openTab: (tab) => {
			advancedSettings?.openSetting(tab)
		}
	})
</script>

<div class="h-full" id="flow-editor-branch-one-wrapper">
	<FlowCard {noEditor} title="Run one branch">
		<div class="flex h-full min-h-0 flex-col gap-6 overflow-auto p-4">
			{#if !noEditor}
				<Alert
					type="info"
					title="Only first branch whose condition is true will be run"
					tooltip="Branch one"
					documentationLink="https://www.windmill.dev/docs/flows/flow_branches#branch-one"
				>
					The result of this step is the result of the branch.
				</Alert>
			{/if}

			<section>
				<h3 class="mb-4">
					{value.branches.length + 1} branch{value.branches.length + 1 > 1 ? 'es' : ''}
				</h3>
				<div class="py-2">
					<div class="flex flex-row gap-2 text-sm p-2">
						<Badge large={true} color="blue">Default branch</Badge>
						<p class="italic text-primary"
							>If none of the predicates' expressions evaluated in-order match, this branch is
							chosen</p
						>
					</div>
					{#each value.branches as branch, i}
						<div class="flex flex-col gap-x-2 items-center">
							<div class="w-full flex gap-2 px-2 pt-4 pb-2">
								<Badge large={true} color="blue">Branch {i + 1}</Badge>
								<input
									class="w-full"
									type="text"
									bind:value={branch.summary}
									placeholder="Summary"
								/>
							</div>
							<div class="w-full border">
								<BranchPredicateEditor
									{branch}
									on:updateSummary={(e) => {
										if (!branch.summary) {
											branch.summary = e.detail
										}
									}}
									parentModule={flowModule}
									{previousModule}
									{enableAi}
								/>
							</div>
						</div>
					{/each}
				</div>
				<p class="text-sm">Add branches and steps directly on the graph.</p>
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
